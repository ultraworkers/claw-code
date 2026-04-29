# claw-web — design plan

This is the place to record decisions and tradeoffs. The README is the
how-to-run; this is the why.

## Origin

Distilled from two queued brain memories on `claw-code`:

- `queued-claw-ui-wrapper-project` — the user wants a UI that owns claw
  subprocesses, switches projects token-free, and works from light clients.
- `queued-research-pwa-frontend-on-cloudflare` — same motivation, framed
  as the remote-access half: PWA on Cloudflare edge + Tunnel back to the
  local backend, so an iPad on any network reaches the agent.

Plus pattern observation from `~/src/ai-local-hardware-deploy` (Sentinel,
the friend's project) — see "Borrowed and rejected" below.

## Phase plan

### Phase 1 — local single-user, mock-shaped (this scaffold)

- ✅ Project picker (list / create / select, last-opened ordering).
- ✅ Session picker per project (list / create / select).
- ✅ WebSocket per active session, status + final events.
- ✅ Mock runner so the UI walks without claw plumbed.
- ✅ Subprocess runner — streams stdout, mediates approval prompts
  (see Phase 2 below), parses claw's `--output-format json` blob at EOF.
- ✅ SQLite registry; claw owns conversation history.
- ✅ PWA framing (manifest + sw stub) so phase 2 has somewhere to land.
- ✅ Resolved open question #1: claw's JSON output field is `message`
  (others observed: `auto_compaction`, `estimated_cost`, `iterations`,
  `model`, `prompt_cache_events`, `tool_results`, `tool_uses`, `usage`).
- ⏳ Live smoke-test subprocess mode against a built `claw` with the
  approval mediation flow (mock-mode round-trip is green).

### Phase 1.5 — permission mediation (LANDED 2026-04-29)

The first non-mock prompt against real claw revealed that
`--output-format json` is non-interactive: any tool call gated by an
`ask` rule (or a mode-mismatch under the cwd-precedence hardening)
auto-denies, leaving claw to generate a response without the tool's
output. Useless for real coding work.

Implemented: streaming mediation, all in `claw_web/claw_runner.py`.

- Spawn claw with `stdin=PIPE` instead of `DEVNULL`.
- Read stdout incrementally, ANSI-stripping each chunk into a clean
  buffer.
- Match the canonical "Permission approval required ... Approve this
  tool call? [y/N]:" block via `_APPROVAL_RE`. Extract Tool / Current
  mode / Required mode / Reason / Input.
- Emit `approval_request` event over the WS; await a callback that
  blocks on the client's `approval_response`.
- Write `y\n` / `n\n` to claw's stdin; remove the matched prompt from
  the buffer; loop. Multiple prompts per turn round-trip cleanly.
- On EOF, parse the trailing JSON blob from the cleaned buffer.

UI changes: `<dialog id="approval-dialog">` shows tool / current mode /
required mode / reason / pretty-printed input. Allow / Deny buttons send
the response. The in-progress turn switches to a `pending-approval`
visual (warn-color pulse) while the dialog is open.

No claw-side changes; zero collision with the parallel OAuth work.

Trade-off: this works because we know the prompt's exact text shape.
A claw rendering refactor would silently break it. The right long-term
fix is a structured prompt protocol upstream — `--permission-prompt-fd`
or a JSON-RPC channel — but that requires touching `rusty-claude-cli/
src/main.rs` which OAuth is also touching, so it's deferred.

### Phase 1.6 — multi-turn context (NOT YET RESOLVED, 2026-04-29)

Live testing exposed that claw's `--resume` is for **inspecting** saved
sessions with slash commands (`claw --resume <path> /status`), not for
continuing a conversation with a new user prompt. There's no CLI flag for
"add a new turn to this existing session" in non-interactive mode — the
only way to extend a session is the REPL's `/resume <path>` slash command.

So today, every claw-web turn runs in a fresh context. Bad UX for any
real coding work; an obvious thing the user will hit immediately.

**Options to fix, ordered by intrusiveness:**

1. **Inject conversation history as text in the prompt.** Read the
   previous turn's session JSONL, render the user/assistant exchange
   into a prefix, hand the combined text to claw as a fresh prompt.
   Cheap (50 lines), no claw changes, works today. Cost: token waste
   per turn proportional to history length, no real session continuity
   from claw's perspective (no resumed tool-call state, no compaction
   memory, no persisted MCP context).

