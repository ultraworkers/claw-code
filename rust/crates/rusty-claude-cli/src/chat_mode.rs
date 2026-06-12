//! Chat mode — controls AI behavior (code/ask/architect).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMode {
    /// Default — full access, edits files and runs commands.
    Code,
    /// Discussion only — no file changes.
    Ask,
    /// Plan first, then implement after approval.
    Architect,
}

impl ChatMode {
    pub fn label(&self) -> &'static str {
        match self {
            ChatMode::Code => "code",
            ChatMode::Ask => "ask",
            ChatMode::Architect => "arch",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ChatMode::Code => "Full access — edits and runs code",
            ChatMode::Ask => "Discussion only — no changes made",
            ChatMode::Architect => "Plan first, then implement",
        }
    }

    /// Appended to system prompt to instruct the model about mode constraints.
    pub fn system_prompt_suffix(&self) -> &'static str {
        match self {
            ChatMode::Code => "",
            ChatMode::Ask => "\n\nIMPORTANT: Do NOT modify any files. Only discuss and explain.",
            ChatMode::Architect => {
                "\n\nIMPORTANT: First create a plan. After the user approves, implement it."
            }
        }
    }

    /// Cycle to the next mode.
    pub fn next(self) -> Self {
        match self {
            ChatMode::Code => ChatMode::Ask,
            ChatMode::Ask => ChatMode::Architect,
            ChatMode::Architect => ChatMode::Code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle() {
        assert_eq!(ChatMode::Code.next(), ChatMode::Ask);
        assert_eq!(ChatMode::Ask.next(), ChatMode::Architect);
        assert_eq!(ChatMode::Architect.next(), ChatMode::Code);
    }

    #[test]
    fn test_labels() {
        assert_eq!(ChatMode::Code.label(), "code");
        assert_eq!(ChatMode::Ask.label(), "ask");
        assert_eq!(ChatMode::Architect.label(), "arch");
    }

    #[test]
    fn test_code_mode_has_no_suffix() {
        assert_eq!(ChatMode::Code.system_prompt_suffix(), "");
    }

    #[test]
    fn test_ask_mode_restrains() {
        assert!(ChatMode::Ask.system_prompt_suffix().contains("Do NOT modify"));
    }

    #[test]
    fn test_architect_mode_plans_first() {
        assert!(ChatMode::Architect.system_prompt_suffix().contains("plan"));
    }
}
