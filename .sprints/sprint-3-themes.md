# Sprint 3: Theme System

> **Duration:** 3 days | **Stories:** 5 | **Goal:** Built-in themes with runtime switching, matching OpenCode's ecosystem
> **Depends on:** Sprint 0, Sprint 1, Sprint 2

---

## S3-1: Create theme module with TuiTheme struct

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Description
Define the `TuiTheme` struct with named color roles for every TUI element.

### Implementation

**New file:** `src/theme.rs`

```rust
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiTheme {
    pub name: String,
    // Conversation pane
    pub conversation_text: ColorDef,
    pub conversation_user: ColorDef,
    pub conversation_system: ColorDef,
    pub conversation_error: ColorDef,
    pub conversation_assistant: ColorDef,
    pub conversation_dim: ColorDef,
    // Code blocks
    pub code_fg: ColorDef,
    pub code_bg: ColorDef,
    pub code_border: ColorDef,
    pub code_language_label: ColorDef,
    // Dashboard
    pub dashboard_header: ColorDef,
    pub dashboard_key: ColorDef,
    pub dashboard_value: ColorDef,
    pub dashboard_separator: ColorDef,
    pub dashboard_keys_hint: ColorDef,
    // Context gauge
    pub gauge_fill_green: ColorDef,
    pub gauge_fill_yellow: ColorDef,
    pub gauge_fill_red: ColorDef,
    pub gauge_bg: ColorDef,
    pub gauge_label: ColorDef,
    // Input area
    pub input_fg: ColorDef,
    pub input_cursor_fg: ColorDef,
    pub input_cursor_bg: ColorDef,
    pub input_border: ColorDef,
    pub input_border_active: ColorDef,
    // UI chrome
    pub border: ColorDef,
    pub border_active: ColorDef,
    pub scrollbar: ColorDef,
    pub spinner: ColorDef,
    pub status_message: ColorDef,
    pub key_hint: ColorDef,
    // Completions
    pub completion_fg: ColorDef,
    pub completion_bg: ColorDef,
    pub completion_selected_fg: ColorDef,
    pub completion_selected_bg: ColorDef,
    // Agent View
    pub agent_running: ColorDef,
    pub agent_waiting: ColorDef,
    pub agent_done: ColorDef,
    pub agent_failed: ColorDef,
    pub agent_cancelled: ColorDef,
    // Syntax highlighting theme name
    pub syntax_theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ColorDef {
    Named(String),
    Ansi256(u8),
    Rgb { r: u8, g: u8, b: u8 },
}

impl ColorDef {
    pub fn to_color(&self) -> Color {
        match self {
            ColorDef::Named(name) => match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "red" => Color::Red,
                "green" => Color::Green,
                "yellow" => Color::Yellow,
                "blue" => Color::Blue,
                "magenta" | "purple" => Color::Magenta,
                "cyan" => Color::Cyan,
                "white" => Color::White,
                "gray" | "grey" | "darkgray" => Color::DarkGray,
                "lightgray" | "lightgrey" => Color::Gray,
                _ => Color::White,
            },
            ColorDef::Ansi256(n) => Color::Indexed(*n),
            ColorDef::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
        }
    }
}

impl TuiTheme {
    pub fn builtin(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::theme_default()),
            "tokyonight" => Some(Self::theme_tokyonight()),
            "catppuccin-mocha" => Some(Self::theme_catppuccin_mocha()),
            "catppuccin-latte" => Some(Self::theme_catppuccin_latte()),
            "nord" => Some(Self::theme_nord()),
            "gruvbox" => Some(Self::theme_gruvbox()),
            "dracula" => Some(Self::theme_dracula()),
            "solarized-dark" => Some(Self::theme_solarized_dark()),
            "solarized-light" => Some(Self::theme_solarized_light()),
            "monokai" => Some(Self::theme_monokai()),
            "system" => Some(Self::theme_system()),
            _ => None,
        }
    }

    pub fn all_builtin_names() -> Vec<&'static str> {
        vec![
            "default", "tokyonight", "catppuccin-mocha", "catppuccin-latte",
            "nord", "gruvbox", "dracula", "solarized-dark", "solarized-light",
            "monokai", "system",
        ]
    }

    /// Deserialize from JSON with validation.
    /// Returns Err if required fields are missing or invalid.
    pub fn from_json(json: &str) -> Result<Self, ThemeError> {
        let theme: Self = serde_json::from_str(json)
            .map_err(|e| ThemeError::ParseError(e.to_string()))?;
        
        // Validate syntax theme exists in syntect (or warn)
        // We don't fail — just log a warning if theme is unknown
        
        Ok(theme)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Validate that the syntax_theme name exists in syntect's ThemeSet.
    /// Returns true if found, false if will fall back to default.
    pub fn validate_syntax_theme(&self) -> bool {
        use crate::markdown::THEME_SET;
        THEME_SET.themes.contains_key(&self.syntax_theme)
    }
}

#[derive(Debug)]
pub enum ThemeError {
    ParseError(String),
}
```

