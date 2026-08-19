use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use serde_json::{Map, Value};
use telemetry::SessionTracer;

use crate::compact::{
    compact_session, estimate_session_tokens, CompactionConfig, CompactionResult,
};
use crate::config::RuntimeFeatureConfig;
use crate::hooks::{HookAbortSignal, HookProgressReporter, HookRunResult, HookRunner};
use crate::permissions::{
    PermissionContext, PermissionOutcome, PermissionPolicy, PermissionPrompter,
};
use crate::session::{ContentBlock, ConversationMessage, Session};
use crate::usage::{TokenUsage, UsageTracker};

const DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD: u32 = 100_000;
const AUTO_COMPACTION_THRESHOLD_ENV_VAR: &str = "CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS";

const DEFAULT_MAX_STALL_NUDGES: usize = 3;
const MAX_STALL_NUDGES_ENV_VAR: &str = "CLAW_MAX_STALL_NUDGES";
/// Placeholder recorded in the trace when a turn is resumed rather than started, so the trace
/// does not attribute a second user message to a turn that only had one.
const RESUMED_TURN_MARKER: &str = "[resumed turn]";
/// Operator-declared context window. Kept as a literal rather than imported because `runtime`
/// sits below `api` in the dependency graph; `api::context_window_override` reads the same name.
const CONTEXT_WINDOW_ENV_VAR: &str = "CLAW_CONTEXT_WINDOW";
/// Share of a declared context window at which compaction is triggered, leaving the remainder
/// as headroom for the reply plus whatever the next tool result carries.
const AUTO_COMPACTION_WINDOW_PERCENT: u32 = 70;

/// Injected when the model narrates its next step (or asks for a go-ahead mid-task)
/// instead of emitting the tool call that would perform it.
const STALL_NUDGE_PROMPT: &str = "[auto-continue] You ended the turn without calling a tool, \
     but the work is not finished. Do not describe the next step and do not ask for \
     confirmation — issue the tool call now and keep going until the task is done. If \
     everything asked for is genuinely complete, reply with a one-line summary containing \
     no forward-looking sentence.";

/// Phrases that mark text as an announcement of work the model has not performed yet.
/// Matched against the final non-empty line of the assistant's reply.
const PENDING_ACTION_MARKERS: [&str; 16] = [
    "let me ",
    "let's ",
    "lets ",
    "i'll ",
    "i will ",
    "i'm going to",
    "i am going to",
    "going to run",
    "about to ",
    "now running",
    "now checking",
    "now i",
    "next i",
    "next, i",
    "proceeding to",
    "moving on to",
];

/// Fully assembled request payload sent to the upstream model client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRequest {
    pub system_prompt: Vec<String>,
    pub messages: Vec<ConversationMessage>,
}

/// Streamed events emitted while processing a single assistant turn.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Prompt-cache telemetry captured from the provider response stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCacheEvent {
    pub unexpected: bool,
    pub reason: String,
    pub previous_cache_read_input_tokens: u32,
    pub current_cache_read_input_tokens: u32,
    pub token_drop: u32,
}

/// Minimal streaming API contract required by [`ConversationRuntime`].
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

/// Trait implemented by tool dispatchers that execute model-requested tools.
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

/// Error returned when a tool invocation fails locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    message: String,
}

impl ToolError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

/// Error returned when a conversation turn cannot be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
    context_window: bool,
}

impl RuntimeError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context_window: false,
        }
    }

    /// A failure the caller may be able to clear by shrinking the session.
    ///
    /// This is a decision the API layer makes and hands up, not one the caller reconstructs by
    /// matching substrings against the rendered message. Recovering from context exhaustion means
    /// discarding conversation history, so a transport hiccup misread as an overflow destroys the
    /// session for no reason — the classification has to come from whoever saw the real error.
    #[must_use]
    pub fn context_window(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context_window: true,
        }
    }

    #[must_use]
    pub const fn is_context_window_failure(&self) -> bool {
        self.context_window
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Summary of one completed runtime turn, including tool results and usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSummary {
    pub assistant_messages: Vec<ConversationMessage>,
    pub tool_results: Vec<ConversationMessage>,
    pub prompt_cache_events: Vec<PromptCacheEvent>,
    pub iterations: usize,
    pub usage: TokenUsage,
    pub auto_compaction: Option<AutoCompactionEvent>,
}

/// Details about automatic session compaction applied during a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoCompactionEvent {
    pub removed_message_count: usize,
}

/// Coordinates the model loop, tool execution, hooks, and session updates.
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
    max_stall_nudges: usize,
}

