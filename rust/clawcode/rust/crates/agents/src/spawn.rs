use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use runtime::ConversationRuntime;

use crate::persist::{
    DEFAULT_AGENT_MAX_ITERATIONS, DEFAULT_AGENT_TIMEOUT_SECS,
};
use crate::runtime::{build_agent_runtime_inner, ProviderRuntimeClient, SubagentToolExecutor};
use crate::types::{AgentJob, AgentProgress, AgentStatus, SharedProgress, SubagentProgressEvent};

pub struct AgentHandle {
    pub agent_id: String,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    pub progress: SharedProgress,
    finished: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryAgain;

/// Reap the worker and drop its progress entry whenever the handle is dropped,
/// not just on the explicit `join` path. Without this, a `try_join`-only
/// consumer (the production `wait_for_agent`) leaks the progress entry for the
/// process lifetime, and a handle dropped after a timeout detaches the worker
/// thread instead of reaping it. The worker's provider calls are time-bounded
/// (api crate), so `join` always terminates.
impl Drop for AgentHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        remove_progress_entry(&self.progress, &self.agent_id);
    }
}

impl AgentHandle {
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn join(mut self) -> Result<String, String> {
        let timeout = Duration::from_secs(DEFAULT_AGENT_TIMEOUT_SECS);
        let rx = match self.rx.take() {
            Some(rx) => rx,
            None => return Ok(String::new()),
        };
        let result = match rx.recv_timeout(timeout) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => Err(e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err("agent timed out".to_string()),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("agent disconnected".to_string())
            }
        };
        self.finished.store(true, Ordering::SeqCst);
        remove_progress_entry(&self.progress, &self.agent_id);
        // Join unconditionally on every exit path. The worker's provider calls
        // are now time-bounded (api crate), so join() always terminates and a
        // timed-out or failed agent never leaks its OS thread.
        let _ = self.thread_handle.take().map(|h| h.join());
        result
    }

    pub fn try_join(&mut self) -> Result<Result<String, String>, TryAgain> {
        let rx = match self.rx.as_ref() {
            Some(rx) => rx,
            None => return Ok(Ok(String::new())),
        };
        match rx.try_recv() {
            Ok(result) => {
                self.finished.store(true, Ordering::SeqCst);
                // The worker sent its result as the final act before exiting;
                // reap it now so the thread never leaks.
                let _ = self.thread_handle.take().map(|h| h.join());
                Ok(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Err(TryAgain),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.finished.store(true, Ordering::SeqCst);
                let _ = self.thread_handle.take().map(|h| h.join());
                Ok(Err("agent disconnected".to_string()))
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }

    /// Signal the worker to stop at the next iteration boundary. The caller
    /// must then reap the thread (via `try_join`) to avoid running the agent
    /// to completion after it was told to stop.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    #[cfg(feature = "test-utils")]
    pub fn noop(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            thread_handle: None,
            rx: None,
            progress: crate::types::new_shared_progress(),
            finished: Arc::new(AtomicBool::new(true)),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(feature = "test-utils")]
    pub fn with_parts(
        agent_id: impl Into<String>,
        thread_handle: std::thread::JoinHandle<()>,
        rx: std::sync::mpsc::Receiver<Result<String, String>>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            thread_handle: Some(thread_handle),
            rx: Some(rx),
            progress: crate::types::new_shared_progress(),
            finished: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(feature = "test-utils")]
    pub fn join_with_timeout(mut self, timeout: Duration) -> Result<String, String> {
        let rx = match self.rx.take() {
            Some(rx) => rx,
            None => return Ok(String::new()),
        };
        let result = match rx.recv_timeout(timeout) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => Err(e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err("agent timed out".to_string()),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("agent disconnected".to_string())
            }
        };
        let _ = self.thread_handle.take().map(|h| h.join());
        result
    }
}

fn remove_progress_entry(shared: &SharedProgress, agent_id: &str) {
    let mut guard = shared.agents.lock().unwrap_or_else(|e| e.into_inner());
    guard.retain(|p| p.agent_id != agent_id);
}

/// Spawn an agent task on a dedicated OS thread so that the
/// `ProviderRuntimeClient::block_on()` call inside `run_agent_job`
/// does not panic with "Cannot start a runtime from within a runtime".
pub fn spawn_agent_task(job: AgentJob) -> Result<AgentHandle, String> {
    spawn_agent_task_with_progress(job, crate::types::new_shared_progress())
}

pub fn spawn_agent_task_with_progress(
    job: AgentJob,
    progress: SharedProgress,
) -> Result<AgentHandle, String> {
    let agent_id = job.manifest.agent_id.clone();
    let name = job.manifest.name.clone();
    let subagent_type = job.manifest.subagent_type.clone().unwrap_or_default();
    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = Arc::clone(&finished);
    let cancel = Arc::new(AtomicBool::new(false));

    {
        let mut guard = progress.agents.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(AgentProgress {
            agent_id: agent_id.clone(),
            name: name.clone(),
            subagent_type: subagent_type.clone(),
            status: AgentStatus::Running,
            events: vec![],
            started_at: std::time::Instant::now(),
            iteration_count: 0,
            final_event: None,
            current_activity: None,
        });
    }

    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

    let progress_for_job = Arc::clone(&progress);
    let agent_id_for_job = agent_id.clone();
    let cancel_for_job = Arc::clone(&cancel);
    let thread_handle = std::thread::spawn(move || {
        let job_progress = Arc::clone(&progress_for_job);
        let job_agent_id = agent_id_for_job.clone();
        let job_with_progress = AssertUnwindSafe(AgentJobWithProgress {
            job,
            progress: progress_for_job,
            agent_id: agent_id_for_job,
            cancel: cancel_for_job,
        });
        let result = std::panic::catch_unwind(move || {
            run_agent_job_sync_with_progress(&job_with_progress)
        });
        clear_current_activity(&job_progress, &job_agent_id);

        let outcome = match result {
            Ok(Ok(text)) => {
                push_progress_event(
                    &job_progress,
                    &job_agent_id,
                    SubagentProgressEvent::Completed {
                        result_preview: text.clone(),
                    },
                );
                push_progress_event(
                    &job_progress,
                    &job_agent_id,
                    SubagentProgressEvent::StatusChange {
                        status: AgentStatus::Completed,
                    },
                );
                Ok(text)
            }
            Ok(Err(error)) => {
                push_progress_event(
                    &job_progress,
                    &job_agent_id,
                    SubagentProgressEvent::Failed {
                        error: error.clone(),
                    },
                );
                Err(error)
            }
            Err(panic_payload) => {
                let panic_msg = panic_message(&panic_payload);
                push_progress_event(
                    &job_progress,
                    &job_agent_id,
                    SubagentProgressEvent::Failed {
                        error: format!("panic: {panic_msg}"),
                    },
                );
                Err(format!("panic: {panic_msg}"))
            }
        };
        finished_clone.store(true, Ordering::SeqCst);
        let _ = tx.send(outcome);
    });

    Ok(AgentHandle {
        agent_id,
        thread_handle: Some(thread_handle),
        rx: Some(rx),
        progress,
        finished,
        cancel,
    })
}

struct AgentJobWithProgress {
    job: AgentJob,
    progress: SharedProgress,
    agent_id: String,
    cancel: Arc<AtomicBool>,
}

fn push_progress_event(shared: &SharedProgress, agent_id: &str, event: SubagentProgressEvent) {
    crate::types::push_progress_event(shared, agent_id, event);
}

fn clear_current_activity(shared: &SharedProgress, agent_id: &str) {
    crate::types::set_current_activity(shared, agent_id, None);
}

fn run_agent_job_sync_with_progress(job: &AgentJobWithProgress) -> Result<String, String> {
    let mut runtime: ConversationRuntime<ProviderRuntimeClient, SubagentToolExecutor> =
        build_agent_runtime_inner(
            &job.job,
            Some(Arc::clone(&job.progress)),
            Some(job.agent_id.clone()),
        )?
        .with_max_iterations(DEFAULT_AGENT_MAX_ITERATIONS)
        .with_cancel_signal(Arc::clone(&job.cancel));
    let summary = runtime
        .run_turn(job.job.prompt.clone(), None)
        .map_err(|error| error.to_string())?;
    match final_assistant_text(&summary) {
        Some(text) => Ok(text),
        None => Err("agent returned no text".to_string()),
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        String::from("unknown panic payload")
    }
}

fn final_assistant_text(summary: &runtime::TurnSummary) -> Option<String> {
    // Walk messages newest-first so a thinking-only final turn does not
    // silently erase the agent's real answer from an earlier message.
    //
    // Messages that carry a `ToolUse` block are skipped as text candidates:
    // any text inside them is transitional narration emitted BEFORE the tool
    // call ("Let me check the file first"), not the sub-agent's answer. Only
    // tool-use-free messages can supply the final result.
    for message in summary.assistant_messages.iter().rev() {
        if message
            .blocks
            .iter()
            .any(|block| matches!(block, runtime::ContentBlock::ToolUse { .. }))
        {
            continue;
        }
        let text = message
            .blocks
            .iter()
            .filter_map(|block| match block {
                runtime::ContentBlock::Text { text } => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if !text.is_empty() {
            return Some(text);
        }
    }

    // No non-empty text block anywhere: surface the latest reasoning so the
    // parent model sees *something* instead of a silently empty result.
    for message in summary.assistant_messages.iter().rev() {
        for block in message.blocks.iter().rev() {
            if let runtime::ContentBlock::Thinking { thinking, .. } = block {
                let trimmed = thinking.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // Truly nothing to report. `None` propagates as an error to the parent so
    // a sub-agent that produced no output is never mistaken for a successful
    // delegation (the old code returned a `"(agent returned no text)"` marker
    // with `is_error=false`, silently swallowing the failure).
    None
}

#[cfg(test)]
mod tests {
    use runtime::{
        AutoCompactionEvent, ContentBlock, ConversationMessage, PromptCacheEvent, TokenUsage,
        TurnSummary,
    };

    use super::final_assistant_text;

    fn summary_with(messages: Vec<ConversationMessage>) -> TurnSummary {
        TurnSummary {
            assistant_messages: messages,
            tool_results: vec![],
            prompt_cache_events: vec![PromptCacheEvent {
                unexpected: false,
                reason: String::new(),
                previous_cache_read_input_tokens: 0,
                current_cache_read_input_tokens: 0,
                token_drop: 0,
            }],
            iterations: 1,
            usage: TokenUsage::default(),
            auto_compaction: Some(AutoCompactionEvent {
                removed_message_count: 0,
                savings_ratio: 0.0,
            }),
        }
    }

    fn text(s: &str) -> ContentBlock {
        ContentBlock::Text { text: s.to_string() }
    }

    fn thinking(s: &str) -> ContentBlock {
        ContentBlock::Thinking {
            thinking: s.to_string(),
            signature: Some("sig".to_string()),
        }
    }

    fn tool_use() -> ContentBlock {
        ContentBlock::ToolUse {
            id: "toolu_test_1".to_string(),
            name: "read_file".to_string(),
            input: serde_json::json!({}),
        }
    }

    fn msg(blocks: Vec<ContentBlock>) -> ConversationMessage {
        ConversationMessage::assistant(blocks)
    }

    #[test]
    fn returns_text_from_last_message() {
        let summary = summary_with(vec![msg(vec![text("hello")])]);
        assert_eq!(final_assistant_text(&summary), Some("hello".to_string()));
    }

    #[test]
    fn returns_last_non_empty_text_message_when_final_is_thinking_only() {
        let summary = summary_with(vec![
            msg(vec![text("earlier result")]),
            msg(vec![thinking("thinking only")]),
        ]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("earlier result".to_string())
        );
    }

    #[test]
    fn returns_thinking_text_when_no_text_blocks_exist() {
        let summary = summary_with(vec![msg(vec![thinking("deep reasoning")])]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("deep reasoning".to_string())
        );
    }

    #[test]
    fn returns_none_when_no_blocks_at_all() {
        let summary = summary_with(vec![]);
        assert_eq!(final_assistant_text(&summary), None);
    }

    #[test]
    fn ignores_empty_text_blocks_when_falling_back() {
        let summary = summary_with(vec![
            msg(vec![text("   ")]),
            msg(vec![text("real answer")]),
        ]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("real answer".to_string())
        );
    }

    #[test]
    fn does_not_return_transitional_text_from_tool_calling_message() {
        let summary = summary_with(vec![
            msg(vec![text("Let me check the file first"), tool_use()]),
            msg(vec![thinking("The real answer is 42")]),
        ]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("The real answer is 42".to_string())
        );
    }

    #[test]
    fn falls_back_to_last_text_only_message_when_tool_calling_message_is_newer() {
        let summary = summary_with(vec![
            msg(vec![text("actual result")]),
            msg(vec![text("Let me verify"), tool_use()]),
            msg(vec![thinking("final reasoning only")]),
        ]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("actual result".to_string())
        );
    }

    #[test]
    fn prefers_thinking_over_transitional_text_from_tool_calling_message() {
        let summary = summary_with(vec![
            msg(vec![text("Let me check the file first"), tool_use()]),
            msg(vec![thinking("the answer is deep reasoning")]),
        ]);
        assert_eq!(
            final_assistant_text(&summary),
            Some("the answer is deep reasoning".to_string())
        );
    }
}