2. **Long-lived REPL subprocess per claw-web session.** Spawn `claw`
   in interactive mode once when the user opens a session; pipe stdin
   for each new prompt; parse stdout-streamed events. Claw threads the
   conversation natively. Cost: significantly more parsing complexity
   (no tidy `--output-format json` blob — we'd have to parse the
   interactive output stream), and it's closer to the ttyd anti-pattern
   the project was meant to escape. But it's the "real" fix.

3. **Add a `--continue <session>` flag to claw.** Upstream change,
   collides with the parallel OAuth work in `rusty-claude-cli/src/
   main.rs`. Right answer long-term; deferred until OAuth merges.

**Verdict held until next session:** option 1 is probably worth doing
right away as a phase-1.6 stopgap — multi-turn becomes usable, the
implementation is contained to `claw_runner.py`, and option 2 or 3 can
land later without rework.

### Phase 2 — light-client + PWA + remote access

- iPad / low-power-laptop polish: composer keyboard handling, viewport
  fixes for soft keyboard, "modifier toolbar" above keyboard for `⌃` `⎋`.
- Real PWA: cache strategy in `sw.js`, icon set, install banner.
- Cloudflare Tunnel + Access wiring docs (NOT in this repo — those
  belong in the personal-deploy repo per
  `queued-package-multi-machine-install`).
- Auth: at minimum a shared secret on the WS upgrade; ideally CF Access
  in front when remote.
- Reconnect-friendly: WS reconnect with jitter, server-side
  in-progress-turn replay.

### Phase 3 — structured tool rendering

- Render tool calls as widgets, not text: file diffs, search results,
  bash output panels.
- File-tree sidebar that respects claw's permission mode (read-only
  branch shows the tree without write affordances).
- Optional: live token-count + cost display per turn.

### Phase 4 — direct runtime linkage (maybe)

- Replace the subprocess runner with a Rust shim that links the
  `runtime` crate directly. Eliminates per-turn process startup, gives
  us a real streaming surface, and makes tool-call rendering trivial.
- Cost: tight coupling to claw internals. Worth it only if the
  prototype graduates from "personal experiment" to "primary daily UI".

## Stack decisions

### Backend: Python + FastAPI

- Matches the rest of your local AI stack (brain MCP, Sentinel) — same
  uvicorn / pydantic shape, no new toolchain.
- Mock-shaped iteration is dramatically faster in Python than Rust;
  this is a prototype, not a binary.
- Subprocess management is ergonomic in `asyncio`.
- Migration path to Axum is a contained rewrite of `api.py` if/when
  Phase 4 lands.

### Frontend: vanilla HTML + JS, no build step

- One file (`app.js`) is small enough to read in a sitting. No bundler,
  no node_modules, no `npm audit` hellscape.
- Ships through FastAPI's `StaticFiles` mount — there's no separate
  frontend deploy story until Phase 2 puts the shell on Cloudflare Pages.
- Promotion path: if the frontend grows, swap in Preact + Vite. Avoid
  React/Next — overkill for a single-user app.

### Persistence: SQLite

- Single file, single user, file-shaped data. Postgres (Sentinel's
  choice) is overkill.
- Schema deliberately tiny: projects + sessions metadata only. Claw owns
  conversation history in its session JSONLs — duplicating that would be
  wrong.

### claw integration: subprocess via `--output-format json`

- The cleanest contract claw exposes today. Per-turn cost is ~1s
  spawn-up; acceptable for prototype, painful at production scale.
- `--resume <session-id>` keeps history continuous across turns without
  the wrapper having to model conversation state.
- The format is not yet stable — `claw_runner.py` does best-effort field
  extraction (`text`, `response`, `assistant`, `session_id`, `session`)
  and falls back to raw stdout if parsing fails. Tighten when the shape
  stabilizes.

