# TUI Runtime Integration Report: claw-code

**Date:** 2026-06-12  
**Repo:** `/mnt/data/git/claw-code` (branch `feat-tui`)  
**Scope:** Map every integration point a TUI layer must call and every event it must receive

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [ConversationRuntime — the Core Loop](#2-conversationruntime--the-core-loop)
3. [TurnProgressReporter Trait](#3-turnprogressreporter-trait)
4. [run_turn() Streaming Model](#4-run_turn-streaming-model)
5. [Tool Execution Events & Lifecycle](#5-tool-execution-events--lifecycle)
6. [Permission Prompting](#6-permission-prompting)
7. [Compaction Flow](#7-compaction-flow)
8. [Reasoning / Thinking Content Streaming](#8-reasoning--thinking-content-streaming)
9. [AssistantEvent Enum — Full Event Catalog](#9-assistantevent-enum--full-event-catalog)
10. [Rendering Pipeline (Markdown → ANSI)](#10-rendering-pipeline-markdown--ansi)
11. [Input Layer (rustyline)](#11-input-layer-rustyline)
12. [Session Persistence](#12-session-persistence)
13. [Hook System](#13-hook-system)
14. [Integration Points Summary](#14-integration-points-summary)
15. [Proposed TUI Event Channel Design](#15-proposed-tui-event-channel-design)
16. [Key Risks & Open Questions](#16-key-risks--open-questions)

---

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│  CLI (rusty-claude-cli)                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │  input.rs    │  │  render.rs   │  │  main.rs               │  │
│  │  rustyline   │  │  pulldown-   │  │  Cli struct             │  │
│  │  LineEditor  │  │  cmark +     │  │  ├── run_turn_to()      │  │
│  │  ReadOutcome │  │  syntect     │  │  ├── CliPermission-     │  │
│  │              │  │  Terminal-   │  │  │   Prompter           │  │
│  │              │  │  Renderer    │  │  ├── AnthropicRuntime-  │  │
│  │              │  │  Markdown-   │  │  │   Client (ApiClient) │  │
│  │              │  │  StreamState │  │  └── BuiltRuntime       │  │
│  └──────────────┘  └──────────────┘  └────────┬───────────────┘  │
│                                               │                  │
└───────────────────────────────────────────────┼──────────────────┘
                                                │ calls
                    ┌───────────────────────────┘
                    ▼
┌──────────────────────────────────────────────────────────────────┐
│  Runtime Crate (runtime)                                         │
│                                                                   │
│  ConversationRuntime<C: ApiClient, T: ToolExecutor>              │
│  ├── session: Session                                            │
│  ├── api_client: C                                               │
│  ├── tool_executor: T                                            │
│  ├── permission_policy: PermissionPolicy                         │
│  ├── hook_runner: HookRunner                                     │
│  ├── usage_tracker: UsageTracker                                  │
│  ├── turn_progress_reporter: Option<Box<dyn TurnProgressReporter>>│
│  ├── hook_progress_reporter: Option<Box<dyn HookProgressReporter>>│
│  └── session_tracer: Option<SessionTracer>                       │
│                                                                   │
│  Key methods:                                                    │
│  │  run_turn(input, prompter) → Result<TurnSummary, RuntimeError>│
│  │  compact(config) → CompactionResult                           │
│  │  fork_session(branch) → Session                                │
│  │  session() / session_mut() → &Session                         │
│  │  usage() → &UsageTracker                                      │
│  └────────────────────────────────────────────────────────────────│
└──────────────────────────────────────────────────────────────────┘
```

The CLI's `Cli` struct owns a `BuiltRuntime` (which wraps `ConversationRuntime<AnthropicRuntimeClient, CliToolExecutor>` plus plugin/MCP lifecycle). The TUI must either:

- **(A)** Own a similar `BuiltRuntime` and call `run_turn()` the same way, but intercept I/O instead of using stdout/stdin, OR
- **(B)** Refactor the runtime to emit typed events through a channel instead of writing to an `io::Write` sink.

---

## 2. ConversationRuntime — the Core Loop

**File:** `runtime/src/conversation.rs`

The runtime is generic over `C: ApiClient` and `T: ToolExecutor`:

```rust
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
    auto_compaction_input_tokens_threshold: u32,
    hook_abort_signal: HookAbortSignal,
    hook_progress_reporter: Option<Box<dyn HookProgressReporter>>,
    session_tracer: Option<SessionTracer>,
    turn_progress_reporter: Option<Box<dyn TurnProgressReporter>>,
}
```

**Builder-style setters:**
- `.with_max_iterations(n)`
- `.with_auto_compaction_input_tokens_threshold(t)`
- `.with_hook_abort_signal(signal)`
- `.with_hook_progress_reporter(reporter)`
- `.with_session_tracer(tracer)`
- `.with_turn_progress_reporter(reporter)`

**Key public API:**
| Method | Returns | Notes |
|--------|---------|-------|
| `run_turn(input, prompter)` | `Result<TurnSummary, RuntimeError>` | Core loop — blocks until turn completes |
| `compact(config)` | `CompactionResult` | Manual compaction |
| `estimated_tokens()` | `usize` | Current token footprint |
| `session()` / `session_mut()` | `&Session` | Read/write session state |
| `usage()` | `&UsageTracker` | Cumulative token usage |
| `api_client_mut()` | `&mut C` | Access underlying API client |
| `fork_session(branch)` | `Session` | Branch the conversation |
| `into_session(self)` | `Session` | Consume runtime, keep session |
| `set_auto_compaction_input_tokens_threshold(t)` | `()` | Adjust threshold at runtime |

---

## 3. TurnProgressReporter Trait

**File:** `runtime/src/conversation.rs:165-175`

```rust
pub trait TurnProgressReporter: Send + Sync {
    fn on_tool_result(
        &self,
        iteration: usize,
        max_iterations: usize,
        tool_name: &str,
        input: &str,
        result: Result<&str, &str>,
    );
}
```

### What events does it emit?

**Only one:** `on_tool_result` — called after **each tool execution completes** (success or failure), inside the Phase 3 loop of `run_turn()`.

### Parameters

| Param | Meaning |
|-------|---------|
| `iteration` | 1-based index of the current loop iteration within the turn |
| `max_iterations` | The configured maximum (default: `usize::MAX`) |
| `tool_name` | e.g., `"read_file"`, `"bash"` |
| `input` | The effective tool input (after pre-hook mutation) |
| `result` | `Ok(output_str)` or `Err(error_str)` |

### When is it called?

After:
1. Tool pre-hooks run
2. Permission check passes
3. `tool_executor.execute_batch()` completes 
4. Tool post-hooks run
5. Result message is pushed to session

The reporter is called **once per tool** in a batch, in original call order.

### Can a TUI version send events through a channel?

**Yes, easily.** The trait is `Send + Sync`, so a channel-based implementation is straightforward:

```rust
struct ChannelProgressReporter {
    tx: mpsc::Sender<TuiEvent>,
}

impl TurnProgressReporter for ChannelProgressReporter {
    fn on_tool_result(
        &self,
        iteration: usize,
        max_iterations: usize,
        tool_name: &str,
        input: &str,
        result: Result<&str, &str>,
    ) {
        let _ = self.tx.send(TuiEvent::ToolResult {
            iteration,
            max_iterations,
            tool_name: tool_name.to_string(),
            input: input.to_string(),
            result: result.map(str::to_string).map_err(str::to_string),
        });
    }
}
```

### ⚠️ Limitation

`TurnProgressReporter` does **NOT** emit:
- Tool-start events (when a tool begins execution)
- Streaming text deltas
- Thinking/reasoning events
- Token usage updates
- Auto-compaction events
- Permission prompts

For those, the TUI needs additional hooks (see Section 15).

---

## 4. run_turn() Streaming Model

### How does run_turn() stream tokens?

**It doesn't.** `run_turn()` is a **synchronous blocking call** that returns a `TurnSummary` after the entire turn completes. The streaming happens *inside* `ApiClient::stream()`, which is also blocking from the runtime's perspective — it returns `Result<Vec<AssistantEvent>, RuntimeError>` (a collected Vec, not a stream).

The current CLI architecture streams tokens to stdout **inside** `AnthropicRuntimeClient::stream()` (which implements `ApiClient::stream()`). The streaming is side-effectual: it writes ANSI-rendered markdown to an `io::Write` sink **while** collecting events.

### The streaming flow

```
run_turn(input, prompter)
 └─► api_client.stream(request)       // blocks until stream ends
      └─► tokio::Runtime::block_on(async {
           └─► client.stream_message(request).await
                └─► loop {
                     event = stream.next_event().await
                     match event {
                       ContentBlockStart → write to out; push to events
                       ContentBlockDelta::TextDelta → 
                         MarkdownStreamState::push() → write ANSI to out
                         push AssistantEvent::TextDelta
                       ContentBlockDelta::ThinkingDelta → 
                         accumulate into pending_thinking
                       ContentBlockDelta::InputJsonDelta →
                         accumulate into pending_tool input
                       ContentBlockDelta::SignatureDelta →
                         accumulate into pending_thinking signature
                       ContentBlockStop →
                         flush MarkdownStreamState
                         emit accumulated Thinking as AssistantEvent::Thinking
                         emit accumulated ToolUse as AssistantEvent::ToolUse
                       MessageDelta → push AssistantEvent::Usage
                       MessageStop → push AssistantEvent::MessageStop
                     }
                   })
           return Vec<AssistantEvent>
          })
```

### Key insight for TUI

The `consume_stream()` method determines the "out" writer at the top:

```rust
let mut stdout = io::stdout();
let mut sink = io::sink();
let out: &mut dyn Write = if self.emit_output {
    &mut stdout
} else {
    &mut sink
};
```

Currently, the TUI workaround (`run_turn_to`) uses a `Vec<u8>` buffer as the writer and reads it after the turn completes. **This means no live streaming in TUI mode** — the TUI gets the rendered output only after the entire turn finishes.

### Is there a callback/channel pattern to hook into?

**Not currently.** The three extension points are:

1. **`TurnProgressReporter`** — only `on_tool_result` (post-tool, not during streaming)
2. **`HookProgressReporter`** — hook lifecycle events (Started/Completed/Cancelled)
3. **`InternalPromptProgressReporter`** — CLI-internal progress for ultraplan mode

None of these fire during the token-by-token streaming inside `consume_stream()`.

### What the TUI needs

To get live streaming in the TUI, one of these approaches is needed:

**Option A: Write to a channel instead of io::Write**

Replace the `out: &mut dyn Write` in `consume_stream()` with an enum that can write to either stdout or an `mpsc::Sender<String>`. Currently the renderer writes ANSI-escaped markdown incrementally, so the TUI could receive ANSI strings through the channel.

**Option B: Emit typed events instead of raw ANSI**

Refactor `consume_stream()` to accept an event sink trait:

```rust
trait StreamSink {
    fn on_text_delta(&mut self, text: &str);
    fn on_thinking_start(&mut self);
    fn on_thinking_delta(&mut self, text: &str);
    fn on_thinking_end(&mut self, char_count: usize, redacted: bool);
    fn on_tool_start(&mut self, name: &str, input: &str);
    fn on_tool_result(&mut self, name: &str, output: &str, is_error: bool);
    fn on_usage(&mut self, usage: &TokenUsage);
    fn on_message_stop(&mut self);
}
```

**Option B is strongly recommended** because:
- The TUI needs to know event types (text vs thinking vs tool call) for layout
- Raw ANSI in a channel defeats the purpose of a structured TUI
- It allows the TUI to render each event type with its own widget style

---

## 5. Tool Execution Events & Lifecycle

### Within run_turn()

The tool execution lifecycle inside `run_turn()` is:

```
1. API stream → assistant_message extracted
2. pending_tool_uses = filter ToolUse blocks from message
   
   ┌─── Phase 1: Pre-hooks + Permissions (sequential) ───┐
   │  For each (tool_use_id, tool_name, input):           │
   │    a. run_pre_tool_use_hook(tool_name, input)        │
   │    b. Apply updated_input from hook                  │
   │    c. Check if hook denied/cancelled/failed          │
   │    d. permission_policy.authorize_with_context(...)   │
   │       → may call PermissionPrompter::decide()        │
   │    e. Record PendingTool { allowed, deny_reason }    │
   └──────────────────────────────────────────────────────┘
   
   ┌─── Phase 2: Execute allowed tools (batch/parallel) ─┐
   │  allowed_calls = filter pending where allowed         │
   │  record_tool_started(iteration, tool_name)            │
   │  batch_results = tool_executor.execute_batch(calls)   │
   └───────────────────────────────────────────────────────┘
   
   ┌─── Phase 3: Post-hooks + Session updates (sequential) ─┐
   │  For each pending tool:                                  │
   │    a. Get batch_result (if allowed)                     │
   │    b. run_post_tool_use_hook / run_post_tool_use_failure│
   │    c. Merge hook feedback into output                    │
   │    d. session.push_message(tool_result)                  │
   │    e. record_tool_finished(iteration, result_message)    │
   │    f. turn_progress_reporter.on_tool_result(...)         │
   └──────────────────────────────────────────────────────────┘
   
   Loop back to step 1 (next API call with tool results)
```

### What events exist?

| Event | Source | Data | Currently Observable? |
|-------|--------|------|---------------------|
| `tool_pre_hook_started` | `HookProgressReporter::on_event(Started)` | event, tool_name, command | ✅ via HookProgressReporter |
| `tool_pre_hook_completed` | `HookProgressReporter::on_event(Completed)` | event, tool_name, command | ✅ via HookProgressReporter |
| `tool_pre_hook_cancelled` | `HookProgressReporter::on_event(Cancelled)` | event, tool_name, command | ✅ via HookProgressReporter |
| `permission_prompt` | `PermissionPrompter::decide()` | tool_name, input, modes | ✅ via PermissionPrompter |
| `tool_execution_started` | `record_tool_started()` (SessionTracer) | iteration, tool_name | ⚠️ only via SessionTracer |
| `tool_execution_finished` | `record_tool_finished()` (SessionTracer) | iteration, tool_name, is_error | ⚠️ only via SessionTracer |
| `tool_result` | `TurnProgressReporter::on_tool_result()` | iteration, tool_name, input, result | ✅ via TurnProgressReporter |

### ⚠️ Tool execution start has no TUI-visible event

The `record_tool_started()` method only emits to `SessionTracer` (not to `TurnProgressReporter`). There is **no `on_tool_start` callback**. The TUI currently has no way to show "Running tool X…" before the tool completes.

**For the TUI**, you need either:
1. Add an `on_tool_start` method to `TurnProgressReporter`
2. Use `HookProgressReporter::on_event(Started)` which fires at pre-hook start (close to tool start)
3. Infer tool start from `AssistantEvent::ToolUse` in the stream (but this is inside `consume_stream()`, not `run_turn()`)

---

## 6. Permission Prompting

### Current CLI Implementation

**File:** `main.rs:12766-12813`

```rust
struct CliPermissionPrompter {
    current_mode: PermissionMode,
}

impl PermissionPrompter for CliPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        if let Some(reason) = &request.reason {
            println!("  Reason           {reason}");
        }
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    PermissionPromptDecision::Allow
                } else {
                    PermissionPromptDecision::Deny { reason: ... }
                }
            }
            Err(error) => PermissionPromptDecision::Deny { reason: ... },
        }
    }
}
```

### PermissionRequest structure

```rust
pub struct PermissionRequest {
    pub tool_name: String,
    pub input: String,
    pub current_mode: PermissionMode,
    pub required_mode: PermissionMode,
    pub reason: Option<String>,
}
```

### PermissionPromptDecision

```rust
pub enum PermissionPromptDecision {
    Allow,
    Deny { reason: String },
}
```

### PermissionMode hierarchy

```rust
pub enum PermissionMode {
    ReadOnly,        // "read-only"
    WorkspaceWrite,  // "workspace-write"
    DangerFullAccess,// "danger-full-access"
    Prompt,          // "prompt" (ask every time)
    Allow,           // "allow" (auto-approve)
}
```

### How should the TUI handle permissions?

The `PermissionPrompter` trait is synchronous and blocking. For a TUI, the implementation should:

1. **Send a permission request event** through a channel to the TUI event loop
2. **Block on a response channel** until the user selects Allow/Deny in the TUI
3. Use `std::sync::mpsc` or a oneshot channel for the response:

```rust
struct TuiPermissionPrompter {
    tx: mpsc::Sender<TuiEvent>,
    rx: mpsc::Receiver<PermissionPromptDecision>,
}

impl PermissionPrompter for TuiPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        let _ = self.tx.send(TuiEvent::PermissionRequired {
            tool_name: request.tool_name.clone(),
            required_mode: request.required_mode,
            input: request.input.clone(),
            reason: request.reason.clone(),
        });
        self.rx.recv().unwrap_or(PermissionPromptDecision::Deny {
            reason: "TUI disconnected".into(),
        })
    }
}
```

The TUI renders a modal/bottom-bar with the permission details and two buttons. On click, it sends the decision back through the channel.

---

## 7. Compaction Flow

### Auto-compaction in run_turn()

Auto-compaction is checked **after each assistant message is pushed** (whether or not tools were used):

```rust
// Inside the loop in run_turn():
if let Some(compaction) = self.maybe_auto_compact() {
    auto_compaction = Some(compaction);
}
```

### maybe_auto_compact()

```rust
fn maybe_auto_compact(&mut self) -> Option<AutoCompactionEvent> {
    if self.usage_tracker.cumulative_usage().input_tokens
        < self.auto_compaction_input_tokens_threshold
    {
        return None;
    }

    let result = compact_session(
        &self.session,
        CompactionConfig {
            max_estimated_tokens: 0,
            ..CompactionConfig::default()
        },
    );

    if result.removed_message_count == 0 {
        return None;
    }

    self.session = result.compacted_session;
    Some(AutoCompactionEvent {
        removed_message_count: result.removed_message_count,
    })
}
```

Threshold: default 100,000 input tokens (configurable via `CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS` env var).

### AutoCompactionEvent

```rust
pub struct AutoCompactionEvent {
    pub removed_message_count: usize,
}
```

Returned in `TurnSummary.auto_compaction`. The CLI displays it:

```rust
if let Some(event) = summary.auto_compaction {
    writeln!(out, "{}", format_auto_compaction_notice(event.removed_message_count))?;
}
```

### Manual compaction

Triggered by `/compact` command via `runtime.compact(config)` → returns `CompactionResult`:

```rust
pub struct CompactionResult {
    pub summary: String,
    pub formatted_summary: String,
    pub compacted_session: Session,
    pub removed_message_count: usize,
}
```

### Context-window retry loop

When the API returns a context window error, `run_turn_to()` enters a retry loop that:
1. Extracts context window size from the error
2. Auto-compacts with decreasing `preserve_recent_messages` (4 → 2 → 1 → 0)
3. Retries `run_turn()` with the compacted session
4. Up to 4 rounds

**TUI must handle:** Displaying compaction progress and "auto-compacting…" messages during this retry loop.

### Does compaction emit events the TUI needs?

Currently, **no events are emitted during compaction itself**. The CLI writes directly to its `out` writer. The TUI would need:
- A `Compacting { round, max_rounds, removed_count }` event
- A `CompactionComplete { removed_message_count }` event

---

## 8. Reasoning / Thinking Content Streaming

### How thinking works in the protocol

The Anthropic API and OpenAI-compatible APIs send thinking/reasoning as separate content blocks with deltas:

1. `ContentBlockStart::Thinking { thinking, signature }` — initial thinking text (may be partial)
2. `ContentBlockDelta::ThinkingDelta { thinking }` — incremental thinking chunks
3. `ContentBlockDelta::SignatureDelta { signature }` — signing data for verified thinking
4. `ContentBlockStop` — marks end of the thinking block

For OpenAI-compatible models (DeepSeek V4), `reasoning_content` is mapped to `ThinkingDelta`.

### How the CLI handles it

Inside `consume_stream()`:

```rust
// Accumulate thinking into pending_thinking
let mut pending_thinking: Option<(String, Option<String>)> = None;
let mut block_has_thinking_summary = false;

// On ContentBlockStart::Thinking:
pending_thinking = Some((thinking.clone(), signature.clone()));

// On ThinkingDelta:
if !block_has_thinking_summary {
    render_thinking_block_summary(out, None, false)?;  // "▶ Thinking hidden"
    block_has_thinking_summary = true;
}
if let Some((t, _)) = &mut pending_thinking {
    t.push_str(&thinking);
}

// On SignatureDelta:
if let Some((_, sig)) = &mut pending_thinking {
    sig.get_or_insert_with(String::new).push_str(&signature);
}

// On ContentBlockStop:
block_has_thinking_summary = false;
if let Some((thinking, signature)) = pending_thinking.take() {
    events.push(AssistantEvent::Thinking { thinking, signature });
}
```

### `render_thinking_block_summary()`

```rust
fn render_thinking_block_summary(
    out: &mut (impl Write + ?Sized),
    char_count: Option<usize>,
    redacted: bool,
) -> Result<(), RuntimeError> {
    let summary = if redacted {
        "\n▶ Thinking block hidden by provider\n".to_string()
    } else if let Some(char_count) = char_count {
        format!("\n▶ Thinking ({char_count} chars hidden)\n")
    } else {
        "\n▶ Thinking hidden\n".to_string()
    };
    write!(out, "{summary}")...
}
```

**Key observation:** The CLI **hides** the thinking content in the terminal — it only shows "▶ Thinking (N chars hidden)". The actual thinking text is preserved in `AssistantEvent::Thinking` for session persistence but not displayed.

### For the TUI

The TUI has more screen real estate and could:
- **Show a collapsible thinking block** with the actual content
- **Show a live streaming thinking indicator** that expands on click
- Stream `ThinkingDelta` chunks directly to a TUI panel

But this requires the `StreamSink` approach (Option B from §4) since `ThinkingDelta` events are currently consumed inside `consume_stream()` and only the final accumulated `AssistantEvent::Thinking` is passed to the caller.

---

## 9. AssistantEvent Enum — Full Event Catalog

```rust
pub enum AssistantEvent {
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    TextDelta(String),
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    Usage(TokenUsage),
    PromptCache(PromptCacheEvent),
    MessageStop,
}
```

| Event | When | TUI needs? |
|-------|------|------------|
| `Thinking` | After thinking block completes | ✅ Show in collapsible section |
| `TextDelta` | Per text chunk | ✅ Append to response area |
| `ToolUse` | After tool input fully accumulated | ✅ Show "Running tool…" |
| `Usage` | Per message delta (token counts) | ✅ Token counter |
| `PromptCache` | Stream metadata | ❌ Internal only |
| `MessageStop` | Stream ends | ✅ Mark turn complete |

**Note:** These events are collected inside `ApiClient::stream()` and returned as a `Vec`. They are NOT streamed to the caller one-by-one. This is the core architectural problem for live TUI updates.

---

## 10. Rendering Pipeline (Markdown → ANSI)

**File:** `rusty-claude-cli/src/render.rs`

### TerminalRenderer

```rust
pub struct TerminalRenderer {
    syntax_set: SyntaxSet,
    syntax_theme: Theme,
    color_theme: ColorTheme,
}
```

Key methods:
- `render_markdown(markdown) → String` — full markdown → ANSI
- `markdown_to_ansi(markdown) → String` — alias
- `color_theme() → &ColorTheme`

### MarkdownStreamState (incremental rendering)

```rust
pub struct MarkdownStreamState {
    pending: String,
}
```

- `push(renderer, delta) → Option<String>` — append delta, render complete chunks
- `flush(renderer) → Option<String>` — render remaining pending text

This handles the problem that markdown can't be partially rendered (e.g., a `**` bold marker spans two chunks). It finds a "stream-safe boundary" and only renders complete blocks.

### ColorTheme

```rust
pub struct ColorTheme {
    heading: Color,      // Cyan
    emphasis: Color,     // Magenta
    strong: Color,       // Yellow
    inline_code: Color,  // Green
    link: Color,         // Blue
    quote: Color,        // DarkGrey
    table_border: Color, // DarkCyan
    code_block_border: Color, // DarkGrey
    spinner_active: Color,    // Blue
    spinner_done: Color,      // Green
    spinner_failed: Color,    // Red
}
```

### For the TUI

The TUI should **NOT** use ANSI rendering. Instead:
- Parse the raw markdown with `pulldown-cmark` directly (same parser, different renderer)
- Map markdown events to TUI widget primitives (heading → styled text, code block → syntax-highlighted block, etc.)
- The `TerminalRenderer` stays available for fallback/legacy mode

The incremental `MarkdownStreamState` logic (boundary detection) could be adapted for the TUI.

---

## 11. Input Layer (rustyline)

**File:** `rusty-claude-cli/src/input.rs`

### LineEditor

```rust
pub struct LineEditor {
    prompt: String,
    editor: Editor<SlashCommandHelper, DefaultHistory>,
}
```

### ReadOutcome

```rust
pub enum ReadOutcome {
    Submit(String),
    Cancel,
    Exit,
    ProviderSwap,
    TeamToggle,
}
```

### Key bindings
- `Enter` — submit
- `Ctrl+J` / `Shift+Enter` — newline (multiline input)
- `Ctrl+P` — provider swap (inserts `\x01` sentinel)
- `Ctrl+T` — team toggle (inserts `\x02` sentinel)
- `Ctrl+C` — cancel current input (or exit if empty)
- `Ctrl+D` — exit

### For the TUI

The TUI replaces `LineEditor` entirely with its own text input widget. The `ReadOutcome` enum should still be used as the adapter interface, but the TUI input widget handles key events directly.

Slash command completions come from the REPL loop's `SlashCommandHelper.completions` list.

---

## 12. Session Persistence

**File:** `runtime/src/session.rs`

### Key types

```rust
pub struct Session {
    pub id: String,
    pub messages: Vec<ConversationMessage>,
    pub compaction: Option<SessionCompaction>,
}

pub struct SessionCompaction {
    pub count: u32,
    pub removed_message_count: usize,
    pub summary: String,
}

pub struct ConversationMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    pub usage: Option<TokenUsage>,
}

pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String, signature: Option<String> },
    ToolUse { id: String, name: String, input: String },
    ToolResult { tool_use_id: String, tool_name: String, output: String, is_error: bool },
}

pub enum MessageRole { System, User, Assistant, Tool }
```

Session is serialized to JSONL (`session.jsonl`) with rotation (256KB max, 3 rotated files). Large fields are truncated at 16KB.

### For the TUI

The TUI reads `Session.messages` to render the conversation history. No changes to the persistence format are needed — the runtime handles all reads/writes.

---

## 13. Hook System

**File:** `runtime/src/hooks.rs`

### HookEvent

```rust
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
}
```

### HookProgressEvent

```rust
pub enum HookProgressEvent {
    Started { event: HookEvent, tool_name: String, command: String },
    Completed { event: HookEvent, tool_name: String, command: String },
    Cancelled { event: HookEvent, tool_name: String, command: String },
}
```

### HookProgressReporter trait

```rust
pub trait HookProgressReporter {
    fn on_event(&mut self, event: &HookProgressEvent);
}
```

### For the TUI

`HookProgressReporter` can be implemented with a channel sender to show hook activity in the TUI (e.g., "Running pre-tool hook for bash…").

---

## 14. Integration Points Summary

### What the TUI Needs to CALL

| Call | Method | Notes |
|------|--------|-------|
| Start a turn | `runtime.run_turn(input, prompter)` | Blocks until complete |
| Compact session | `runtime.compact(config)` | Manual `/compact` |
| Get session state | `runtime.session()` | For rendering history |
| Get usage stats | `runtime.usage()` | Token counter display |
| Get estimated tokens | `runtime.estimated_tokens()` | Status bar |
| Set compaction threshold | `runtime.set_auto_compaction_threshold(t)` | After context window error |
| Fork session | `runtime.fork_session(branch)` | `/session fork` |
| Prepare runtime | `cli.prepare_turn_runtime(emit_output)` | Rebuilds runtime from session |
| Replace runtime | `cli.replace_runtime(runtime)` | After compaction retry |

### What the TUI Needs to RECEIVE

| Event | Current Source | Proposed TUI Source |
|-------|---------------|-------------------|
| Text delta (streaming) | **Inside consume_stream()** | Refactor: StreamSink |
| Thinking delta (streaming) | **Inside consume_stream()** | Refactor: StreamSink |
| Tool use start | `HookProgressReporter::Started` | Channel from HookProgressReporter |
| Tool use complete | `TurnProgressReporter::on_tool_result` | Channel from TurnProgressReporter |
| Permission prompt | `PermissionPrompter::decide()` | Channel + response channel |
| Token usage update | `AssistantEvent::Usage` (post-stream) | StreamSink or TurnSummary |
| Auto-compaction happened | `TurnSummary.auto_compaction` | Post-turn, or event |
| Auto-compaction retry | `run_turn_to()` retry loop | Needs new event source |
| Message stop | `AssistantEvent::MessageStop` | StreamSink |
| Hook started/completed | `HookProgressReporter` | Channel from reporter |
| Spinner state | CLI writes directly | TUI manages its own |

### Current Missing Events (Must Be Added for TUI)

| Missing Event | Why Needed |
|---------------|-----------|
| `on_tool_start(iteration, tool_name, input)` | Show "Running X…" before result |
| `on_text_delta(text)` | Live streaming text to response area |
| `on_thinking_delta(text)` | Live thinking indicator / collapsible |
| `on_compacting(round, max_rounds, removed)` | Compaction retry progress |
| `on_auto_compaction(event)` | Mid-turn compaction notification |
| `on_permission_prompt(request)` | Delegated to PermissionPrompter (has channel solution) |

---

## 15. Proposed TUI Event Channel Design

### Core Idea

Replace the `io::Write`-based output with a typed event channel that the TUI consumes in its render loop.

### Event Enum

```rust
enum TuiEvent {
    // ── Streaming events (from consume_stream refactor) ──
    TurnStarted { input: String },
    TextDelta { text: String },
    ThinkingStart,
    ThinkingDelta { text: String },
    ThinkingEnd { char_count: usize, redacted: bool },
    ToolUseStart { id: String, name: String, input: String },
    ToolUseResult { id: String, name: String, output: String, is_error: bool },
    UsageUpdate { usage: TokenUsage },
    MessageStop,

    // ── Tool lifecycle (from TurnProgressReporter + new hooks) ──
    ToolStarted { iteration: usize, tool_name: String },
    ToolResult { 
        iteration: usize, 
        tool_name: String, 
        result: Result<String, String> 
    },

    // ── Permission (from PermissionPrompter channel) ──
    PermissionRequired {
        tool_name: String,
        required_mode: PermissionMode,
        input: String,
        reason: Option<String>,
        // Response sent back through separate oneshot channel
        response_tx: oneshot::Sender<PermissionPromptDecision>,
    },

    // ── Compaction ──
    Compacting { round: usize, max_rounds: usize, removed: usize },
    AutoCompaction { removed_message_count: usize },

    // ── Hook progress ──
    HookStarted { event: HookEvent, tool_name: String, command: String },
    HookCompleted { event: HookEvent, tool_name: String, command: String },
    HookCancelled { event: HookEvent, tool_name: String, command: String },

    // ── Lifecycle ──
    TurnCompleted { summary: TurnSummary },
    TurnFailed { error: RuntimeError },
}
```

### Integration Architecture

```
┌───────────────────────┐
│       TUI Loop        │
│  ┌─────────────────┐  │
│  │  Event Receiver │◄─── mpsc::Receiver<TuiEvent>
│  │  (render loop)  │  │
│  └─────────────────┘  │
│  ┌─────────────────┐  │
│  │  Input Widget    │───► on submit: send input to runtime thread
│  └─────────────────┘  │
└───────────────────────┘
         ▲
         │ TuiEvent channel
         │
┌────────┴──────────────┐
│    Runtime Thread      │
│                        │
│  ┌──────────────────┐  │
│  │  ChannelSender   │  │ ← implements StreamSink
│  │  + TurnProgress  │  │ ← implements TurnProgressReporter
│  │  + HookProgress  │  │ ← implements HookProgressReporter
│  │  + PermPrompter  │  │ ← implements PermissionPrompter
│  └──────────────────┘  │
│          │              │
│          ▼              │
│  ConversationRuntime    │
│  .run_turn(input, ...)  │
│                        │
└────────────────────────┘
```

### Implementation Sketch

```rust
// --- StreamSink: replaces io::Write in consume_stream() ---
trait StreamSink {
    fn text_delta(&mut self, text: &str);
    fn thinking_start(&mut self);
    fn thinking_delta(&mut self, text: &str);
    fn thinking_end(&mut self, char_count: usize, redacted: bool);
    fn tool_use(&mut self, id: &str, name: &str, input: &str);
    fn usage(&mut self, usage: &TokenUsage);
    fn message_stop(&mut self);
}

// Channel-based implementation
struct ChannelStreamSink {
    tx: mpsc::Sender<TuiEvent>,
}

impl StreamSink for ChannelStreamSink {
    fn text_delta(&mut self, text: &str) {
        let _ = self.tx.send(TuiEvent::TextDelta { text: text.into() });
    }
    // ... etc
}

// --- Modified consume_stream signature ---
// BEFORE:
async fn consume_stream(&self, req: &MessageRequest, stall: bool) 
    -> Result<Vec<AssistantEvent>, RuntimeError>

// AFTER (or alongside):
async fn consume_stream_with_sink(
    &self, 
    req: &MessageRequest, 
    stall: bool,
    sink: &mut dyn StreamSink,
) -> Result<Vec<AssistantEvent>, RuntimeError>
```

### Migration Strategy

1. **Phase 1 (Minimal TUI):** Use `run_turn_to()` with `Vec<u8>` buffer — no live streaming, but functional
2. **Phase 2 (Channel events):** Add `StreamSink` trait, implement `ChannelStreamSink`, add `consume_stream_with_sink()`
3. **Phase 3 (Full TUI):** TUI consumes `Receiver<TuiEvent>`, renders each event with appropriate widget

---

## 16. Key Risks & Open Questions

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `consume_stream()` is 200+ lines with deep coupling to `io::Write` | High — refactoring is invasive | Add parallel `consume_stream_with_sink()` rather than modifying existing |
| `run_turn()` is synchronous and blocks the runtime thread | Medium — TUI input can't be processed during turn | Accept this: runtime runs on a background thread, TUI sends input via channel |
| `PermissionPrompter` blocks during `decide()` | Medium — TUI must wait for user input | Implemented as channel round-trip (send event → receive decision) |
| Hook runner may deny tool calls silently (no TUI event) | Low — Denied tools appear as error results | `TurnProgressReporter.on_tool_result()` already receives `Err` for denials |
| Context window retry loop is inside `run_turn_to()` (CLI layer) | High — TUI must replicate or share this logic | Extract into runtime or shared utility |

### Open Questions

1. **Should the TUI own the `BuiltRuntime` or the `Cli` struct?**  
   Currently `Cli` holds state (session, model, permission_mode) and wraps `BuiltRuntime`. The TUI probably needs its own "TuiApp" struct with similar state management.

2. **How to handle the context-window retry loop?**  
   This is currently 80+ lines in `run_turn_to()`. The TUI needs the same logic. Options: extract to shared function, or move it into the runtime.

3. **Should `InternalPromptProgressReporter` be exposed for the TUI?**  
   It's CLI-internal (ultraplan mode). The TUI might want its own progress reporting.

4. **What about MCP/plugin lifecycle?**  
   `BuiltRuntime` manages plugin and MCP startup/shutdown. The TUI must handle the same lifecycle (currently `prepare_turn_runtime()` handles this).

5. **How does the TUI handle `/session`, `/compact`, `/model` etc.?**  
   These are all handled in the REPL loop in `main.rs` (thousands of lines of command dispatch). The TUI needs its own command handler that calls the same runtime methods.

6. **Token streaming granularity for the TUI render loop?**  
   The TUI render loop (e.g., ratatui) typically redraws at 60fps. TextDeltas may arrive much faster. Need to batch/merge deltas between frames.

---

*End of report. All source references point to `/mnt/data/git/claw-code` on branch `feat-tui`.*
