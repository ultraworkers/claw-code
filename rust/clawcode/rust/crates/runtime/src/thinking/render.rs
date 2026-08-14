//! `render_reasoning` — formats accumulated reasoning text as an
//! ANSI-styled terminal block with a `┃` gutter and a
//! `Thinking:` / `Thought:` label.
//!
//! The renderer applies a *mixed* dimmed color (foreground blended
//! toward the background by a fixed ratio) rather than the `\x1b[2m`
//! DIM attribute. The DIM attribute renders inconsistently across
//! terminals (Windows Terminal dims heavily; Ghostty barely dims at
//! all). Using a literal mixed color gives the same visual on every
//! terminal.
//!
//! The renderer does **not** perform markdown processing — it treats
//! the input as opaque text and applies word-wrap at `width - 2` (to
//! account for the `┃ ` gutter). Callers that want markdown rendering
//! of the reasoning should pre-render with their markdown helper of
//! choice and pass the lines through [`render_reasoning_lines`].

/// Color theme for reasoning rendering.
///
/// `dim` and `label` are RGB triples that get serialized as
/// `\x1b[38;2;R;G;Bm` truecolor sequences. Most modern terminals
/// (`Windows Terminal`, iTerm, `Ghostty`, `Kitty`, `WezTerm`) render truecolor
/// directly; older terminals fall back to a close ANSI 256-color
/// approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReasoningTheme {
    /// Foreground color for the dimmed body text (RGB).
    pub dim: (u8, u8, u8),
    /// Color for the `Thinking:` / `Thought:` label and the gutter `┃`
    /// character (RGB).
    pub label: (u8, u8, u8),
}

impl Default for ReasoningTheme {
    fn default() -> Self {
        // grey64 for body, grey100 for label — readable on both light
        // and dark backgrounds
        Self {
            dim: (0x80, 0x80, 0x80),
            label: (0xC0, 0xC0, 0xC0),
        }
    }
}

/// Render the reasoning text to an ANSI-styled string with a `┃` gutter
/// and a `Thinking:` / `Thought:` label.
///
/// Output format (one line per `\n`):
/// ```text
/// ┃ <label>                        (italic label color)
/// ┃ <word-wrapped line 1>          (dim color)
/// ┃ <word-wrapped line 2>
/// ...
/// ```
///
/// - `width` is the total terminal width in columns.
/// - Empty `reasoning` returns just the header line.
/// - The `┃` gutter is exactly 2 columns wide (1 for the character, 1
///   for the space).
#[must_use]
pub fn render_reasoning(
    reasoning: &str,
    width: usize,
    theme: &ReasoningTheme,
    is_streaming: bool,
) -> String {
    let lines = render_reasoning_lines(reasoning, width, theme, is_streaming);
    lines.join("\n") + "\n"
}

/// Same as [`render_reasoning`] but returns one `String` per line, no
/// trailing newline. Useful for callers that want to compose with other
/// line-based output.
#[must_use]
pub fn render_reasoning_lines(
    reasoning: &str,
    width: usize,
    theme: &ReasoningTheme,
    is_streaming: bool,
) -> Vec<String> {
    let label = if is_streaming { "Thinking:" } else { "Thought:" };
    let mut output: Vec<String> = Vec::new();

    // Header line
    output.push(format!(
        "{gutter_color}┃ {italic}{label_color}{italic_label}{reset}",
        gutter_color = fg(theme.label),
        italic = "\x1b[3m",
        label_color = fg(theme.label),
        italic_label = label,
        reset = "\x1b[0m",
    ));

    let body_width = width.saturating_sub(2).max(1);
    let dim_seq = fg(theme.dim);

    // Word-wrap the body. Skip entirely when reasoning is empty/whitespace
    // so callers get just the header (matches tidev's
    // `reasoning_lines_preserve_empty_state` behavior).
    if reasoning.trim().is_empty() {
        return output;
    }

    for line in reasoning.split('\n') {
        if line.is_empty() {
            output.push(format!("{dim_seq}┃ \x1b[0m"));
            continue;
        }
        for wrapped in wrap_line(line, body_width) {
            output.push(format!("{dim_seq}┃ \x1b[0m{wrapped}"));
        }
    }

    output
}

/// Render a single line of reasoning into one or more wrapped lines,
/// each no longer than `width` columns.
fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }

    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_cols: usize = 0;

    for word in line.split_whitespace() {
        let word_cols = unicode_width::UnicodeWidthStr::width(word);

        if current.is_empty() {
            // first word on the line
            if word_cols > width {
                // word is longer than the available width — emit it
                // as-is, no wrap possible
                out.push(word.to_string());
                continue;
            }
            current.push_str(word);
            current_cols = word_cols;
        } else {
            // need a space before the word
            if current_cols + 1 + word_cols > width {
                out.push(std::mem::take(&mut current));
                current.push_str(word);
                current_cols = word_cols;
            } else {
                current.push(' ');
                current.push_str(word);
                current_cols += 1 + word_cols;
            }
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    if out.is_empty() {
        out.push(String::new());
    }

    out
}

fn fg((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_reasoning_header_switches() {
        let theme = ReasoningTheme::default();
        let streaming = render_reasoning_lines("hello", 80, &theme, true);
        let final_ = render_reasoning_lines("hello", 80, &theme, false);
        assert!(streaming[0].contains("Thinking:"));
        assert!(!streaming[0].contains("Thought:"));
        assert!(final_[0].contains("Thought:"));
        assert!(!final_[0].contains("Thinking:"));
    }

    #[test]
    fn render_reasoning_dimmed_color() {
        let theme = ReasoningTheme {
            dim: (100, 110, 120),
            label: (200, 200, 200),
        };
        let lines = render_reasoning_lines("first\nsecond", 40, &theme, true);
        // lines[1] and lines[2] should contain the truecolor dim sequence
        assert!(lines[1].contains("\x1b[38;2;100;110;120m"));
        assert!(lines[2].contains("\x1b[38;2;100;110;120m"));
    }

    #[test]
    fn render_reasoning_word_wrap() {
        let theme = ReasoningTheme::default();
        // width=40 → body_width=38 (subtracting the 2-col gutter)
        let long = "a b c d e f g h i j k l m n o p q r s t u v w x y z";
        let lines = render_reasoning_lines(long, 40, &theme, true);
        // every body line (skipping the header) must have at most
        // 38 cols of content *after* the 2-col "┃ " gutter
        for line in &lines[1..] {
            let visible = strip_ansi(line);
            // visible looks like "┃ <content>" — drop the gutter before
            // measuring the wrapped body
            let content = visible.strip_prefix("┃ ").unwrap_or(&visible);
            assert!(
                unicode_width::UnicodeWidthStr::width(content) <= 38,
                "line too long ({} cols): {line}",
                unicode_width::UnicodeWidthStr::width(content)
            );
        }
    }

    #[test]
    fn render_reasoning_empty() {
        let theme = ReasoningTheme::default();
        let lines = render_reasoning_lines("", 80, &theme, true);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Thinking:"));
    }

    #[test]
    fn render_reasoning_multiline_input() {
        let theme = ReasoningTheme::default();
        let lines = render_reasoning_lines("line1\nline2", 80, &theme, false);
        // header + 2 body lines
        assert_eq!(lines.len(), 3);
        assert!(lines[1].contains("line1"));
        assert!(lines[2].contains("line2"));
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // skip until terminator (letter)
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }
}
