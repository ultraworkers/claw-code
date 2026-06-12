//! Theme system for TUI — 11 built-in themes + custom JSON loading.

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
    // Syntax highlighting
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

fn named(s: &str) -> ColorDef { ColorDef::Named(s.to_string()) }
fn rgb(r: u8, g: u8, b: u8) -> ColorDef { ColorDef::Rgb { r, g, b } }

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

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // Built-in themes
    // -----------------------------------------------------------------------

    fn theme_default() -> Self {
        Self {
            name: "default".into(),
            conversation_text: named("white"),
            conversation_user: named("cyan"),
            conversation_system: named("yellow"),
            conversation_error: named("red"),
            conversation_assistant: named("white"),
            conversation_dim: named("darkgray"),
            code_fg: named("white"),
            code_bg: named("black"),
            code_border: named("darkgray"),
            code_language_label: named("darkgray"),
            dashboard_header: named("cyan"),
            dashboard_key: named("darkgray"),
            dashboard_value: named("white"),
            dashboard_separator: named("darkgray"),
            dashboard_keys_hint: named("darkgray"),
            gauge_fill_green: named("green"),
            gauge_fill_yellow: named("yellow"),
            gauge_fill_red: named("red"),
            gauge_bg: named("darkgray"),
            gauge_label: named("white"),
            input_fg: named("white"),
            input_cursor_fg: named("black"),
            input_cursor_bg: named("cyan"),
            input_border: named("cyan"),
            input_border_active: named("cyan"),
            border: named("darkgray"),
            border_active: named("cyan"),
            scrollbar: named("darkgray"),
            spinner: named("blue"),
            status_message: named("blue"),
            key_hint: named("darkgray"),
            completion_fg: named("white"),
            completion_bg: rgb(40, 40, 40),
            completion_selected_fg: named("black"),
            completion_selected_bg: named("cyan"),
            agent_running: named("cyan"),
            agent_waiting: named("yellow"),
            agent_done: named("green"),
            agent_failed: named("red"),
            agent_cancelled: named("darkgray"),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_tokyonight() -> Self {
        Self {
            name: "tokyonight".into(),
            conversation_text: rgb(192, 202, 245),
            conversation_user: rgb(125, 207, 255),
            conversation_system: rgb(224, 175, 104),
            conversation_error: rgb(247, 118, 142),
            conversation_assistant: rgb(192, 202, 245),
            conversation_dim: rgb(86, 95, 137),
            code_fg: rgb(192, 202, 245),
            code_bg: rgb(26, 27, 48),
            code_border: rgb(86, 95, 137),
            code_language_label: rgb(86, 95, 137),
            dashboard_header: rgb(125, 207, 255),
            dashboard_key: rgb(86, 95, 137),
            dashboard_value: rgb(192, 202, 245),
            dashboard_separator: rgb(59, 66, 97),
            dashboard_keys_hint: rgb(86, 95, 137),
            gauge_fill_green: rgb(158, 206, 106),
            gauge_fill_yellow: rgb(224, 175, 104),
            gauge_fill_red: rgb(247, 118, 142),
            gauge_bg: rgb(59, 66, 97),
            gauge_label: rgb(192, 202, 245),
            input_fg: rgb(192, 202, 245),
            input_cursor_fg: rgb(26, 27, 48),
            input_cursor_bg: rgb(125, 207, 255),
            input_border: rgb(59, 66, 97),
            input_border_active: rgb(125, 207, 255),
            border: rgb(59, 66, 97),
            border_active: rgb(125, 207, 255),
            scrollbar: rgb(59, 66, 97),
            spinner: rgb(125, 207, 255),
            status_message: rgb(125, 207, 255),
            key_hint: rgb(86, 95, 137),
            completion_fg: rgb(192, 202, 245),
            completion_bg: rgb(35, 40, 65),
            completion_selected_fg: rgb(26, 27, 48),
            completion_selected_bg: rgb(125, 207, 255),
            agent_running: rgb(125, 207, 255),
            agent_waiting: rgb(224, 175, 104),
            agent_done: rgb(158, 206, 106),
            agent_failed: rgb(247, 118, 142),
            agent_cancelled: rgb(86, 95, 137),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_catppuccin_mocha() -> Self {
        Self {
            name: "catppuccin-mocha".into(),
            conversation_text: rgb(205, 214, 244),
            conversation_user: rgb(137, 180, 250),
            conversation_system: rgb(249, 226, 175),
            conversation_error: rgb(243, 139, 168),
            conversation_assistant: rgb(205, 214, 244),
            conversation_dim: rgb(108, 112, 134),
            code_fg: rgb(205, 214, 244),
            code_bg: rgb(30, 30, 46),
            code_border: rgb(108, 112, 134),
            code_language_label: rgb(108, 112, 134),
            dashboard_header: rgb(137, 180, 250),
            dashboard_key: rgb(108, 112, 134),
            dashboard_value: rgb(205, 214, 244),
            dashboard_separator: rgb(69, 71, 90),
            dashboard_keys_hint: rgb(108, 112, 134),
            gauge_fill_green: rgb(166, 227, 161),
            gauge_fill_yellow: rgb(249, 226, 175),
            gauge_fill_red: rgb(243, 139, 168),
            gauge_bg: rgb(69, 71, 90),
            gauge_label: rgb(205, 214, 244),
            input_fg: rgb(205, 214, 244),
            input_cursor_fg: rgb(30, 30, 46),
            input_cursor_bg: rgb(137, 180, 250),
            input_border: rgb(69, 71, 90),
            input_border_active: rgb(137, 180, 250),
            border: rgb(69, 71, 90),
            border_active: rgb(137, 180, 250),
            scrollbar: rgb(69, 71, 90),
            spinner: rgb(137, 180, 250),
            status_message: rgb(137, 180, 250),
            key_hint: rgb(108, 112, 134),
            completion_fg: rgb(205, 214, 244),
            completion_bg: rgb(49, 50, 68),
            completion_selected_fg: rgb(30, 30, 46),
            completion_selected_bg: rgb(137, 180, 250),
            agent_running: rgb(137, 180, 250),
            agent_waiting: rgb(249, 226, 175),
            agent_done: rgb(166, 227, 161),
            agent_failed: rgb(243, 139, 168),
            agent_cancelled: rgb(108, 112, 134),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_catppuccin_latte() -> Self {
        Self {
            name: "catppuccin-latte".into(),
            conversation_text: rgb(76, 79, 105),
            conversation_user: rgb(30, 102, 245),
            conversation_system: rgb(223, 142, 29),
            conversation_error: rgb(210, 15, 57),
            conversation_assistant: rgb(76, 79, 105),
            conversation_dim: rgb(156, 160, 176),
            code_fg: rgb(76, 79, 105),
            code_bg: rgb(239, 241, 245),
            code_border: rgb(156, 160, 176),
            code_language_label: rgb(156, 160, 176),
            dashboard_header: rgb(30, 102, 245),
            dashboard_key: rgb(156, 160, 176),
            dashboard_value: rgb(76, 79, 105),
            dashboard_separator: rgb(204, 208, 218),
            dashboard_keys_hint: rgb(156, 160, 176),
            gauge_fill_green: rgb(64, 160, 43),
            gauge_fill_yellow: rgb(223, 142, 29),
            gauge_fill_red: rgb(210, 15, 57),
            gauge_bg: rgb(204, 208, 218),
            gauge_label: rgb(76, 79, 105),
            input_fg: rgb(76, 79, 105),
            input_cursor_fg: rgb(239, 241, 245),
            input_cursor_bg: rgb(30, 102, 245),
            input_border: rgb(204, 208, 218),
            input_border_active: rgb(30, 102, 245),
            border: rgb(204, 208, 218),
            border_active: rgb(30, 102, 245),
            scrollbar: rgb(204, 208, 218),
            spinner: rgb(30, 102, 245),
            status_message: rgb(30, 102, 245),
            key_hint: rgb(156, 160, 176),
            completion_fg: rgb(76, 79, 105),
            completion_bg: rgb(220, 224, 232),
            completion_selected_fg: rgb(239, 241, 245),
            completion_selected_bg: rgb(30, 102, 245),
            agent_running: rgb(30, 102, 245),
            agent_waiting: rgb(223, 142, 29),
            agent_done: rgb(64, 160, 43),
            agent_failed: rgb(210, 15, 57),
            agent_cancelled: rgb(156, 160, 176),
            syntax_theme: "InspiredGitHub".into(),
        }
    }

    fn theme_nord() -> Self {
        Self {
            name: "nord".into(),
            conversation_text: rgb(216, 222, 233),
            conversation_user: rgb(136, 192, 208),
            conversation_system: rgb(235, 203, 139),
            conversation_error: rgb(191, 97, 106),
            conversation_assistant: rgb(216, 222, 233),
            conversation_dim: rgb(76, 86, 106),
            code_fg: rgb(216, 222, 233),
            code_bg: rgb(46, 52, 64),
            code_border: rgb(76, 86, 106),
            code_language_label: rgb(76, 86, 106),
            dashboard_header: rgb(136, 192, 208),
            dashboard_key: rgb(76, 86, 106),
            dashboard_value: rgb(216, 222, 233),
            dashboard_separator: rgb(59, 66, 81),
            dashboard_keys_hint: rgb(76, 86, 106),
            gauge_fill_green: rgb(163, 190, 140),
            gauge_fill_yellow: rgb(235, 203, 139),
            gauge_fill_red: rgb(191, 97, 106),
            gauge_bg: rgb(59, 66, 81),
            gauge_label: rgb(216, 222, 233),
            input_fg: rgb(216, 222, 233),
            input_cursor_fg: rgb(46, 52, 64),
            input_cursor_bg: rgb(136, 192, 208),
            input_border: rgb(59, 66, 81),
            input_border_active: rgb(136, 192, 208),
            border: rgb(59, 66, 81),
            border_active: rgb(136, 192, 208),
            scrollbar: rgb(59, 66, 81),
            spinner: rgb(136, 192, 208),
            status_message: rgb(136, 192, 208),
            key_hint: rgb(76, 86, 106),
            completion_fg: rgb(216, 222, 233),
            completion_bg: rgb(67, 76, 94),
            completion_selected_fg: rgb(46, 52, 64),
            completion_selected_bg: rgb(136, 192, 208),
            agent_running: rgb(136, 192, 208),
            agent_waiting: rgb(235, 203, 139),
            agent_done: rgb(163, 190, 140),
            agent_failed: rgb(191, 97, 106),
            agent_cancelled: rgb(76, 86, 106),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_gruvbox() -> Self {
        Self {
            name: "gruvbox".into(),
            conversation_text: rgb(235, 219, 178),
            conversation_user: rgb(131, 165, 152),
            conversation_system: rgb(250, 189, 47),
            conversation_error: rgb(251, 73, 52),
            conversation_assistant: rgb(235, 219, 178),
            conversation_dim: rgb(146, 131, 116),
            code_fg: rgb(235, 219, 178),
            code_bg: rgb(40, 40, 40),
            code_border: rgb(146, 131, 116),
            code_language_label: rgb(146, 131, 116),
            dashboard_header: rgb(131, 165, 152),
            dashboard_key: rgb(146, 131, 116),
            dashboard_value: rgb(235, 219, 178),
            dashboard_separator: rgb(80, 73, 69),
            dashboard_keys_hint: rgb(146, 131, 116),
            gauge_fill_green: rgb(184, 187, 38),
            gauge_fill_yellow: rgb(250, 189, 47),
            gauge_fill_red: rgb(251, 73, 52),
            gauge_bg: rgb(80, 73, 69),
            gauge_label: rgb(235, 219, 178),
            input_fg: rgb(235, 219, 178),
            input_cursor_fg: rgb(40, 40, 40),
            input_cursor_bg: rgb(131, 165, 152),
            input_border: rgb(80, 73, 69),
            input_border_active: rgb(131, 165, 152),
            border: rgb(80, 73, 69),
            border_active: rgb(131, 165, 152),
            scrollbar: rgb(80, 73, 69),
            spinner: rgb(131, 165, 152),
            status_message: rgb(131, 165, 152),
            key_hint: rgb(146, 131, 116),
            completion_fg: rgb(235, 219, 178),
            completion_bg: rgb(60, 56, 54),
            completion_selected_fg: rgb(40, 40, 40),
            completion_selected_bg: rgb(131, 165, 152),
            agent_running: rgb(131, 165, 152),
            agent_waiting: rgb(250, 189, 47),
            agent_done: rgb(184, 187, 38),
            agent_failed: rgb(251, 73, 52),
            agent_cancelled: rgb(146, 131, 116),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_dracula() -> Self {
        Self {
            name: "dracula".into(),
            conversation_text: rgb(248, 248, 242),
            conversation_user: rgb(139, 233, 253),
            conversation_system: rgb(241, 250, 140),
            conversation_error: rgb(255, 85, 85),
            conversation_assistant: rgb(248, 248, 242),
            conversation_dim: rgb(98, 114, 164),
            code_fg: rgb(248, 248, 242),
            code_bg: rgb(40, 42, 54),
            code_border: rgb(98, 114, 164),
            code_language_label: rgb(98, 114, 164),
            dashboard_header: rgb(139, 233, 253),
            dashboard_key: rgb(98, 114, 164),
            dashboard_value: rgb(248, 248, 242),
            dashboard_separator: rgb(68, 71, 90),
            dashboard_keys_hint: rgb(98, 114, 164),
            gauge_fill_green: rgb(80, 250, 123),
            gauge_fill_yellow: rgb(241, 250, 140),
            gauge_fill_red: rgb(255, 85, 85),
            gauge_bg: rgb(68, 71, 90),
            gauge_label: rgb(248, 248, 242),
            input_fg: rgb(248, 248, 242),
            input_cursor_fg: rgb(40, 42, 54),
            input_cursor_bg: rgb(139, 233, 253),
            input_border: rgb(68, 71, 90),
            input_border_active: rgb(139, 233, 253),
            border: rgb(68, 71, 90),
            border_active: rgb(139, 233, 253),
            scrollbar: rgb(68, 71, 90),
            spinner: rgb(139, 233, 253),
            status_message: rgb(139, 233, 253),
            key_hint: rgb(98, 114, 164),
            completion_fg: rgb(248, 248, 242),
            completion_bg: rgb(55, 57, 72),
            completion_selected_fg: rgb(40, 42, 54),
            completion_selected_bg: rgb(139, 233, 253),
            agent_running: rgb(139, 233, 253),
            agent_waiting: rgb(241, 250, 140),
            agent_done: rgb(80, 250, 123),
            agent_failed: rgb(255, 85, 85),
            agent_cancelled: rgb(98, 114, 164),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_solarized_dark() -> Self {
        Self {
            name: "solarized-dark".into(),
            conversation_text: rgb(147, 161, 161),
            conversation_user: rgb(42, 161, 152),
            conversation_system: rgb(181, 137, 0),
            conversation_error: rgb(220, 50, 47),
            conversation_assistant: rgb(147, 161, 161),
            conversation_dim: rgb(88, 110, 117),
            code_fg: rgb(147, 161, 161),
            code_bg: rgb(0, 43, 54),
            code_border: rgb(88, 110, 117),
            code_language_label: rgb(88, 110, 117),
            dashboard_header: rgb(42, 161, 152),
            dashboard_key: rgb(88, 110, 117),
            dashboard_value: rgb(147, 161, 161),
            dashboard_separator: rgb(7, 54, 66),
            dashboard_keys_hint: rgb(88, 110, 117),
            gauge_fill_green: rgb(133, 153, 0),
            gauge_fill_yellow: rgb(181, 137, 0),
            gauge_fill_red: rgb(220, 50, 47),
            gauge_bg: rgb(7, 54, 66),
            gauge_label: rgb(147, 161, 161),
            input_fg: rgb(147, 161, 161),
            input_cursor_fg: rgb(0, 43, 54),
            input_cursor_bg: rgb(42, 161, 152),
            input_border: rgb(7, 54, 66),
            input_border_active: rgb(42, 161, 152),
            border: rgb(7, 54, 66),
            border_active: rgb(42, 161, 152),
            scrollbar: rgb(7, 54, 66),
            spinner: rgb(42, 161, 152),
            status_message: rgb(42, 161, 152),
            key_hint: rgb(88, 110, 117),
            completion_fg: rgb(147, 161, 161),
            completion_bg: rgb(14, 63, 75),
            completion_selected_fg: rgb(0, 43, 54),
            completion_selected_bg: rgb(42, 161, 152),
            agent_running: rgb(42, 161, 152),
            agent_waiting: rgb(181, 137, 0),
            agent_done: rgb(133, 153, 0),
            agent_failed: rgb(220, 50, 47),
            agent_cancelled: rgb(88, 110, 117),
            syntax_theme: "base16-ocean.dark".into(),
        }
    }

    fn theme_solarized_light() -> Self {
        Self {
            name: "solarized-light".into(),
            conversation_text: rgb(101, 123, 131),
            conversation_user: rgb(42, 161, 152),
            conversation_system: rgb(181, 137, 0),
            conversation_error: rgb(220, 50, 47),
            conversation_assistant: rgb(101, 123, 131),
            conversation_dim: rgb(147, 161, 161),
            code_fg: rgb(101, 123, 131),
            code_bg: rgb(253, 246, 227),
            code_border: rgb(147, 161, 161),
            code_language_label: rgb(147, 161, 161),
            dashboard_header: rgb(42, 161, 152),
            dashboard_key: rgb(147, 161, 161),
            dashboard_value: rgb(101, 123, 131),
            dashboard_separator: rgb(238, 232, 213),
            dashboard_keys_hint: rgb(147, 161, 161),
            gauge_fill_green: rgb(133, 153, 0),
            gauge_fill_yellow: rgb(181, 137, 0),
            gauge_fill_red: rgb(220, 50, 47),
            gauge_bg: rgb(238, 232, 213),
            gauge_label: rgb(101, 123, 131),
            input_fg: rgb(101, 123, 131),
            input_cursor_fg: rgb(253, 246, 227),
            input_cursor_bg: rgb(42, 161, 152),
            input_border: rgb(238, 232, 213),
            input_border_active: rgb(42, 161, 152),
            border: rgb(238, 232, 213),
            border_active: rgb(42, 161, 152),
            scrollbar: rgb(238, 232, 213),
            spinner: rgb(42, 161, 152),
            status_message: rgb(42, 161, 152),
            key_hint: rgb(147, 161, 161),
            completion_fg: rgb(101, 123, 131),
            completion_bg: rgb(238, 232, 213),
            completion_selected_fg: rgb(253, 246, 227),
            completion_selected_bg: rgb(42, 161, 152),
            agent_running: rgb(42, 161, 152),
            agent_waiting: rgb(181, 137, 0),
            agent_done: rgb(133, 153, 0),
            agent_failed: rgb(220, 50, 47),
            agent_cancelled: rgb(147, 161, 161),
            syntax_theme: "InspiredGitHub".into(),
        }
    }

    fn theme_monokai() -> Self {
        Self {
            name: "monokai".into(),
            conversation_text: rgb(248, 248, 242),
            conversation_user: rgb(166, 226, 46),
            conversation_system: rgb(230, 219, 116),
            conversation_error: rgb(249, 38, 114),
            conversation_assistant: rgb(248, 248, 242),
            conversation_dim: rgb(117, 113, 94),
            code_fg: rgb(248, 248, 242),
            code_bg: rgb(39, 40, 34),
            code_border: rgb(117, 113, 94),
            code_language_label: rgb(117, 113, 94),
            dashboard_header: rgb(166, 226, 46),
            dashboard_key: rgb(117, 113, 94),
            dashboard_value: rgb(248, 248, 242),
            dashboard_separator: rgb(75, 73, 62),
            dashboard_keys_hint: rgb(117, 113, 94),
            gauge_fill_green: rgb(166, 226, 46),
            gauge_fill_yellow: rgb(230, 219, 116),
            gauge_fill_red: rgb(249, 38, 114),
            gauge_bg: rgb(75, 73, 62),
            gauge_label: rgb(248, 248, 242),
            input_fg: rgb(248, 248, 242),
            input_cursor_fg: rgb(39, 40, 34),
            input_cursor_bg: rgb(166, 226, 46),
            input_border: rgb(75, 73, 62),
            input_border_active: rgb(166, 226, 46),
            border: rgb(75, 73, 62),
            border_active: rgb(166, 226, 46),
            scrollbar: rgb(75, 73, 62),
            spinner: rgb(166, 226, 46),
            status_message: rgb(166, 226, 46),
            key_hint: rgb(117, 113, 94),
            completion_fg: rgb(248, 248, 242),
            completion_bg: rgb(55, 54, 46),
            completion_selected_fg: rgb(39, 40, 34),
            completion_selected_bg: rgb(166, 226, 46),
            agent_running: rgb(166, 226, 46),
            agent_waiting: rgb(230, 219, 116),
            agent_done: rgb(166, 226, 46),
            agent_failed: rgb(249, 38, 114),
            agent_cancelled: rgb(117, 113, 94),
            syntax_theme: "base16-monokai.dark".into(),
        }
    }

    fn theme_system() -> Self {
        let mut theme = Self::theme_default();
        theme.name = "system".into();
        theme.syntax_theme = "InspiredGitHub".into();
        theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_builtins_exist() {
        for name in TuiTheme::all_builtin_names() {
            assert!(TuiTheme::builtin(name).is_some(), "missing theme: {name}");
        }
    }

    #[test]
    fn test_color_def_named() {
        assert_eq!(ColorDef::Named("red".into()).to_color(), Color::Red);
        assert_eq!(ColorDef::Named("cyan".into()).to_color(), Color::Cyan);
    }

    #[test]
    fn test_color_def_rgb() {
        assert_eq!(ColorDef::Rgb { r: 255, g: 0, b: 0 }.to_color(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_color_def_ansi256() {
        assert_eq!(ColorDef::Ansi256(196).to_color(), Color::Indexed(196));
    }

    #[test]
    fn test_from_json_valid() {
        let theme = TuiTheme::builtin("default").unwrap();
        let json = theme.to_json();
        let parsed = TuiTheme::from_json(&json);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().name, "default");
    }

    #[test]
    fn test_from_json_invalid() {
        assert!(TuiTheme::from_json("not json").is_err());
    }

    #[test]
    fn test_unknown_theme_returns_none() {
        assert!(TuiTheme::builtin("nonexistent").is_none());
    }

    #[test]
    fn test_system_theme_uses_ansi_only() {
        let theme = TuiTheme::builtin("system").unwrap();
        // All fields should be Named colors
        assert!(matches!(theme.conversation_text, ColorDef::Named(_)));
        assert!(matches!(theme.conversation_user, ColorDef::Named(_)));
    }
}
