# TUI Failure Mode Analysis: claw-code (feat-tui branch)

**Date:** 2026-06-12  
**Branch:** feat-tui  
**Scope:** Output bleeding and scroll rendering failures across 8 commits

---

## Part 1: Output Bleeding

Output bleeding occurs when bytes written to stdout/stderr during a model turn appear on the terminal in the middle of the TUI frame, corrupting the alternate-screen layout.

### Failure 1: Suspend/Resume Pattern (eb717bc1)

**Approach:** Before a `run_turn()`, call `app.suspend()` which leaves the alternate screen, disables raw mode, and clears the terminal. After the turn, `app.resume()` re-enters the alternate screen and redraws.

**Root cause:** The TUI runs in crossterm's alternate screen buffer. When `suspend()` calls `LeaveAlternateScreen`, the terminal switches back to the primary buffer where normal stdout goes. But `run_turn()` invokes `ConversationRuntime::run_turn()` which internally streams tool output via `consume_stream` → `TerminalRenderer` → `println!` / `io::stdout()`. This output lands in the primary buffer, which the user sees as a flicker of raw text before the TUI re-enters alternate screen.

**Why the fix didn't work:** The suspend/resume timing is inherently racy. Between `LeaveAlternateScreen` and the first byte of runtime output, there's a gap where the user sees an empty terminal. Between the last byte of runtime output and `EnterAlternateScreen`, there's another gap. Worse, if the turn panics or errors mid-stream, `resume()` may never execute, leaving the terminal in a broken state (raw mode disabled, no alternate screen). The suspends also destroy the alternate screen contents — every turn requires a full redraw from scratch.

**The correct fix:** Never leave the alternate screen. Instead, route the runtime's output away from the real stdout fd. The runtime needs an abstraction layer where all output goes through a handle the TUI controls: either a `Write` trait object (as `run_turn_to` later attempts), or an fd-level redirect (as `libc::dup` later attempts), or by making the runtime emit structured events (deltas, tool calls) on a channel instead of printing directly.

**How codex-rs avoids it:** codex-rs uses a `tokio`-async event loop where the runtime streams `AssistantEvent` / `TurnProgressReporter` events into an `mpsc` channel consumed by the TUI's `EventBroker`. The TUI renders these events as `ChatWidget` cells. The runtime never writes to stdout directly — it only emits structured events.

---

### Failure 2: In-Place Turn Execution (cc191fd6)

**Approach:** Remove `suspend()`/`resume()` entirely. Run the turn in-place while the TUI is still on the alternate screen. The comment says "output goes to the alternate screen buffer which ratatui owns — it will be overwritten on next redraw."

**Root cause:** The alternate screen buffer is a terminal feature, not a ratatui-owned buffer. When the runtime writes ANSI escape sequences and text to stdout while the TUI's alternate screen is active, those bytes are rendered immediately by the terminal emulator into the alternate screen. But ratatui's `Terminal::draw()` only updates the regions it knows about. The runtime's output appears as garbage characters overlaid on the TUI frame, and because the runtime writes at the cursor position (which moves as it prints), the text appears at random screen positions — "full-width bleeding" across both panes.

**Why the fix didn't work:** The assumption that "ratatui will overwrite on next redraw" is wrong for two reasons. First, the runtime's output is not constrained to any layout region — it writes to the entire screen at the cursor position. Second, ratatui's `draw()` uses a diff algorithm that only writes changed cells. If the runtime's output doesn't change any cell that ratatui considers "already correct," the garbage persists. There's no screen clear between the turn ending and the next draw.

**The correct fix:** Same as Failure 1 — the runtime must not write to the real stdout while the TUI is active. The TUI must own the output path. Either capture/redirect the output, or restructure the runtime to emit events.

**How codex-rs avoids it:** The runtime never prints; it sends events. The TUI's event loop renders them as structured cells, not raw text.

---

### Failure 3: Buffer Output via `run_turn_to` (58e095a0)

**Approach:** Add `run_turn_to<W: Write>(input, &mut out, emit_output)` that routes all explicit `println!` → `writeln!(out, ...)` calls through a custom writer. In TUI mode, `out` is a `Vec<u8>` buffer. Set `emit_output=false` to prevent the runtime from streaming tool output.

**Root cause:** The `run_turn_to` function only captures output from the `LiveCli` layer — the `writeln!(out, ...)` calls, spinners, and status lines in `run_turn_to` itself. But the deeper runtime (`ConversationRuntime::run_turn()`) has its own internal stdout writes that bypass the `out` parameter. Specifically: `consume_stream` writes streaming token output directly to `io::stdout()`, the tool executor prints formatted results to stdout, and child processes (bash, git) inherit the TUI's stdout fd and write directly. The `emit_output=false` flag was supposed to suppress this, but the flag only controls whether the runtime's `ToolStreamEvent` emissions reach the TUI — it doesn't prevent the underlying FD writes from child processes or the internal renderer.