## Borrowed and rejected (from Sentinel) — REVISED 2026-04-29

The original table dismissed tmux + xterm.js as wrong-shape. Live testing
proved this was a serious misread: those patterns exist because they
solve EXACTLY the problems we hit (multi-turn, approval mediation,
disconnect resilience, end-of-turn detection). See
`web-ui/PLAN.md` § "Phase 2 (revised)" below for the corrected target
architecture.

The corrected table:

| Pattern                                      | Verdict        | Reasoning (corrected)                                                                                                                                       |
|----------------------------------------------|----------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| tmux per session                             | ✅ borrow      | tmux IS the state store. Without it: multi-turn breaks (claw exits between turns), browser disconnect = lost work, server restart = lost work.               |
| xterm.js front end                           | ✅ borrow      | xterm.js *inside a structured shell* (project picker, file tree, header) is fundamentally different from ttyd-only. It lets claw be itself (rustyline, REPL, approval prompts via terminal interaction) while the surrounding UI adds project switching, file tree, etc. Best of both. |
| `CLAW_CONFIG_HOME=<per-session>` injection   | ✅ borrow      | Per-session verbosity/model overrides. Sentinel writes one settings.json per tmux session.                                                                    |
| Resilient exit-loop with R/T/Q banner        | 🟡 simplified | We don't need the multi-relaunch sentinel pattern, but a "claw exited; press a key to relaunch" banner inside the terminal IS useful for transient API errors. |
| Workspace label / mounted-repos env vars     | 🟡 borrow eventually | Surfaces the claw banner more usefully when a project has sub-repos.                                                                                  |
| bwrap filesystem isolation                   | ❌ skip        | Single-user, trusted operator. Claw's own permission system suffices. Sentinel needs this because it runs untrusted-ish prompts on shared infra.              |
| Postgres                                     | ❌ skip        | Wrong shape for single-user. SQLite stays.                                                                                                                   |
| Multi-user / sharing / sessions auth         | ❌ skip        | Single-user.                                                                                                                                                |
| Confidence-weighted pattern extraction       | ✅ already     | Already ported into Ai-Brain.                                                                                                                                |

## Phase 2 (revised) — tmux + xterm.js pivot

The phase-1 + phase-1.5 + phase-1.6 architecture (subprocess per turn +
JSON-output parsing + approval-prompt regex mediation + history
injection) is a USEFUL STOPGAP that gets multi-turn working with code
we control end-to-end. But it's structurally fragile (claw rendering
changes break the regex; history injection burns tokens; no real
session continuity in claw's memory) and the right architectural fit
is what Sentinel chose.

### Target architecture

- `claw_web` continues to own: project registry, session metadata,
  the structured UI shell (sidebar, project picker, header, modals).
- The conversation panel swaps from "JSON render of run_turn events" to
  "xterm.js terminal bound to a PTY-relay WebSocket".
- The server-side runner becomes a tmux-session manager:
  - `manager.create_tmux_session(session_id, cwd, model, ...)` spawns
    tmux running the user's `cl` wrapper (or `claw` directly).
  - `manager.attach_pty(session_id)` opens a PTY and `tmux attach`-es
    inside it; returns the master fd.
  - `manager.kill_tmux_session(session_id)` cleans up on session
    delete.
- The WS handler relays bytes:
  - PTY → WS (terminal output).
  - WS → PTY (user keystrokes, including `y`/`n` for approval prompts).
  - WS → tmux resize on terminal-size events from xterm.js.
- Persistence is implicit: tmux holds the claw REPL between WS
  connects, so reconnect resumes the conversation natively.

### What gets deleted in the pivot

- `claw_runner.py`'s subprocess-spawn-and-stream logic.
- `_APPROVAL_RE` and the approval-prompt parsing.
- `_extract_text` / `_extract_last_json_object`.
- `_read_session_history` / `_format_history_prompt` (the option-1
  stopgap is no longer needed — claw remembers internally).
