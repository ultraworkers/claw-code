//! Command palette — fuzzy-filterable modal for all available commands.

use crate::keybindings::Action;
use commands::slash_command_specs;

pub struct CommandPalette {
    pub active: bool,
    pub query: String,
    pub entries: Vec<PaletteEntry>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub label: String,
    pub description: String,
    pub action: Action,
    pub key_hint: String,
    pub category: String,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut entries = vec![
            PaletteEntry {
                label: "Submit".into(),
                description: "Send message".into(),
                action: Action::Submit,
                key_hint: "Enter".into(),
                category: "Input".into(),
            },
            PaletteEntry {
                label: "Swap Provider".into(),
                description: "Change AI provider".into(),
                action: Action::ProviderSwap,
                key_hint: "Ctrl+P".into(),
                category: "Settings".into(),
            },
            PaletteEntry {
                label: "Toggle Team".into(),
                description: "Show/hide team info".into(),
                action: Action::TeamToggle,
                key_hint: "Ctrl+T".into(),
                category: "View".into(),
            },
            PaletteEntry {
                label: "Agent View".into(),
                description: "Multi-session dashboard".into(),
                action: Action::ToggleAgentView,
                key_hint: "Ctrl+A".into(),
                category: "View".into(),
            },
            PaletteEntry {
                label: "Toggle Sidebar".into(),
                description: "Show/hide file sidebar".into(),
                action: Action::ToggleSidebar,
                key_hint: "Ctrl+B".into(),
                category: "View".into(),
            },
            PaletteEntry {
                label: "Clear Conversation".into(),
                description: "Clear all messages".into(),
                action: Action::ClearConversation,
                key_hint: "Ctrl+L".into(),
                category: "Session".into(),
            },
            PaletteEntry {
                label: "Help".into(),
                description: "Keyboard shortcuts".into(),
                action: Action::Help,
                key_hint: "F1".into(),
                category: "Help".into(),
            },
            PaletteEntry {
                label: "Exit".into(),
                description: "Exit TUI mode".into(),
                action: Action::Exit,
                key_hint: "Ctrl+D".into(),
                category: "Session".into(),
            },
            PaletteEntry {
                label: "Cycle Chat Mode".into(),
                description: "Code → Ask → Architect".into(),
                action: Action::CycleChatMode,
                key_hint: "Tab".into(),
                category: "Modes".into(),
            },
        ];

        // Add every registered slash command so Ctrl+K can search /theme,
        // /permissions, /model, etc. The index is stable because the spec
        // slice is built once at first access.
        for (i, spec) in slash_command_specs().iter().enumerate() {
            let label = format!("/{}", spec.name);
            let aliases = spec.aliases.join(", ");
            let description = if aliases.is_empty() {
                spec.summary.to_string()
            } else {
                format!("{} (aliases: {})", spec.summary, aliases)
            };
            entries.push(PaletteEntry {
                label,
                description,
                action: Action::RunSlashCommand(i),
                key_hint: "slash".into(),
                category: "Slash".into(),
            });
        }

        let filtered = (0..entries.len()).collect();
        Self {
            active: false,
            query: String::new(),
            entries,
            filtered,
            selected: 0,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.filtered = (0..self.entries.len()).collect();
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
    }

    pub fn input(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = if self.selected == 0 {
                self.filtered.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_action(&self) -> Option<Action> {
        self.filtered
            .get(self.selected)
            .map(|&i| self.entries[i].action)
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.label.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q)
                        || e.category.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_opens_and_closes() {
        let mut cp = CommandPalette::new();
        assert!(!cp.active);
        cp.open();
        assert!(cp.active);
        assert_eq!(cp.selected, 0);
        cp.close();
        assert!(!cp.active);
    }

    #[test]
    fn test_filter_by_label() {
        let mut cp = CommandPalette::new();
        cp.open();
        // 'ZZZZ' doesn't appear in any default entry label/description/category,
        // so it should filter down to zero results.
        for _ in 0..4 {
            cp.input('Z');
        }
        assert!(cp.filtered.is_empty());
        // A more specific query that matches only some entries
        for _ in 0..4 {
            cp.backspace();
        }
        cp.input('H');
        cp.input('e');
        cp.input('l');
        cp.input('p');
        assert!(cp.filtered.len() < cp.entries.len());
        assert!(cp.filtered.len() >= 1);
    }

    #[test]
    fn test_select_navigation() {
        let mut cp = CommandPalette::new();
        cp.open();
        let initial = cp.selected;
        cp.select_next();
        assert_ne!(cp.selected, initial);
    }

    #[test]
    fn test_selected_action() {
        let mut cp = CommandPalette::new();
        cp.open();
        assert!(cp.selected_action().is_some());
    }

    #[test]
    fn test_empty_query_shows_all() {
        let mut cp = CommandPalette::new();
        cp.open();
        cp.input('x');
        assert!(cp.filtered.len() < cp.entries.len());
        cp.backspace();
        assert_eq!(cp.filtered.len(), cp.entries.len());
    }
}