impl<C, T> ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    #[must_use]
    pub fn new(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
    ) -> Self {
        Self::new_with_features(
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            &RuntimeFeatureConfig::default(),
        )
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new_with_features(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
        feature_config: &RuntimeFeatureConfig,
    ) -> Self {
        let usage_tracker = UsageTracker::from_session(&session);
        Self {
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            max_iterations: usize::MAX,
            usage_tracker,
            hook_runner: HookRunner::from_feature_config(feature_config),
            auto_compaction_input_tokens_threshold: auto_compaction_threshold_from_env(),
            hook_abort_signal: HookAbortSignal::default(),
            hook_progress_reporter: None,
            session_tracer: None,
            max_stall_nudges: max_stall_nudges_from_env(),
        }
    }

    /// Override how many auto-continue nudges a stalled turn may receive. `0` disables them.
    #[must_use]
    pub fn with_max_stall_nudges(mut self, max_stall_nudges: usize) -> Self {
        self.max_stall_nudges = max_stall_nudges;
        self
    }

    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    #[must_use]
    pub fn with_auto_compaction_input_tokens_threshold(mut self, threshold: u32) -> Self {
        self.auto_compaction_input_tokens_threshold = threshold;
        self
    }

    /// Update the auto-compaction threshold after construction. This allows the
    /// caller to tune the threshold based on runtime information (e.g., the
    /// server-returned context window size from a 400 error).
    pub fn set_auto_compaction_input_tokens_threshold(&mut self, threshold: u32) {
        self.auto_compaction_input_tokens_threshold = threshold;
    }

    #[must_use]
    pub fn with_hook_abort_signal(mut self, hook_abort_signal: HookAbortSignal) -> Self {
        self.hook_abort_signal = hook_abort_signal;
        self
    }

    #[must_use]
    pub fn with_hook_progress_reporter(
        mut self,
        hook_progress_reporter: Box<dyn HookProgressReporter>,
    ) -> Self {
        self.hook_progress_reporter = Some(hook_progress_reporter);
        self
    }

    #[must_use]
    pub fn with_session_tracer(mut self, session_tracer: SessionTracer) -> Self {
        self.session_tracer = Some(session_tracer);
        self
    }

    fn run_pre_tool_use_hook(&mut self, tool_name: &str, input: &str) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    fn run_post_tool_use_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
        is_error: bool,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    fn run_post_tool_use_failure_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    /// Run a session health probe to verify the runtime is functional after compaction.
    /// Returns Ok(()) if healthy, Err if the session appears broken.
    fn run_session_health_probe(&mut self) -> Result<(), String> {
        // Check if we have basic session integrity
        if self.session.messages.is_empty() && self.session.compaction.is_some() {
            // Freshly compacted with no messages - this is normal
            return Ok(());
        }

        // Verify tool executor is responsive with a non-destructive probe
        // Using glob_search with a pattern that won't match anything
        let probe_input = r#"{"pattern": "*.health-check-probe-"}"#;
        match self.tool_executor.execute("glob_search", probe_input) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Tool executor probe failed: {e}")),
        }
    }

    pub fn run_turn(
        &mut self,
        user_input: impl Into<String>,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        self.drive_turn(Some(user_input.into()), prompter)
    }

    /// Continues the turn already recorded in the session, without adding a user message.
    ///
    /// Recovery paths retry a turn that failed part-way through. Calling [`Self::run_turn`] again
    /// with the original input would append that input a second time and invite the model to redo
    /// tool calls whose side effects have already landed. Resuming reads the transcript as it
    /// stands — including the tool results that did complete — and carries on from there.
    pub fn resume_turn(
        &mut self,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        self.drive_turn(None, prompter)
    }

    #[allow(clippy::too_many_lines)]
    fn drive_turn(
        &mut self,
        user_input: Option<String>,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        // ROADMAP #38: Session-health canary - probe if context was compacted
        if self.session.compaction.is_some() {
            if let Err(error) = self.run_session_health_probe() {
                return Err(RuntimeError::new(format!(
                    "Session health probe failed after compaction: {error}. \
                     The session may be in an inconsistent state. \
                     Consider starting a fresh session with /session new."
                )));
            }
        }

        match user_input {
            Some(user_input) => {
                self.record_turn_started(&user_input);
                self.session
                    .push_user_text(user_input)
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
            }
            None => self.record_turn_started(RESUMED_TURN_MARKER),
        }

        let mut assistant_messages = Vec::new();
        let mut tool_results = Vec::new();
        let mut prompt_cache_events = Vec::new();
        let mut iterations = 0;
        let mut auto_compaction = None;
        let max_stall_nudges = self.max_stall_nudges;
        let mut stall_nudges = 0;
        let mut tools_ran = false;

        loop {
            iterations += 1;
            if iterations > self.max_iterations {
                let error = RuntimeError::new(
                    "conversation loop exceeded the maximum number of iterations",
                );
                self.record_turn_failed(iterations, &error);
                return Err(error);
            }

            // Compact BEFORE the request goes out, not only after a reply lands.
            //
            // The usage-based check below is post-hoc: it can only see the context once a call
            // has returned and reported its counters. Between that measurement and the next
            // request the session can grow enormously — a reasoning model emitting a long
            // <think> block, then a tool result carrying a large file. Observed live: a call
            // measured at 17,858 prompt tokens generated 10,842 more, a read_file added ~19k on
            // top, and the next request hit the provider at 47,755 tokens and was rejected
            // outright. No amount of tuning the post-hoc threshold prevents that, because
            // nothing measures the session in between.
            //
            // `estimate_session_tokens` is a local heuristic over the transcript, so this costs
            // no API call and works before any usage has ever been recorded.
            if let Some(compaction) = self.maybe_auto_compact_before_request() {
                auto_compaction = Some(compaction);
            }

            let request = ApiRequest {
                system_prompt: self.system_prompt.clone(),
                messages: self.session.messages.clone(),
            };
            let events = match self.api_client.stream(request) {
                Ok(events) => events,
                Err(error) => {
                    self.record_turn_failed(iterations, &error);
                    return Err(error);
                }
            };
            let (assistant_message, usage, turn_prompt_cache_events) =
                match build_assistant_message(events) {
                    Ok(result) => result,
                    Err(error) => {
                        let error = self.classify_empty_reply(error);
                        self.record_turn_failed(iterations, &error);
                        return Err(error);
                    }
                };
            if let Some(usage) = usage {
                self.usage_tracker.record(usage);
            }
            prompt_cache_events.extend(turn_prompt_cache_events);
            let pending_tool_uses = assistant_message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.record_assistant_iteration(
                iterations,
                &assistant_message,
                pending_tool_uses.len(),
            );

            self.session
                .push_message(assistant_message.clone())
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            assistant_messages.push(assistant_message);

            // Run auto-compaction check before next API call, including on the terminal
            // (no-tool) iteration, to prevent unbounded session growth (#3106).
            if let Some(compaction) = self.maybe_auto_compact() {
                auto_compaction = Some(compaction);
            }

            if pending_tool_uses.is_empty() {
                // A reply with no tool call usually means the turn is done. On small local
                // models it just as often means the model narrated the next step — or asked
                // for a go-ahead mid-task — and then emitted EOS, leaving the announced work
                // undone. Nudge it back into the loop instead of dropping the user at the
                // prompt, bounded so a model that insists it is finished still gets to stop.
                let stalled = stall_nudges < max_stall_nudges
                    && is_stalled_reply(
                        &assistant_text(
                            assistant_messages
                                .last()
                                .expect("assistant message pushed above"),
                        ),
                        tools_ran,
                    );
                if stalled {
                    stall_nudges += 1;
                    // Pushed as a system turn, not a user one. The model needs to see it, but the
                    // transcript must not claim the user asked for it: this text is persisted,
                    // re-sent on every later request, and folded into the next compaction summary,
                    // where a user-labelled nudge becomes indistinguishable from something the
                    // user actually said.
                    self.session
                        .push_message(ConversationMessage::system_text(STALL_NUDGE_PROMPT))
                        .map_err(|error| RuntimeError::new(error.to_string()))?;
                    continue;
                }
                break;
            }
            tools_ran = true;

            for (tool_use_id, tool_name, input) in pending_tool_uses {
                let pre_hook_result = self.run_pre_tool_use_hook(&tool_name, &input);
                let effective_input = pre_hook_result
                    .updated_input()
                    .map_or_else(|| input.clone(), ToOwned::to_owned);
                let permission_context = PermissionContext::new(
                    pre_hook_result.permission_override(),
                    pre_hook_result.permission_reason().map(ToOwned::to_owned),
                );

                let permission_outcome = if pre_hook_result.is_cancelled() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook cancelled tool `{tool_name}`"),
                        ),
                    }
                } else if pre_hook_result.is_failed() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook failed for tool `{tool_name}`"),
                        ),
                    }
                } else if pre_hook_result.is_denied() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook denied tool `{tool_name}`"),
                        ),
                    }
                } else if let Some(prompt) = prompter.as_mut() {
                    self.permission_policy.authorize_with_context(
                        &tool_name,
                        &effective_input,
                        &permission_context,
                        Some(*prompt),
                    )
                } else {
                    self.permission_policy.authorize_with_context(
                        &tool_name,
                        &effective_input,
                        &permission_context,
                        None,
                    )
                };

                let result_message = match permission_outcome {
                    PermissionOutcome::Allow => {
                        self.record_tool_started(iterations, &tool_name);
                        let (mut output, mut is_error) =
                            match self.tool_executor.execute(&tool_name, &effective_input) {
                                Ok(output) => (output, false),
                                Err(error) => (error.to_string(), true),
                            };
                        output = merge_hook_feedback(pre_hook_result.messages(), output, false);

                        let post_hook_result = if is_error {
                            self.run_post_tool_use_failure_hook(
                                &tool_name,
                                &effective_input,
                                &output,
                            )
                        } else {
                            self.run_post_tool_use_hook(
                                &tool_name,
                                &effective_input,
                                &output,
                                false,
                            )
                        };
                        if post_hook_result.is_denied()
                            || post_hook_result.is_failed()
                            || post_hook_result.is_cancelled()
                        {
                            is_error = true;
                        }
                        output = merge_hook_feedback(
                            post_hook_result.messages(),
                            output,
                            post_hook_result.is_denied()
                                || post_hook_result.is_failed()
                                || post_hook_result.is_cancelled(),
                        );

                        ConversationMessage::tool_result(tool_use_id, tool_name, output, is_error)
                    }
                    PermissionOutcome::Deny { reason } => ConversationMessage::tool_result(
                        tool_use_id,
                        tool_name,
                        merge_hook_feedback(pre_hook_result.messages(), reason, true),
                        true,
                    ),
                };
                self.session
                    .push_message(result_message.clone())
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                self.record_tool_finished(iterations, &result_message);
                tool_results.push(result_message);
            }
        }

        let summary = TurnSummary {
            assistant_messages,
            tool_results,
            prompt_cache_events,
            iterations,
            usage: self.usage_tracker.cumulative_usage(),
            auto_compaction,
        };
        self.record_turn_completed(&summary);

        Ok(summary)
    }

    #[must_use]
    pub fn compact(&self, config: CompactionConfig) -> CompactionResult {
        compact_session(&self.session, config)
    }

    #[must_use]
    pub fn estimated_tokens(&self) -> usize {
        estimate_session_tokens(&self.session)
    }

    #[must_use]
    pub fn usage(&self) -> &UsageTracker {
        &self.usage_tracker
    }

    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn api_client_mut(&mut self) -> &mut C {
        &mut self.api_client
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    #[must_use]
    pub fn fork_session(&self, branch_name: Option<String>) -> Session {
        self.session.fork(branch_name)
    }

    #[must_use]
    pub fn into_session(self) -> Session {
        self.session
    }

    /// Compaction check run immediately before a request is sent, based on a locally estimated
    /// transcript size rather than on provider-reported usage.
    ///
    /// This is the guard that actually keeps a session inside the context window: it sees growth
    /// that usage counters cannot, because it runs after tool results and long generations have
    /// been appended but before the request that would carry them is built.
    /// Decides whether an empty or unreadable reply was an overflow, by size rather than by text.
    ///
    /// A stream that produces no content says nothing about why. Some backends answer an
    /// oversized request that way instead of with a typed error, so the failure is worth
    /// recovering from — but only when the session was already large enough for that to be the
    /// plausible cause. Below the compaction threshold it is a blip, and discarding the
    /// transcript would cost more than the retry could win.
    fn classify_empty_reply(&self, error: RuntimeError) -> RuntimeError {
        if error.is_context_window_failure() {
            return error;
        }
        if estimate_session_tokens(&self.session)
            >= self.auto_compaction_input_tokens_threshold as usize
        {
            return RuntimeError::context_window(error.to_string());
        }
        error
    }

    fn maybe_auto_compact_before_request(&mut self) -> Option<AutoCompactionEvent> {
        let estimated = estimate_session_tokens(&self.session);
        if estimated < self.auto_compaction_input_tokens_threshold as usize {
            return None;
        }
        self.compact_now()
    }

    fn maybe_auto_compact(&mut self) -> Option<AutoCompactionEvent> {
        // Gate on the LIVE context size — the most recent call's input counters — not on a
        // cumulative running total, and count cached input as the context it is.
        //
        // The previous gate (`cumulative_usage().input_tokens`) was wrong twice over:
        //   1. It ignored `cache_read_input_tokens`. With prompt caching a warm call reports
        //      `input_tokens: 2`, so the counter crawled and compaction never fired however
        //      large the context grew.
        //   2. Summing across calls is not a context measurement, and the sum never decreased,
        //      so once it did trip, every later iteration compacted forever.
        // Reading the latest turn fixes both: it rises with the real context and falls back
        // below the threshold as soon as a compaction has done its job.
        if self.usage_tracker.current_turn_usage().total_input_tokens()
            < self.auto_compaction_input_tokens_threshold
        {
            return None;
        }

        self.compact_now()
    }

    /// Compacts the session unconditionally, returning `None` when nothing actually changed.
    fn compact_now(&mut self) -> Option<AutoCompactionEvent> {
        let result = compact_session(
            &self.session,
            CompactionConfig {
                max_estimated_tokens: 0,
                ..CompactionConfig::default()
            },
        );

        // `removed_message_count` alone is the wrong test for "did anything happen": compaction
        // also trims oversized tool-result payloads in messages it must PRESERVE, and on a
        // session too short to have anything droppable that trimming is the entire result.
        // Gating on the count would compute the smaller session and then throw it away.
        let shrank = result.compacted_session.messages != self.session.messages;
        if result.removed_message_count == 0 && !shrank {
            return None;
        }

        self.session = result.compacted_session;
        Some(AutoCompactionEvent {
            removed_message_count: result.removed_message_count,
        })
    }

    fn record_turn_started(&self, user_input: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert(
            "user_input".to_string(),
            Value::String(user_input.to_string()),
        );
        session_tracer.record("turn_started", attributes);
    }

    fn record_assistant_iteration(
        &self,
        iteration: usize,
        assistant_message: &ConversationMessage,
        pending_tool_use_count: usize,
    ) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert(
            "assistant_blocks".to_string(),
            Value::from(assistant_message.blocks.len() as u64),
        );
        attributes.insert(
            "pending_tool_use_count".to_string(),
            Value::from(pending_tool_use_count as u64),
        );
        session_tracer.record("assistant_iteration_completed", attributes);
    }

    fn record_tool_started(&self, iteration: usize, tool_name: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert(
            "tool_name".to_string(),
            Value::String(tool_name.to_string()),
        );
        session_tracer.record("tool_execution_started", attributes);
    }

    fn record_tool_finished(&self, iteration: usize, result_message: &ConversationMessage) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let Some(ContentBlock::ToolResult {
            tool_name,
            is_error,
            ..
        }) = result_message.blocks.first()
        else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("tool_name".to_string(), Value::String(tool_name.clone()));
        attributes.insert("is_error".to_string(), Value::Bool(*is_error));
        session_tracer.record("tool_execution_finished", attributes);
    }

    fn record_turn_completed(&self, summary: &TurnSummary) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert(
            "iterations".to_string(),
            Value::from(summary.iterations as u64),
        );
        attributes.insert(
            "assistant_messages".to_string(),
            Value::from(summary.assistant_messages.len() as u64),
        );
        attributes.insert(
            "tool_results".to_string(),
            Value::from(summary.tool_results.len() as u64),
        );
        attributes.insert(
            "prompt_cache_events".to_string(),
            Value::from(summary.prompt_cache_events.len() as u64),
        );
        session_tracer.record("turn_completed", attributes);
    }

    fn record_turn_failed(&self, iteration: usize, error: &RuntimeError) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("error".to_string(), Value::String(error.to_string()));
        session_tracer.record("turn_failed", attributes);
    }
}