**Why the fix didn't work:** There are too many code paths that write to stdout. The `out` parameter is a Rust-level abstraction, not an OS-level one. Any code that calls `io::stdout()` directly (including code in dependencies, tokio tasks, or child processes) bypasses it entirely. The buffer captures some output but not all — the uncaptured writes still bleed onto the alternate screen.

**The correct fix:** Redirect at the fd level, not the Rust `Write` trait level. Either use `gag::BufferRedirect` (which redirects fd 1 to a pipe at the OS level), or use `libc::dup`/`dup2` to swap fd 1 to `/dev/null` during the turn. These operate at the kernel level and catch all writes regardless of which code path they come from.

**How codex-rs avoids it:** It doesn't need fd tricks because the runtime never writes to stdout. Streams are consumed by the event loop and rendered as structured cells.

---

### Failure 4: Post-Turn Screen Clear (99c4650a)

**Approach:** Keep the `run_turn_to` buffer, but add a `crossterm::terminal::Clear(ClearType::All)` + `MoveTo(0,0)` after the turn to wipe any debris that leaked onto the alternate screen. Then force a full TUI redraw.

**Root cause:** The clear+redraw is a degenerate form of the suspend/resume pattern. It acknowledges that some output will leak, and tries to fix it after the fact. But the damage is already visible to the user during the turn — they see flickering garbage text between the start of the turn and the clear. Additionally, the clear itself causes a visible flash (entire screen goes blank then redraws), which is jarring.

**Why the fix didn't work:** The screen clear is applied after `run_turn_to()` returns, but during the turn the runtime's internal writes have already corrupted the screen. The user sees the corruption in real-time as the model is "thinking." The clear only fixes it after the fact — and even then, interleaved stdout writes during the clear sequence can leave partial artifacts if the timing is unlucky. It's a band-aid, not a cure.

**The correct fix:** Prevent writes from reaching the terminal during the turn, don't try to clean them up afterward.

**How codex-rs avoids it:** No stdout writes during streaming — events flow through a channel.

---

### Failure 5: libc::dup FD Redirect (8f1ad0dc)

**Approach:** Before the turn, `dup(1)` to save the real stdout fd, then `dup2(devnull_fd, 1)` to redirect fd 1 to `/dev/null`. Run the turn. Then `dup2(saved_fd, 1)` to restore. This catches ALL writes to fd 1 at the kernel level: runtime streaming, tool output, child processes, crossterm escape codes, everything.

**Root cause:** This actually works for stdout suppression, but introduces new problems: (1) it requires `unsafe` code, which the workspace had set to `forbid`; (2) it blocks ALL stdout, including legitimate crossterm terminal control sequences that ratatui needs during the turn (e.g., resize handling); (3) stderr (fd 2) is not redirected, so error output from tools still bleeds; (4) the `run_turn_to` buffer is still in the code but now useless since fd 1 goes to `/dev/null` — the buffer captures nothing, so the conversation pane never gets the turn's output; (5) restoring the fd after a panic (if the turn panics mid-execution) requires a panic hook, otherwise fd 1 is permanently stuck on `/dev/null`.

**Why the fix didn't work:** The approach is architecturally wrong — it treats the symptom (stdout bytes reaching the terminal) by silencing all stdout, rather than routing the output where the TUI can use it. The TUI needs to *receive* the output (to render it in the conversation pane), not just *suppress* it. With fd 1 pointing to `/dev/null`, the TUI loses access to all turn output. The conversation pane is populated only by reading the last assistant message from the session afterward, which loses tool output, compaction notices, and streaming tokens.

**The correct fix:** Use `gag::BufferRedirect` (OS-level pipe redirect that captures rather than discards) or restructure the runtime to emit events. `gag` redirects fd 1 to a pipe — writes from any code path end up in the pipe buffer, which the TUI reads after the turn. This is the same OS-level interception as `dup`/`dup2`, but captures instead of discarding. This is what commit 4af649b4 eventually does.

**How codex-rs avoids it:** Event-driven architecture — the runtime streams events, the TUI renders them. No fd manipulation needed.

---

### Failure 6: Leave Alternate Screen During Turns (25610f2a)

**Approach:** `leave_for_turn()` exits the alternate screen before running the turn. The turn's output goes to the normal terminal. After the turn, `wait_to_return()` prompts "Press any key to return to TUI..." then `reenter_after_turn()` re-enters the alternate screen and redraws.

**Root cause:** This is the suspend/resume pattern reborn. The key improvement is that it deliberately shows the user the turn's output in the normal terminal (rather than letting it corrupt the TUI). But the UX is terrible — the user sees the TUI disappear, raw output appear, then has to press a key to get the TUI back. This is not a TUI experience; it's a REPL with extra steps.

