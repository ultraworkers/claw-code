use std::fmt;

#[derive(Debug)]
pub struct CommandContext {
    pub session_id: Option<String>,
}

#[derive(Debug)]
pub enum CommandOutcome {
    Ok,
    Message(String),
}

#[derive(Debug)]
pub enum CommandError {
    UnknownCommand(String),
    Handler(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(n) => write!(f, "unknown command: {n}"),
            Self::Handler(msg) => write!(f, "handler error: {msg}"),
        }
    }
}

impl std::error::Error for CommandError {}

pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }
    fn description(&self) -> &'static str;
    fn usage(&self) -> &'static str {
        ""
    }
    fn execute(&self, ctx: &CommandContext, args: &str) -> Result<CommandOutcome, CommandError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;
    impl CommandHandler for EchoHandler {
        fn name(&self) -> &'static str {
            "echo"
        }
        fn description(&self) -> &'static str {
            "echoes args"
        }
        fn execute(
            &self,
            _ctx: &CommandContext,
            args: &str,
        ) -> Result<CommandOutcome, CommandError> {
            Ok(CommandOutcome::Message(args.to_string()))
        }
    }

    #[test]
    fn trait_dispatch_via_dyn() {
        let h: Box<dyn CommandHandler> = Box::new(EchoHandler);
        let ctx = CommandContext { session_id: None };
        let outcome = h.execute(&ctx, "hello").expect("ok");
        match outcome {
            CommandOutcome::Message(s) => assert_eq!(s, "hello"),
            CommandOutcome::Ok => panic!("wrong outcome"),
        }
    }
}