/// Reads the automatic compaction threshold from the environment.
#[must_use]
pub fn auto_compaction_threshold_from_env() -> u32 {
    resolve_auto_compaction_threshold(
        std::env::var(AUTO_COMPACTION_THRESHOLD_ENV_VAR)
            .ok()
            .as_deref(),
        std::env::var(CONTEXT_WINDOW_ENV_VAR).ok().as_deref(),
    )
}

/// Resolves the threshold from an explicit setting, else from the declared context window.
///
/// `DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD` is a 200k-window number. Against a smaller
/// server it sits above the window entirely, so the pre-request guard can never fire and the
/// first sign of trouble is the backend rejecting the request. Deriving from the declared window
/// keeps the guard meaningful without every caller having to compute a second number by hand.
#[must_use]
fn resolve_auto_compaction_threshold(explicit: Option<&str>, context_window: Option<&str>) -> u32 {
    if let Some(threshold) = parse_positive_u32(explicit) {
        return threshold;
    }
    parse_positive_u32(context_window).map_or(
        DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD,
        |window| {
            (u64::from(window) * u64::from(AUTO_COMPACTION_WINDOW_PERCENT) / 100)
                .try_into()
                .unwrap_or(DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD)
        },
    )
}

#[must_use]
fn parse_positive_u32(value: Option<&str>) -> Option<u32> {
    value
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|parsed| *parsed > 0)
}

