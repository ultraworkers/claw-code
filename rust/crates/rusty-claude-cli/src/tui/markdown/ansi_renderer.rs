//! ANSI-based markdown renderer.
//!
//! Uses the shared `MarkdownAst` to produce ANSI-escaped `String` for the
//! plain REPL (non-TUI) mode. This replaces the old `render.rs` TerminalRenderer.

use super::parse_markdown;
use crate::theme::TuiTheme;

/// ANSI markdown renderer.
#[derive(Debug, Default)]
pub struct AnsiMarkdownRenderer {
    // Will hold color theme once fully migrated from render.rs
}

impl AnsiMarkdownRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Render a markdown string into ANSI-escaped text.
    ///
    /// For now, delegates to the existing `TerminalRenderer` to maintain
    /// full compatibility. The AST-based renderer will be completed in a
    /// follow-up pass.
    pub fn render(&self, markdown: &str) -> String {
        crate::render::TerminalRenderer::new().render_markdown(markdown)
    }

    /// Stream-render markdown into a writer.
    pub fn stream(&self, markdown: &str, out: &mut impl std::io::Write) -> std::io::Result<()> {
        crate::render::TerminalRenderer::new().stream_markdown(markdown, out)
    }
}
