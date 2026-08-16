use std::collections::HashMap;

use crate::handler::{CommandContext, CommandError, CommandHandler, CommandOutcome};

#[derive(Debug)]
pub struct DuplicateCommand(pub String);

pub struct CommandRegistry {
    handlers: Vec<Box<dyn CommandHandler>>,
    by_name: HashMap<String, usize>,
}

impl CommandRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// Construct a registry pre-populated with one `BuiltinAdapter` per entry
    /// in the static `SLASH_COMMAND_SPECS` table. Duplicate-name registration
    /// is ignored defensively (the static table is the single source of truth
    /// and should not contain duplicates).
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        for spec in crate::slash_command_specs() {
            let _ = r.register(Box::new(BuiltinAdapter::new(spec)));
        }
        r
    }

    pub fn register(&mut self, h: Box<dyn CommandHandler>) -> Result<(), DuplicateCommand> {
        let key = h.name().to_string();
        if self.by_name.contains_key(&key) {
            return Err(DuplicateCommand(key));
        }
        let idx = self.handlers.len();
        self.handlers.push(h);
        self.by_name.insert(key, idx);
        Ok(())
    }

    pub fn dispatch(
        &self,
        line: &str,
        ctx: &CommandContext,
    ) -> Result<CommandOutcome, CommandError> {
        let trimmed = line.trim_start_matches('/');
        let (cmd, rest) = match trimmed.split_once(' ') {
            Some((c, r)) => (c, r),
            None => (trimmed, ""),
        };
        let idx = self
            .by_name
            .get(cmd)
            .copied()
            .ok_or_else(|| CommandError::UnknownCommand(cmd.to_string()))?;
        self.handlers[idx].execute(ctx, rest)
    }

    #[must_use]
    pub fn list(&self) -> Vec<&dyn CommandHandler> {
        self.handlers.iter().map(AsRef::as_ref).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter that wraps a single `SlashCommandSpec` from the static table and
/// exposes it as a `CommandHandler`. Phase 1: validates dispatchability via
/// `SlashCommand::from_name`; actual side-effect execution is deferred to
/// Phase 2 (requires a `Session` which is not yet in `CommandContext`).
pub struct BuiltinAdapter {
    spec: &'static crate::SlashCommandSpec,
}

impl BuiltinAdapter {
    #[must_use]
    pub fn new(spec: &'static crate::SlashCommandSpec) -> Self {
        Self { spec }
    }
}

impl CommandHandler for BuiltinAdapter {
    fn name(&self) -> &'static str {
        self.spec.name
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.spec.aliases
    }

    fn description(&self) -> &'static str {
        self.spec.summary
    }

    fn usage(&self) -> &'static str {
        self.spec.argument_hint.unwrap_or("")
    }

    fn execute(
        &self,
        _ctx: &CommandContext,
        _args: &str,
    ) -> Result<CommandOutcome, CommandError> {
        crate::SlashCommand::from_name(self.spec.name)
            .ok_or_else(|| CommandError::UnknownCommand(self.spec.name.to_string()))?;
        Ok(CommandOutcome::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::{CommandContext, CommandError, CommandHandler, CommandOutcome};

    struct FooHandler;
    impl CommandHandler for FooHandler {
        fn name(&self) -> &'static str {
            "foo"
        }
        fn aliases(&self) -> &'static [&'static str] {
            &["f"]
        }
        fn description(&self) -> &'static str {
            "foo command"
        }
        fn execute(
            &self,
            _ctx: &CommandContext,
            _args: &str,
        ) -> Result<CommandOutcome, CommandError> {
            Ok(CommandOutcome::Ok)
        }
    }

    struct BarHandler;
    impl CommandHandler for BarHandler {
        fn name(&self) -> &'static str {
            "bar"
        }
        fn description(&self) -> &'static str {
            "bar command"
        }
        fn execute(
            &self,
            _ctx: &CommandContext,
            _args: &str,
        ) -> Result<CommandOutcome, CommandError> {
            Ok(CommandOutcome::Ok)
        }
    }

    #[test]
    fn register_and_dispatch_by_name() {
        let mut r = CommandRegistry::new();
        r.register(Box::new(FooHandler)).unwrap();
        let ctx = CommandContext { session_id: None };
        let outcome = r.dispatch("/foo", &ctx).expect("ok");
        assert!(matches!(outcome, CommandOutcome::Ok));
    }

    #[test]
    fn dispatch_unknown_command_returns_error() {
        let r = CommandRegistry::new();
        let ctx = CommandContext { session_id: None };
        let err = r.dispatch("/missing", &ctx).expect_err("should fail");
        assert!(matches!(err, CommandError::UnknownCommand(n) if n == "missing"));
    }

    #[test]
    fn duplicate_name_is_rejected() {
        let mut r = CommandRegistry::new();
        r.register(Box::new(FooHandler)).unwrap();
        let result = r.register(Box::new(FooHandler));
        assert!(matches!(result, Err(DuplicateCommand(n)) if n == "foo"));
    }

    #[test]
    fn list_returns_all_handlers() {
        let mut r = CommandRegistry::new();
        r.register(Box::new(FooHandler)).unwrap();
        r.register(Box::new(BarHandler)).unwrap();
        let list = r.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name(), "foo");
        assert_eq!(list[1].name(), "bar");
    }

    #[test]
    fn with_builtins_populates_from_static_table() {
        let r = CommandRegistry::with_builtins();
        let list = r.list();
        assert!(
            !list.is_empty(),
            "with_builtins should populate from the static spec table"
        );
        let names: Vec<&str> = list.iter().map(|h| h.name()).collect();
        assert!(
            names.contains(&"help"),
            "with_builtins should include the help command, got: {names:?}"
        );
    }
}