**Why the fix didn't work:** The fundamental problem is that the TUI and the runtime both want to own stdout. Leaving the alternate screen "works" (zero bleeding) but destroys the immersive TUI experience. The user context-switches between two different views on every turn. The "press any key" prompt adds friction. And the conversation pane only gets populated from the session afterward (reading the last assistant message), so tool output shown in the normal terminal is not rendered inside the TUI.

**The correct fix:** The TUI must stay on the alternate screen. The runtime's output must be captured at the OS level (via `gag::BufferRedirect` or similar) and fed into the TUI's conversation pane. The `leave_for_turn` approach works as a last resort but defeats the purpose of having a TUI.

**How codex-rs avoids it:** Async event loop — the runtime emits events, the TUI renders them inline. The user never leaves the TUI.

---

## Part 2: Scroll Rendering

Scroll is broken across all iterations because the code uses an inverted offset model that conflicts with ratatui's `Paragraph::scroll()` semantics.

### Scroll Failure 1: Inverted Offset with `Paragraph::scroll()` (eb717bc1)

**Root cause:** `auto_scroll()` sets `conversation_scroll = line_count - 20`, treating the value as "how far from the bottom." But `Paragraph::scroll((y, 0))` interprets `y` as "how far from the top." So a large scroll value shows the *top* of the conversation, not the bottom. The code then inverts the key bindings: PageUp *decreases* scroll (moves viewport down in `Paragraph::scroll` terms = moves toward bottom in user terms), PageDown *increases* scroll (moves toward top). This is backward from every terminal convention.

**Why the fix didn't work:** The inverted model is confusing but internally consistent — if you squint, PageUp = "scroll toward newer" = decrease the bottom-offset. But the magic number `20` (hardcoded pane height) breaks when the terminal is resized. And `Paragraph::scroll()` combined with `.wrap(Wrap{trim:true})` produces visual glitches: ratatui re-wraps text on each render, changing the line count, which makes the scroll offset jump wildly.

**The correct fix:** Compute scroll in terms of "lines from the top" (matching `Paragraph::scroll()` semantics). `auto_scroll` should set scroll to `max(0, total_wrapped_lines - visible_rows)`. PageUp adds to scroll (move viewport down = see older content), PageDown subtracts (see newer content). Recompute wrapped line count on every render to account for width changes.

**How codex-rs avoids it:** codex-rs uses `ChatWidget` which maintains a `ScrollState` on a list of `MessageCell` objects. Each cell is a self-contained render unit. Scrolling operates on the cell list, not on flat wrapped lines, so re-wrapping doesn't cause offset jumps.

---

### Scroll Failure 2: Custom Wrapped-Line Rendering (8f1ad0dc)

**Root cause:** This commit introduces `build_wrapped_conversation()` which pre-computes wrapped lines and `expand_counts` (how many visual rows each logical line expands to). The scroll math computes `start = total_visual - (pane_rows + offset)` and `visible = wrapped.skip(start).take(pane_rows)`. The `conversation_scroll` value is still inverted: 0 = bottom, increasing = scrolling up. The `auto_scroll` sets it to 0, and `scroll_up` *adds* to it, `scroll_down` *subtracts*. The rendering inverts this back to show the correct window.

**Why the fix didn't work:** The double-inversion is fragile and error-prone. The `expand_counts` vector tracks how many visual rows each logical line produces, but it's only used for the trim-notice insertion logic, not for scroll calculation. The scroll math recomputes `start` from `total_visual` every frame, which is correct, but the conversion from `conversation_scroll` (inverted offset) to `start` (forward offset) is: `start = total - pane_rows - scroll`. If `scroll` overflows (e.g., `u16::MAX` for "scroll to top"), then `pane_rows + scroll` overflows and `start` wraps to a garbage value, showing the wrong lines.

**The correct fix:** The 6e787c2b version handles this correctly: `let offset = scroll.min(max_offset)` clamps the offset before computing `start`. But it still uses the inverted model. The correct approach is to use a forward offset (0 = top of conversation, max = bottom), which matches both `Paragraph::scroll()` semantics and user intuition. With the `RenderCache` pattern, the wrapped line count is precomputed and cached, making the forward-offset math trivial.

**How codex-rs avoids it:** codex-rs maintains a `ScrollState` on its `ChatWidget` that tracks an absolute offset in the cell list. Scrolling adjusts this offset directly, clamped to `[0, total_cells - visible_cells]`. No inversion needed.

---

### Scroll Failure 3: RenderCache with RefCell (6e787c2b — current)