- The approval `<dialog>` and its WS protocol.
- `mock_runner.py`'s scripted-approval flow (mock mode itself can
  keep working with simple scripted output).

### What stays

- All of `projects.py`, `sessions.py`, `db.py`, `config.py`.
- The structured UI shell.
- PWA framing.
- The Abort button (now tmux-kill-session).

### Estimate

Sentinel's `terminal/manager.py` + `terminal/routes.py` is ~1000 lines.
We don't need: bwrap, multi-user auth, session sharing, ai-account
pre-flight, ccproxy, integration credential injection, workspace
isolation, n8n integration. The minimal port for single-user is
~250-350 lines of Python + ~150 lines of frontend (xterm.js binding +
WS event handling).

### Migration order when we pivot

1. Add tmux-session manager (new file).
2. Add PTY-relay WS endpoint (alongside the existing one).
3. Add xterm.js panel to the frontend (alongside the existing
   conversation rendering, behind a feature flag).
4. Live-test multi-turn through the new path.
5. Delete the old subprocess runner + approval modal + history
   injection.

The "alongside, then delete" ordering means we never break the working
phase-1 path while the new path stabilizes.

## Open questions

These are the things to decide before the scaffold becomes real.

1. ~~**Where does claw's session-id show up in `--output-format json`?**~~
   **RESOLVED (2026-04-29):** the assistant text field is `message`. The
   session id field is still under-investigated — `parsed.get("session_id")`
   may be wrong; check against the next live turn. Other top-level keys
   confirmed: `auto_compaction`, `estimated_cost`, `iterations`, `model`,
   `prompt_cache_events`, `tool_results`, `tool_uses`, `usage`.

2. **Multi-tab semantics.** Two browser tabs on the same session: do
   they share a WS, or does one win? Sentinel solves this with tmux
   attach semantics (multiple viewers, single source of truth). For us,
   single WS per session with last-writer-wins is the simplest answer
   that doesn't require cross-tab coordination — but we should verify
   it's what we actually want before locking it in.

3. **New-project flow.** Phase 1 requires the directory to already
   exist. Phase 2 should offer scaffolding ("create from git URL",
   "scaffold from a template"). Worth deciding now whether templates
   live in the repo or in a separate registry.

4. **PWA caching strategy.** The current `sw.js` is intentionally
   permissive (network-first, no aggressive caching). Once we go
   remote, we'll want stale-while-revalidate for the shell + true
   network-only for `/api/*`. Verify on iOS Safari which is notoriously
   aggressive about backgrounding WebSockets.

5. **Does the personal-deploy repo (queued initiative) ship this?**
   Two-repo packaging is fine, but a user adding a new machine should
   get one install command that lights up `cl`, ttyd (or claw-web),
   brain MCP, and the vault. Decide whether claw-web is part of that
   bundle or a separate optional install.

6. **Auth for remote.** Phase 2 question, but the architecture should
   accommodate it now. Plan: trust the WS upgrade if a header is
   present (set by Cloudflare Access on the edge); fail closed otherwise.

7. **Approval-prompt parsing fragility.** The `_APPROVAL_RE` regex
   depends on claw's exact prompt rendering. Any TUI refactor breaks
   us silently. Mitigations: (a) snapshot test against the real prompt
   text periodically; (b) eventually push for a structured prompt
   protocol upstream once OAuth merges and `rusty-claude-cli/src/
   main.rs` is no longer hot.

8. **Allow-once vs allow-for-session vs always-allow.** Current modal
   has a single "Allow once" button. Should we offer "Allow for the
   rest of this turn" / "Always allow" controls? The latter would write
   into `~/.claw/settings.json`'s `allow` list — probably out of scope
   for this app (that's the user's job). Worth deciding when the
   prompt fatigue becomes real.

## Non-goals (for now)

Things explicitly NOT in scope, so we don't drift:

- Multi-user. Single-user is the entire premise.
- Slack / Discord / webhook integrations. That's clawhip's job.
- Replacing the TUI. The TUI keeps existing for shell-native workflows;
  this is additive.
- Touching upstream claw-code. The hardened fork is where this lives.