### Acceptance Criteria
- [ ] `TuiTheme` struct with all color roles (including Agent View colors)
- [ ] `ColorDef` enum with named/256/RGB
- [ ] `from_json()` validates without panicking
- [ ] `validate_syntax_theme()` checks syntect availability
- [ ] `builtin()` returns `Some` for all 11 names

---

## S3-2: Implement all 11 built-in themes

**Priority:** P1 — High  
**Estimate:** 1.5 days (up from 0.5 — Opus review noted this is design work, not just coding)

### Description
Define color palettes for all 11 themes. Each needs ~40 color values that look good together.

### Implementation

**File:** `src/theme.rs` — implement private methods

```rust
impl TuiTheme {
    fn theme_default() -> Self {
        Self {
            name: "default".into(),
            conversation_text: ColorDef::Named("white".into()),
            conversation_user: ColorDef::Named("cyan".into()),
            conversation_system: ColorDef::Named("yellow".into()),
            conversation_error: ColorDef::Named("red".into()),
            conversation_assistant: ColorDef::Named("white".into()),
            conversation_dim: ColorDef::Named("darkgray".into()),
            code_fg: ColorDef::Named("white".into()),
            code_bg: ColorDef::Named("black".into()),
            code_border: ColorDef::Named("darkgray".into()),
            code_language_label: ColorDef::Named("darkgray".into()),
            dashboard_header: ColorDef::Named("cyan".into()),
            dashboard_key: ColorDef::Named("darkgray".into()),
            dashboard_value: ColorDef::Named("white".into()),
            dashboard_separator: ColorDef::Named("darkgray".into()),
            dashboard_keys_hint: ColorDef::Named("darkgray".into()),
            gauge_fill_green: ColorDef::Named("green".into()),
            gauge_fill_yellow: ColorDef::Named("yellow".into()),
            gauge_fill_red: ColorDef::Named("red".into()),
            gauge_bg: ColorDef::Named("darkgray".into()),
            gauge_label: ColorDef::Named("white".into()),
            input_fg: ColorDef::Named("white".into()),
            input_cursor_fg: ColorDef::Named("black".into()),
            input_cursor_bg: ColorDef::Named("cyan".into()),
            input_border: ColorDef::Named("cyan".into()),
            input_border_active: ColorDef::Named("cyan".into()),
            border: ColorDef::Named("darkgray".into()),
            border_active: ColorDef::Named("cyan".into()),
            scrollbar: ColorDef::Named("darkgray".into()),
            spinner: ColorDef::Named("blue".into()),
            status_message: ColorDef::Named("blue".into()),
            key_hint: ColorDef::Named("darkgray".into()),
            completion_fg: ColorDef::Named("white".into()),
            completion_bg: ColorDef::Rgb { r: 40, g: 40, b: 40 },
            completion_selected_fg: ColorDef::Named("black".into()),
            completion_selected_bg: ColorDef::Named("cyan".into()),
            agent_running: ColorDef::Named("cyan".into()),
            agent_waiting: ColorDef::Named("yellow".into()),
            agent_done: ColorDef::Named("green".into()),
            agent_failed: ColorDef::Named("red".into()),
            agent_cancelled: ColorDef::Named("darkgray".into()),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_tokyonight() -> Self {
        Self {
            name: "tokyonight".into(),
            conversation_text: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            conversation_user: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            conversation_system: ColorDef::Rgb { r: 224, g: 175, b: 104 },
            conversation_error: ColorDef::Rgb { r: 247, g: 118, b: 142 },
            conversation_assistant: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            conversation_dim: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            code_fg: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            code_bg: ColorDef::Rgb { r: 26, g: 27, b: 48 },
            code_border: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            code_language_label: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            dashboard_header: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            dashboard_key: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            dashboard_value: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            dashboard_separator: ColorDef::Rgb { r: 59, g: 66, b: 97 },
            dashboard_keys_hint: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            gauge_fill_green: ColorDef::Rgb { r: 158, g: 206, b: 106 },
            gauge_fill_yellow: ColorDef::Rgb { r: 224, g: 175, b: 104 },
            gauge_fill_red: ColorDef::Rgb { r: 247, g: 118, b: 142 },
            gauge_bg: ColorDef::Rgb { r: 59, g: 66, b: 97 },
            gauge_label: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            input_fg: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            input_cursor_fg: ColorDef::Rgb { r: 26, g: 27, b: 48 },
            input_cursor_bg: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            input_border: ColorDef::Rgb { r: 59, g: 66, b: 97 },
            input_border_active: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            border: ColorDef::Rgb { r: 59, g: 66, b: 97 },
            border_active: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            scrollbar: ColorDef::Rgb { r: 59, g: 66, b: 97 },
            spinner: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            status_message: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            key_hint: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            completion_fg: ColorDef::Rgb { r: 192, g: 202, b: 245 },
            completion_bg: ColorDef::Rgb { r: 35, g: 40, b: 65 },
            completion_selected_fg: ColorDef::Rgb { r: 26, g: 27, b: 48 },
            completion_selected_bg: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            agent_running: ColorDef::Rgb { r: 125, g: 207, b: 255 },
            agent_waiting: ColorDef::Rgb { r: 224, g: 175, b: 104 },
            agent_done: ColorDef::Rgb { r: 158, g: 206, b: 106 },
            agent_failed: ColorDef::Rgb { r: 247, g: 118, b: 142 },
            agent_cancelled: ColorDef::Rgb { r: 86, g: 95, b: 137 },
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    // catppuccin-mocha, catppuccin-latte, nord, gruvbox, dracula,
    // solarized-dark, solarized-light, monokai follow the same pattern
    // with their respective color palettes.

    fn theme_system() -> Self {
        // Uses only ANSI named colors — works on any terminal
        let mut theme = Self::theme_default();
        theme.name = "system".into();
        theme.syntax_theme = "InspiredGitHub".into();
        theme
    }
}
```