/// How many auto-continue nudges a single turn may inject. `0` disables the behaviour.
#[must_use]
pub fn max_stall_nudges_from_env() -> usize {
    parse_max_stall_nudges(std::env::var(MAX_STALL_NUDGES_ENV_VAR).ok().as_deref())
}

#[must_use]
fn parse_max_stall_nudges(value: Option<&str>) -> usize {
    value
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_STALL_NUDGES)
}

/// Concatenated text of an assistant message, ignoring thinking and tool blocks.
#[must_use]
fn assistant_text(message: &ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// True when a tool-less reply is a stall rather than a finished turn.
///
/// Small local models routinely narrate the next tool call in prose and then emit EOS, which
/// ends the turn with the announced work never performed. `after_tool_use` additionally treats
/// a mid-task question as a stall: once tools have run in this turn, stopping to ask for a
/// go-ahead is the same failure wearing a different hat.
#[must_use]
fn is_stalled_reply(text: &str, after_tool_use: bool) -> bool {
    let Some(last_line) = text.lines().map(str::trim).rfind(|line| !line.is_empty()) else {
        return false;
    };
    let lowered = last_line.to_lowercase();

    if after_tool_use && last_line.ends_with('?') {
        return true;
    }
    PENDING_ACTION_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn build_assistant_message(
    events: Vec<AssistantEvent>,
) -> Result<
    (
        ConversationMessage,
        Option<TokenUsage>,
        Vec<PromptCacheEvent>,
    ),
    RuntimeError,
> {
    let mut text = String::new();
    let mut blocks = Vec::new();
    let mut prompt_cache_events = Vec::new();
    let mut finished = false;
    let mut usage = None;

    for event in events {
        match event {
            AssistantEvent::Thinking {
                thinking,
                signature,
            } => {
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::Thinking {
                    thinking,
                    signature,
                });
            }
            AssistantEvent::TextDelta(delta) => text.push_str(&delta),
            AssistantEvent::ToolUse { id, name, input } => {
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::ToolUse { id, name, input });
            }
            AssistantEvent::Usage(value) => usage = Some(value),
            AssistantEvent::PromptCache(event) => prompt_cache_events.push(event),
            AssistantEvent::MessageStop => {
                finished = true;
            }
        }
    }

    flush_text_block(&mut text, &mut blocks);

    if !finished {
        return Err(RuntimeError::new(
            "assistant stream ended without a message stop event",
        ));
    }
    if blocks.is_empty() {
        return Err(RuntimeError::new("assistant stream produced no content"));
    }

    Ok((
        ConversationMessage::assistant_with_usage(blocks, usage),
        usage,
        prompt_cache_events,
    ))
}

fn flush_text_block(text: &mut String, blocks: &mut Vec<ContentBlock>) {
    if !text.is_empty() {
        blocks.push(ContentBlock::Text {
            text: std::mem::take(text),
        });
    }
}

fn format_hook_message(result: &HookRunResult, fallback: &str) -> String {
    if result.messages().is_empty() {
        fallback.to_string()
    } else {
        result.messages().join("\n")
    }
}

fn merge_hook_feedback(messages: &[String], output: String, is_error: bool) -> String {
    if messages.is_empty() {
        return output;
    }

    let mut sections = Vec::new();
    if !output.trim().is_empty() {
        sections.push(output);
    }
    let label = if is_error {
        "Hook feedback (error)"
    } else {
        "Hook feedback"
    };
    sections.push(format!("{label}:\n{}", messages.join("\n")));
    sections.join("\n\n")
}

type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError>>;

/// Simple in-memory tool executor for tests and lightweight integrations.
#[derive(Default)]
pub struct StaticToolExecutor {
    handlers: BTreeMap<String, ToolHandler>,
}

impl StaticToolExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn register(
        mut self,
        tool_name: impl Into<String>,
        handler: impl FnMut(&str) -> Result<String, ToolError> + 'static,
    ) -> Self {
        self.handlers.insert(tool_name.into(), Box::new(handler));
        self
    }
}