**Root cause:** The current `ConversationPane` introduces a `RenderCache` with `RefCell<RenderCache>` to allow cache rebuilding inside `render(&self)`. The cache stores `wrapped_lines: Vec<Line<'static>>` and `built_width: u16`. On render, if `dirty || built_width != area.width`, the cache is rebuilt. This correctly avoids recomputation when nothing changed.

**Why the fix doesn't fully work:** The `auto_scroll()` still sets `scroll = 0` and `dirty = true`, meaning "show the bottom." But it then calls `rebuild_cache()` inside `render(&self)` which borrows `self.cache` via `RefCell`, and can't clear `self.dirty` because that would require `&mut self` while the cache is borrowed. The workaround is `mark_clean()` called after render, but this creates a subtle ordering dependency: if anything reads `is_dirty()` between `rebuild_cache` and `mark_clean`, it returns true. More critically, the scroll model is still inverted (0 = bottom, increasing = scroll up), and `scroll_up` adds while `scroll_down` subtracts, which still contradicts `Paragraph::scroll()` semantics even though the rendering manually inverts it via `start = total - visible - offset`.

**The correct fix:** Flip to a forward-offset scroll model. `scroll` should mean "number of visual lines from the top." `auto_scroll` sets `scroll = max(0, total_wrapped - pane_rows)`. `scroll_up` subtracts, `scroll_down` adds. This matches `Paragraph::scroll()`, matches user intuition, and eliminates the double-inversion. The `RenderCache` pattern is sound and should be kept — it just needs the scroll semantics fixed.

**How codex-rs avoids it:** codex-rs uses a forward-offset `ScrollState` on a list of message cells, not wrapped lines. Cells are the unit of scroll, which is coarser but more stable. Individual cells handle their own wrapping internally.

---

## Part 3: The Architectural Root Cause

All failures share a common root: **the runtime was built to print to stdout, and the TUI was grafted on top.** Every attempt to fix output bleeding is a workaround for the fact that `ConversationRuntime::run_turn()` writes to `io::stdout()` through multiple internal paths.

The correct architectural fix — as documented in `docs/TUI.md` and practiced by codex-rs — is:

1. **Make the runtime emit events, not print.** The TUI provides a `TurnProgressReporter` (or similar trait) that receives structured events: `AssistantDelta(text)`, `ToolCallStart(tool)`, `ToolCallResult(output)`, `TurnComplete(summary)`, `TurnError(error)`.
2. **The TUI event loop consumes these events** via an `mpsc` channel and renders them as `MessageCell` objects in a `ChatWidget`.
3. **No stdout writes during turns.** The runtime's internal renderer is either disabled (TUI mode) or replaced with a no-op reporter.
4. **Scroll operates on the cell list**, not on flat wrapped lines. Each cell manages its own wrapping. Scroll offset is forward (0 = top) and clamped to the cell count.

This is the codex-rs architecture, and it's what `docs/TUI.md` prescribes for Phase 2 (Live Conversation). The current implementation is stuck in Phase 1 (Skeleton) — the TUI is a view-only shell that runs `run_turn()` and scrapes the result, rather than an active participant in the turn's event stream.

---

## Summary Table

| Commit | Approach | Type | Root Cause | Worked? |
|--------|----------|------|------------|---------|
| eb717bc1 | Suspend/resume (LeaveAltScreen) | Output | Runtime prints to stdout between Leave/EnterAltScreen | No — flicker, racy, loses TUI state on panic |
| cc191fd6 | In-place turn (no suspend) | Output | Runtime output renders on alternate screen; ratatui can't overwrite it | No — full-width bleeding |
| 58e095a0 | `run_turn_to<Vec<u8>>` buffer | Output | Only captures `LiveCli` layer output; runtime internals bypass the buffer | No — partial capture, child processes leak |
| 99c4650a | Post-turn screen clear | Output | Corruption visible during turn; clear causes flash | No — visible flicker; cosmetic only |
| 8f1ad0dc | `libc::dup` fd redirect to /dev/null | Output | Suppresses stdout but discards it; TUI can't read turn output | Partial — no bleeding, but no conversation content |
| 4af649b4 | `gag::BufferRedirect` | Output | Captures all fd 1 writes including child processes | Partial — works for capture, but loses ratatui control during turn |
| 25610f2a | Leave alternate screen during turns | Output | Works (zero bleeding) but destroys TUX UX | Functional but bad UX — REPL with extra steps |
| eb717bc1 | Inverted scroll + Paragraph::scroll() | Scroll | Inverted offset model; hardcoded pane height | No — wrong direction, resize breaks |
| 8f1ad0dc | Inverted scroll + custom wrap/render | Scroll | Double-inversion; u16 overflow on scroll-top | Partial — mostly works if scroll stays small |
| 6e787c2b | RenderCache + RefCell + inverted scroll | Scroll | Still inverted; dirty/clean ordering; RefCell borrow conflict | Partial — cache works, scroll semantics still inverted |