### Acceptance Criteria
- [ ] All 11 themes defined with distinct palettes
- [ ] Each theme has all ~40 color roles filled
- [ ] `system` theme uses only ANSI named colors
- [ ] All themes use syntect theme names that exist in defaults

---

## S3-3: Wire theme into TuiApp — NO HARDCODED COLORS

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Description
Replace ALL hardcoded `Color::X` references with `self.theme.field.to_color()`. This is critical — Sprint 6 and 7 must also use theme colors, not introduce new hardcoded values.

### Implementation

**File:** `src/tui.rs`

1. Add theme field to `TuiApp`:
```rust
pub struct TuiApp {
    theme: TuiTheme,
    markdown_renderer: MarkdownRenderer,
    // ...
}
```

2. Add helper to reduce boilerplate:
```rust
impl TuiApp {
    fn tc(&self, color: &ColorDef) -> Color {
        color.to_color()
    }
}
```

3. Replace every `Color::X` in rendering with theme lookup:
```rust
// BEFORE:
Style::default().fg(Color::Cyan)

// AFTER:
Style::default().fg(self.tc(&self.theme.dashboard_header))
```

4. Files to update:
   - `draw_frame()` → borders, chrome
   - `draw_left_pane()` → conversation colors
   - `draw_right_pane()` → dashboard colors
   - `push_user_input()` → `self.theme.conversation_user`
   - `push_system_message()` → `self.theme.conversation_system`
   - `push_output()` → `self.theme.conversation_error` / `self.theme.conversation_assistant`
   - Input area setup → `self.theme.input_*`
   - Spinner → `self.theme.spinner`
   - Status → `self.theme.status_message`

### Acceptance Criteria
- [ ] `grep -r "Color::" src/tui.rs` returns zero matches in rendering code
- [ ] All colors come from `self.theme`
- [ ] Changing theme at runtime updates all colors on next frame
- [ ] Add `#[cfg(test)]` assertion that no hardcoded colors remain

---

## S3-4: Implement `/theme` slash command

**Priority:** P1 — High  
**Estimate:** 0.25 day  

### Implementation

```rust
// In src/tui_commands.rs:
TuiCommand::Theme(args) => {
    if args.is_empty() {
        let names = TuiTheme::all_builtin_names();
        app.push_system_message(&format!(
            "Available themes: {}\nUsage: /theme <name>",
            names.join(", ")
        ));
    } else if let Some(theme) = TuiTheme::builtin(args) {
        app.set_theme(theme);
        app.push_system_message(&format!("Theme: {args}"));
    } else {
        app.push_system_message(&format!("Unknown theme: {args}"));
    }
}
```

### Acceptance Criteria
- [ ] `/theme` lists all 11 themes
- [ ] `/theme tokyonight` switches immediately
- [ ] Theme persists for the session

---

## S3-5: Add theme persistence and custom JSON loading

**Priority:** P2 — Medium  
**Estimate:** 0.5 day  

### Implementation

1. Save selected theme name to runtime config
2. Load on TUI startup
3. Custom theme loading: `/theme --custom <path>`
4. **Validation:** reject JSON with missing required fields or invalid color values

```rust
TuiCommand::Theme(args) if args.starts_with("--custom ") => {
    let path = args.strip_prefix("--custom ").unwrap().trim();
    match std::fs::read_to_string(path) {
        Ok(content) => match TuiTheme::from_json(&content) {
            Ok(theme) => {
                if !theme.validate_syntax_theme() {
                    app.push_system_message(&format!(
                        "Warning: syntax theme '{}' not found, using fallback",
                        theme.syntax_theme
                    ));
                }
                app.set_theme(theme);
                app.push_system_message("Custom theme loaded");
            }
            Err(e) => {
                app.push_system_message(&format!("Invalid theme: {e:?}"));
            }
        },
        Err(e) => {
            app.push_system_message(&format!("Cannot read {path}: {e}"));
        }
    }
}
```

### Acceptance Criteria
- [ ] Selected theme persists in config
- [ ] Custom JSON themes load correctly
- [ ] Malformed JSON shows error (doesn't panic)
- [ ] Invalid syntax theme name shows warning but loads

---

## Sprint 3 Definition of Done

- [ ] All 5 stories completed
- [ ] 11 built-in themes implemented
- [ ] `/theme` command works at runtime
- [ ] Zero hardcoded `Color::` in rendering code
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] Manual test: switch between 3 themes, verify all colors change