impl ToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        self.handlers
            .get_mut(tool_name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {tool_name}")))?(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assistant_text, build_assistant_message, is_stalled_reply, parse_max_stall_nudges,
        resolve_auto_compaction_threshold, ApiClient, ApiRequest, AssistantEvent,
        AutoCompactionEvent, ConversationRuntime, PromptCacheEvent, RuntimeError,
        StaticToolExecutor, ToolExecutor, DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD,
        DEFAULT_MAX_STALL_NUDGES,
    };
    use crate::compact::{estimate_session_tokens, CompactionConfig};
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};
    use crate::permissions::{
        PermissionMode, PermissionPolicy, PermissionPromptDecision, PermissionPrompter,
        PermissionRequest,
    };
    use crate::prompt::{ProjectContext, SystemPromptBuilder};
    use crate::session::{ContentBlock, MessageRole, Session};
    use crate::usage::TokenUsage;
    use crate::ToolError;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use telemetry::{MemoryTelemetrySink, SessionTracer, TelemetryEvent};

    struct ScriptedApiClient {
        call_count: usize,
    }

    impl ApiClient for ScriptedApiClient {
        fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
            self.call_count += 1;
            match self.call_count {
                1 => {
                    assert!(request
                        .messages
                        .iter()
                        .any(|message| message.role == MessageRole::User));
                    Ok(vec![
                        AssistantEvent::TextDelta("Let me calculate that.".to_string()),
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: "2,2".to_string(),
                        },
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 20,
                            output_tokens: 6,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 2,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                }
                2 => {
                    let last_message = request
                        .messages
                        .last()
                        .expect("tool result should be present");
                    assert_eq!(last_message.role, MessageRole::Tool);
                    Ok(vec![
                        AssistantEvent::TextDelta("The answer is 4.".to_string()),
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 24,
                            output_tokens: 4,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 3,
                        }),
                        AssistantEvent::PromptCache(PromptCacheEvent {
                            unexpected: true,
                            reason:
                                "cache read tokens dropped while prompt fingerprint remained stable"
                                    .to_string(),
                            previous_cache_read_input_tokens: 6_000,
                            current_cache_read_input_tokens: 1_000,
                            token_drop: 5_000,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                }
                _ => unreachable!("extra API call"),
            }
        }
    }

    struct PromptAllowOnce;

    impl PermissionPrompter for PromptAllowOnce {
        fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
            assert_eq!(request.tool_name, "add");
            PermissionPromptDecision::Allow
        }
    }

    #[test]
    fn runs_user_to_tool_to_result_loop_end_to_end_and_tracks_usage() {
        let api_client = ScriptedApiClient { call_count: 0 };
        let tool_executor = StaticToolExecutor::new().register("add", |input| {
            let total = input
                .split(',')
                .map(|part| part.parse::<i32>().expect("input must be valid integer"))
                .sum::<i32>();
            Ok(total.to_string())
        });
        let permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite);
        let system_prompt = SystemPromptBuilder::new()
            .with_project_context(ProjectContext {
                cwd: PathBuf::from("/tmp/project"),
                current_date: "2026-03-31".to_string(),
                git_status: None,
                git_diff: None,
                git_context: None,
                instruction_files: Vec::new(),
            })
            .with_os("linux", "6.8")
            .build();
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
        );

        let summary = runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        assert_eq!(summary.iterations, 2);
        assert_eq!(summary.assistant_messages.len(), 2);
        assert_eq!(summary.tool_results.len(), 1);
        assert_eq!(summary.prompt_cache_events.len(), 1);
        assert_eq!(runtime.session().messages.len(), 4);
        assert_eq!(summary.usage.output_tokens, 10);
        assert_eq!(summary.auto_compaction, None);
        assert!(matches!(
            runtime.session().messages[1].blocks[1],
            ContentBlock::ToolUse { .. }
        ));
        assert!(matches!(
            runtime.session().messages[2].blocks[0],
            ContentBlock::ToolResult {
                is_error: false,
                ..
            }
        ));
    }

    #[test]
    fn records_runtime_session_trace_events() {
        let sink = Arc::new(MemoryTelemetrySink::default());
        let tracer = SessionTracer::new("session-runtime", sink.clone());
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        )
        .with_session_tracer(tracer);

        runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        let events = sink.events();
        let trace_names = events
            .iter()
            .filter_map(|event| match event {
                TelemetryEvent::SessionTrace(trace) => Some(trace.name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(trace_names.contains(&"turn_started"));
        assert!(trace_names.contains(&"assistant_iteration_completed"));
        assert!(trace_names.contains(&"tool_execution_started"));
        assert!(trace_names.contains(&"tool_execution_finished"));
        assert!(trace_names.contains(&"turn_completed"));
    }

    #[test]
    fn records_denied_tool_results_when_prompt_rejects() {
        struct RejectPrompter;
        impl PermissionPrompter for RejectPrompter {
            fn decide(&mut self, _request: &PermissionRequest) -> PermissionPromptDecision {
                PermissionPromptDecision::Deny {
                    reason: "not now".to_string(),
                }
            }
        }

        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("I could not use the tool.".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: "secret".to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("use the tool", Some(&mut RejectPrompter))
            .expect("conversation should continue after denied tool");

        assert_eq!(summary.tool_results.len(), 1);
        assert!(matches!(
            &summary.tool_results[0].blocks[0],
            ContentBlock::ToolResult { is_error: true, output, .. } if output == "not now"
        ));
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_blocks() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("blocked".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: r#"{"path":"secret.txt"}"#.to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new().register("blocked", |_input| {
                panic!("tool should not execute when hook denies")
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'blocked by hook'; exit 2")],
                Vec::new(),
                Vec::new(),
            )),
        );

        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook denial");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "hook denial should produce an error result: {output}"
        );
        assert!(
            output.contains("denied tool") || output.contains("blocked by hook"),
            "unexpected hook denial output: {output:?}"
        );
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_fails() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("failed".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: r#"{"path":"secret.txt"}"#.to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new().register("blocked", |_input| {
                panic!("tool should not execute when hook fails")
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'broken hook'; exit 1")],
                Vec::new(),
                Vec::new(),
            )),
        );

        // when
        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook failure");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "hook failure should produce an error result: {output}"
        );
        assert!(
            output.contains("exited with status 1") || output.contains("broken hook"),
            "unexpected hook failure output: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: r#"{"lhs":2,"rhs":2}"#.to_string(),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(request
                            .messages
                            .iter()
                            .any(|message| message.role == MessageRole::Tool));
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    }
                    _ => unreachable!("extra API call"),
                }
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'pre hook ran'")],
                vec![shell_snippet("printf 'post hook ran'")],
                Vec::new(),
            )),
        );

        let summary = runtime
            .run_turn("use add", None)
            .expect("tool loop succeeds");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            !*is_error,
            "post hook should preserve non-error result: {output:?}"
        );
        assert!(
            output.contains('4'),
            "tool output missing value: {output:?}"
        );
        assert!(
            output.contains("pre hook ran"),
            "tool output missing pre hook feedback: {output:?}"
        );
        assert!(
            output.contains("post hook ran"),
            "tool output missing post hook feedback: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_use_failure_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "fail".to_string(),
                            input: r#"{"path":"README.md"}"#.to_string(),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(request
                            .messages
                            .iter()
                            .any(|message| message.role == MessageRole::Tool));
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    }
                    _ => unreachable!("extra API call"),
                }
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new()
                .register("fail", |_input| Err(ToolError::new("tool exploded"))),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                Vec::new(),
                vec![shell_snippet("printf 'post hook should not run'")],
                vec![shell_snippet("printf 'failure hook ran'")],
            )),
        );

        // when
        let summary = runtime
            .run_turn("use fail", None)
            .expect("tool loop succeeds");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "failure hook path should preserve error result: {output:?}"
        );
        assert!(
            output.contains("tool exploded"),
            "tool output missing failure reason: {output:?}"
        );
        assert!(
            output.contains("failure hook ran"),
            "tool output missing failure hook feedback: {output:?}"
        );
        assert!(
            !output.contains("post hook should not run"),
            "normal post hook should not run on tool failure: {output:?}"
        );
    }

    #[test]
    fn reconstructs_usage_tracker_from_restored_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session
            .messages
            .push(crate::session::ConversationMessage::assistant_with_usage(
                vec![ContentBlock::Text {
                    text: "earlier".to_string(),
                }],
                Some(TokenUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                }),
            ));

        let runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        assert_eq!(runtime.usage().turns(), 1);
        assert_eq!(runtime.usage().cumulative_usage().total_tokens(), 21);
    }

    #[test]
    fn compacts_session_after_turns() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );
        runtime.run_turn("a", None).expect("turn a");
        runtime.run_turn("b", None).expect("turn b");
        runtime.run_turn("c", None).expect("turn c");

        let result = runtime.compact(CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 1,
        });
        assert!(result.summary.contains("Conversation summary"));
        assert_eq!(
            result.compacted_session.messages[0].role,
            MessageRole::System
        );
        assert_eq!(
            result.compacted_session.session_id,
            runtime.session().session_id
        );
        assert!(result.compacted_session.compaction.is_some());
    }

    #[test]
    fn persists_conversation_turn_messages_to_jsonl_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let path = temp_session_path("persisted-turn");
        let session = Session::new().with_persistence_path(path.clone());
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime
            .run_turn("persist this turn", None)
            .expect("turn should succeed");

        let restored = Session::load_from_path(&path).expect("persisted session should reload");
        fs::remove_file(&path).expect("temp session file should be removable");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].role, MessageRole::User);
        assert_eq!(restored.messages[1].role, MessageRole::Assistant);
        assert_eq!(restored.session_id, runtime.session().session_id);
    }

    #[test]
    fn forks_runtime_session_without_mutating_original() {
        let mut session = Session::new();
        session
            .push_user_text("branch me")
            .expect("message should append");

        let runtime = ConversationRuntime::new(
            session.clone(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let forked = runtime.fork_session(Some("alt-path".to_string()));

        assert_eq!(forked.messages, session.messages);
        assert_ne!(forked.session_id, session.session_id);
        assert_eq!(
            forked
                .fork
                .as_ref()
                .map(|fork| (fork.parent_session_id.as_str(), fork.branch_name.as_deref())),
            Some((session.session_id.as_str(), Some("alt-path")))
        );
        assert!(runtime.session().fork.is_none());
    }

    fn temp_session_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-conversation-{label}-{nanos}.json"))
    }

    #[cfg(windows)]
    fn shell_snippet(script: &str) -> String {
        script.replace('\'', "\"")
    }

    #[cfg(not(windows))]
    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }

    #[test]
    fn auto_compacts_when_cumulative_input_threshold_is_crossed() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 120_000,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text("one"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            crate::session::ConversationMessage::user_text("three"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");

        assert_eq!(
            summary.auto_compaction,
            Some(AutoCompactionEvent {
                removed_message_count: 2,
            })
        );
        assert_eq!(runtime.session().messages[0].role, MessageRole::System);
    }

    #[test]
    fn skips_auto_compaction_below_threshold() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 99_999,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    /// Prompt caching reports the bulk of the context as `cache_read_input_tokens` and leaves
    /// `input_tokens` at a token or two, so a gate that reads `input_tokens` alone never fires
    /// on a cached session no matter how large the context has grown.
    #[test]
    fn auto_compaction_counts_cached_context_toward_threshold() {
        struct CachedApi;
        impl ApiClient for CachedApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 2,
                        output_tokens: 4,
                        cache_creation_input_tokens: 1_804,
                        cache_read_input_tokens: 112_639,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text("one"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            crate::session::ConversationMessage::user_text("three"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            CachedApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");

        assert_eq!(
            summary.auto_compaction,
            Some(AutoCompactionEvent {
                removed_message_count: 2,
            })
        );
    }

    /// The threshold describes the LIVE context, not a monotonic running total. Once compaction
    /// has shrunk the context back down, later small turns must not keep compacting.
    #[test]
    fn auto_compaction_stops_once_the_context_shrinks_again() {
        struct ShrinkingApi {
            calls: u32,
        }
        impl ApiClient for ShrinkingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                let call = self.calls;
                self.calls += 1;
                // The first call is over budget; every call after it is small.
                let input_tokens = if call == 0 { 150_000 } else { 900 };
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text("one"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            crate::session::ConversationMessage::user_text("three"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            ShrinkingApi { calls: 0 },
            // Every turn after a compaction runs the session health probe, which calls
            // `glob_search`; without it the probe fails the turn before the assertion is reached.
            StaticToolExecutor::new().register("glob_search", |_| Ok(String::new())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let first = runtime.run_turn("trigger", None).expect("first turn");
        assert!(
            first.auto_compaction.is_some(),
            "the over-budget turn should compact"
        );

        let second = runtime.run_turn("again", None).expect("second turn");
        assert_eq!(
            second.auto_compaction, None,
            "a small turn after compaction must not compact again"
        );
        let third = runtime.run_turn("and again", None).expect("third turn");
        assert_eq!(
            third.auto_compaction, None,
            "compaction must not latch on once the threshold has been crossed"
        );
    }

    /// Usage-based compaction is post-hoc: it only sees the context AFTER a call returns. A
    /// large tool result (or a long reasoning block) appended after that measurement can blow
    /// the window on the very next request, which the provider rejects outright — so there must
    /// also be a check BEFORE the request goes out, based on a locally estimated session size.
    #[test]
    fn auto_compaction_fires_before_the_first_request_when_the_session_is_already_oversized() {
        struct RecordingApi {
            largest_request_messages: std::rc::Rc<std::cell::Cell<usize>>,
        }
        impl ApiClient for RecordingApi {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.largest_request_messages.set(
                    self.largest_request_messages
                        .get()
                        .max(request.messages.len()),
                );
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    // Deliberately no Usage event: nothing has been measured yet, which is
                    // exactly the situation the pre-send check has to cover.
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // Six messages that are individually large — the shape of a session that has just
        // absorbed a couple of big file reads.
        let bulk = "x".repeat(40_000);
        let mut session = Session::new();
        for _ in 0..3 {
            session
                .messages
                .push(crate::session::ConversationMessage::user_text(bulk.clone()));
            session
                .messages
                .push(crate::session::ConversationMessage::assistant(vec![
                    ContentBlock::Text { text: bulk.clone() },
                ]));
        }
        let before = estimate_session_tokens(&session);
        assert!(
            before > 24_000,
            "test fixture must start over the threshold, got {before}"
        );

        let largest = std::rc::Rc::new(std::cell::Cell::new(0usize));
        let mut runtime = ConversationRuntime::new(
            session,
            RecordingApi {
                largest_request_messages: std::rc::Rc::clone(&largest),
            },
            StaticToolExecutor::new().register("glob_search", |_| Ok(String::new())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(24_000);

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");

        assert!(
            summary.auto_compaction.is_some(),
            "an already-oversized session must compact before the request is sent"
        );
        assert!(
            estimate_session_tokens(runtime.session()) < before,
            "compaction must actually shrink the session"
        );
        assert!(
            largest.get() < 8,
            "the request must be sent with the compacted history, not the original 7 messages"
        );
    }

    #[test]
    fn auto_compaction_threshold_defaults_and_parses_values() {
        assert_eq!(
            resolve_auto_compaction_threshold(None, None),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(resolve_auto_compaction_threshold(Some("4321"), None), 4321);
        assert_eq!(
            resolve_auto_compaction_threshold(Some("0"), None),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(
            resolve_auto_compaction_threshold(Some("not-a-number"), None),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
    }

    #[test]
    fn auto_compaction_threshold_derives_from_a_declared_context_window() {
        // 70% of 65536, so the guard sits inside the window instead of 34k above it.
        assert_eq!(
            resolve_auto_compaction_threshold(None, Some("65536")),
            45875
        );
    }

    #[test]
    fn an_explicit_threshold_still_wins_over_the_declared_window() {
        assert_eq!(
            resolve_auto_compaction_threshold(Some("48000"), Some("65536")),
            48000
        );
    }

    #[test]
    fn an_unparseable_context_window_falls_back_to_the_default_threshold() {
        assert_eq!(
            resolve_auto_compaction_threshold(None, Some("plenty")),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
    }

    #[test]
    fn compaction_health_probe_blocks_turn_when_tool_executor_is_broken() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                panic!("API should not run when health probe fails");
            }
        }

        let mut session = Session::new();
        session.record_compaction("summarized earlier work", 4);
        session
            .push_user_text("previous message")
            .expect("message should append");

        let tool_executor = StaticToolExecutor::new().register("glob_search", |_input| {
            Err(ToolError::new("transport unavailable"))
        });
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let error = runtime
            .run_turn("trigger", None)
            .expect_err("health probe failure should abort the turn");
        assert!(
            error
                .to_string()
                .contains("Session health probe failed after compaction"),
            "unexpected error: {error}"
        );
        assert!(
            error.to_string().contains("transport unavailable"),
            "expected underlying probe error: {error}"
        );
    }

    #[test]
    fn compaction_health_probe_skips_empty_compacted_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.record_compaction("fresh summary", 2);

        let tool_executor = StaticToolExecutor::new().register("glob_search", |_input| {
            Err(ToolError::new(
                "glob_search should not run for an empty compacted session",
            ))
        });
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("trigger", None)
            .expect("empty compacted session should not fail health probe");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    #[test]
    fn build_assistant_message_requires_message_stop_event() {
        // given
        let events = vec![AssistantEvent::TextDelta("hello".to_string())];

        // when
        let error = build_assistant_message(events)
            .expect_err("assistant messages should require a stop event");

        // then
        assert!(error
            .to_string()
            .contains("assistant stream ended without a message stop event"));
    }

    #[test]
    fn build_assistant_message_requires_content() {
        // given
        let events = vec![AssistantEvent::MessageStop];

        // when
        let error =
            build_assistant_message(events).expect_err("assistant messages should require content");

        // then
        assert!(error
            .to_string()
            .contains("assistant stream produced no content"));
    }

    #[test]
    fn build_assistant_message_places_thinking_block_before_text_and_tool_use() {
        // given
        let events = vec![
            AssistantEvent::Thinking {
                thinking: "pondering".to_string(),
                signature: Some("sig".to_string()),
            },
            AssistantEvent::TextDelta("hello".to_string()),
            AssistantEvent::ToolUse {
                id: "tool-1".to_string(),
                name: "echo".to_string(),
                input: "payload".to_string(),
            },
            AssistantEvent::MessageStop,
        ];

        // when
        let (message, _, _) = build_assistant_message(events)
            .expect("assistant message should preserve thinking, text, and tool blocks");

        // then
        assert_eq!(
            message.blocks,
            vec![
                ContentBlock::Thinking {
                    thinking: "pondering".to_string(),
                    signature: Some("sig".to_string()),
                },
                ContentBlock::Text {
                    text: "hello".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "echo".to_string(),
                    input: "payload".to_string(),
                },
            ]
        );
    }

    #[test]
    fn static_tool_executor_rejects_unknown_tools() {
        // given
        let mut executor = StaticToolExecutor::new();

        // when
        let error = executor
            .execute("missing", "{}")
            .expect_err("unregistered tools should fail");

        // then
        assert_eq!(error.to_string(), "unknown tool: missing");
    }

    /// Replays a fixed script of stream outcomes, one per call.
    struct ScriptedApi {
        steps: Vec<Result<Vec<AssistantEvent>, String>>,
        calls: usize,
    }

    impl ScriptedApi {
        fn new(steps: Vec<Result<Vec<AssistantEvent>, String>>) -> Self {
            Self { steps, calls: 0 }
        }
    }

    impl ApiClient for ScriptedApi {
        fn stream(&mut self, _request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
            let step = self
                .steps
                .get(self.calls)
                .cloned()
                .unwrap_or_else(|| Err("script exhausted".to_string()));
            self.calls += 1;
            step.map_err(RuntimeError::new)
        }
    }

    fn tool_call_events(id: &str) -> Vec<AssistantEvent> {
        vec![
            AssistantEvent::ToolUse {
                id: id.to_string(),
                name: "echo".to_string(),
                input: "payload".to_string(),
            },
            AssistantEvent::MessageStop,
        ]
    }

    fn scripted_runtime(
        steps: Vec<Result<Vec<AssistantEvent>, String>>,
    ) -> ConversationRuntime<ScriptedApi, StaticToolExecutor> {
        ConversationRuntime::new(
            Session::new(),
            ScriptedApi::new(steps),
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
    }

    #[test]
    fn a_failed_turn_keeps_the_results_of_tools_that_already_ran() {
        // given: a turn whose first iteration runs a tool, and whose second call dies
        let mut runtime = scripted_runtime(vec![
            Ok(tool_call_events("tool-1")),
            Err("upstream failed".to_string()),
        ]);

        // when
        runtime
            .run_turn("do the thing", None)
            .expect_err("the second call fails");

        // then: the executed tool's result is still in the transcript. The side effect happened;
        // a caller that discards this record will let the model run the same tool again.
        let tool_results = runtime
            .session()
            .messages
            .iter()
            .filter(|message| {
                message
                    .blocks
                    .iter()
                    .any(|block| matches!(block, ContentBlock::ToolResult { .. }))
            })
            .count();
        assert_eq!(tool_results, 1);
    }

    #[test]
    fn resume_turn_continues_without_appending_another_user_message() {
        // given: a session that already carries a user turn and a completed tool result
        let mut runtime = scripted_runtime(vec![
            Ok(tool_call_events("tool-1")),
            Err("upstream failed".to_string()),
            Ok(vec![
                AssistantEvent::TextDelta("done".to_string()),
                AssistantEvent::MessageStop,
            ]),
        ]);
        runtime.run_turn("do the thing", None).expect_err("fails");
        let user_messages_before = runtime
            .session()
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::User)
            .count();

        // when
        runtime
            .resume_turn(None)
            .expect("the resumed turn succeeds");

        // then: the prompt was not repeated
        let user_messages_after = runtime
            .session()
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::User)
            .count();
        assert_eq!(user_messages_before, user_messages_after);
    }

    #[test]
    fn a_stall_nudge_is_recorded_as_a_system_turn_not_a_user_one() {
        // given: a reply that announces work and stops, so the nudge fires once
        let mut runtime = nudge_runtime(
            vec![
                text_only("Let me check the logs."),
                text_only("The logs were clean."),
            ],
            1,
        );
        runtime
            .run_turn("check the logs", None)
            .expect("the nudged turn completes");

        // then: exactly one user message — the real one
        let user_messages = runtime
            .session()
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::User)
            .count();
        assert_eq!(user_messages, 1);
        assert!(runtime
            .session()
            .messages
            .iter()
            .any(|message| message.role == MessageRole::System
                && assistant_text(message).contains("[auto-continue]")));
    }

    #[test]
    fn run_turn_errors_when_max_iterations_is_exceeded() {
        struct LoopingApi;

        impl ApiClient for LoopingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "echo".to_string(),
                        input: "payload".to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            LoopingApi,
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_max_iterations(1);

        // when
        let error = runtime
            .run_turn("loop", None)
            .expect_err("conversation loop should stop after the configured limit");

        // then
        assert!(error
            .to_string()
            .contains("conversation loop exceeded the maximum number of iterations"));
    }

    #[test]
    fn run_turn_propagates_api_errors() {
        struct FailingApi;

        impl ApiClient for FailingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Err(RuntimeError::new("upstream failed"))
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            FailingApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        // when
        let error = runtime
            .run_turn("hello", None)
            .expect_err("API failures should propagate");

        // then
        assert_eq!(error.to_string(), "upstream failed");
    }

    fn text_only(text: &str) -> Vec<AssistantEvent> {
        vec![
            AssistantEvent::TextDelta(text.to_string()),
            AssistantEvent::MessageStop,
        ]
    }

    struct CountingApi {
        calls: usize,
        replies: Vec<Vec<AssistantEvent>>,
    }

    impl ApiClient for CountingApi {
        fn stream(&mut self, _request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
            let reply = self
                .replies
                .get(self.calls)
                .cloned()
                .unwrap_or_else(|| text_only("Now running the next probe:"));
            self.calls += 1;
            Ok(reply)
        }
    }

    fn nudge_runtime(
        replies: Vec<Vec<AssistantEvent>>,
        max_stall_nudges: usize,
    ) -> ConversationRuntime<CountingApi, StaticToolExecutor> {
        ConversationRuntime::new(
            Session::new(),
            CountingApi { calls: 0, replies },
            StaticToolExecutor::new().register("probe", |_| Ok("probe output".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_max_stall_nudges(max_stall_nudges)
    }

    #[test]
    fn announced_but_uncalled_tool_is_auto_continued() {
        // given: the model narrates the next probe instead of calling it
        let mut runtime = nudge_runtime(
            vec![
                text_only("Now running the Ford EEC-V tuner probe (ATSP1):"),
                vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "probe".to_string(),
                        input: "atsp1".to_string(),
                    },
                    AssistantEvent::MessageStop,
                ],
                text_only("No response on ATSP1, which is expected here."),
            ],
            3,
        );

        // when
        let summary = runtime.run_turn("run the probes", None).expect("turn runs");

        // then: the nudge pulled it back into the loop and the tool actually ran
        assert_eq!(summary.iterations, 3);
        assert_eq!(summary.tool_results.len(), 1);
    }

    #[test]
    fn nudges_are_bounded_so_a_stuck_model_still_stops() {
        // given: every reply is an announcement and nothing is ever called
        let mut runtime = nudge_runtime(Vec::new(), 2);

        // when
        let summary = runtime.run_turn("run the probes", None).expect("turn runs");

        // then: initial reply plus exactly two nudged retries
        assert_eq!(summary.iterations, 3);
        assert!(summary.tool_results.is_empty());
    }

    #[test]
    fn a_plain_completion_is_not_nudged() {
        // given
        let mut runtime = nudge_runtime(vec![text_only("The DTC scan came back clean.")], 3);

        // when
        let summary = runtime.run_turn("scan for codes", None).expect("turn runs");

        // then
        assert_eq!(summary.iterations, 1);
    }

    #[test]
    fn zero_budget_disables_auto_continue() {
        // given
        let mut runtime = nudge_runtime(vec![text_only("Let me run the DTC scan.")], 0);

        // when
        let summary = runtime.run_turn("scan for codes", None).expect("turn runs");

        // then
        assert_eq!(summary.iterations, 1);
    }

    #[test]
    fn a_question_stalls_only_after_a_tool_has_run() {
        assert!(is_stalled_reply("Should I run the DTC scan next?", true));
        assert!(!is_stalled_reply("Which port is the adapter on?", false));
    }

    #[test]
    fn stall_detection_reads_the_last_line_not_the_whole_reply() {
        // an announcement earlier in the reply does not make a finished turn look stalled
        assert!(!is_stalled_reply(
            "Let me check the port.\nThe scan finished with no codes stored.",
            true
        ));
        assert!(is_stalled_reply(
            "The scan finished.\nNext I'll read the freeze-frame data.",
            true
        ));
    }

    #[test]
    fn stall_nudge_budget_parses_from_env_value() {
        // asserted against the literal, not the constant: comparing the parser to the same
        // constant it falls back to would pass no matter what the default became
        assert_eq!(DEFAULT_MAX_STALL_NUDGES, 3);
        assert_eq!(parse_max_stall_nudges(None), 3);
        assert_eq!(parse_max_stall_nudges(Some("not a number")), 3);
        assert_eq!(parse_max_stall_nudges(Some(" 5 ")), 5);
        assert_eq!(parse_max_stall_nudges(Some("0")), 0);
    }
}
