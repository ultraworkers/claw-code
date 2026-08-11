use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use crossterm::cursor::{MoveToColumn, RestorePosition, SavePosition};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor, Stylize};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, queue};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use phf::phf_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTheme {
    heading: Color,
    emphasis: Color,
    strong: Color,
    inline_code: Color,
    link: Color,
    quote: Color,
    table_border: Color,
    table_row_alt: Color,
    code_block_border: Color,
    math_fraction: Color,
    spinner_active: Color,
    spinner_done: Color,
    spinner_failed: Color,
    reasoning_header: Color,
    reasoning_body: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            heading: Color::Cyan,
            emphasis: Color::Magenta,
            strong: Color::Yellow,
            inline_code: Color::Green,
            link: Color::Blue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            table_row_alt: Color::Rgb {
                r: 0x2E,
                g: 0x2E,
                b: 0x33,
            },
            code_block_border: Color::DarkGrey,
            math_fraction: Color::Cyan,
            spinner_active: Color::Blue,
            spinner_done: Color::DarkGrey,
            spinner_failed: Color::Red,
            reasoning_header: Color::Rgb {
                r: 0xC0,
                g: 0xC0,
                b: 0xC0,
            },
            reasoning_body: Color::Rgb {
                r: 0x80,
                g: 0x80,
                b: 0x80,
            },
        }
    }
}

impl ColorTheme {
    /// Build a theme from an iterator of `(key, value)` pairs.  Unrecognised
    /// keys or colours silently fall back to the default.
    pub fn from_iter<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut theme = Self::default();
        for (key, value) in iter {
            if let Ok(color) = Self::parse_color(value.as_ref()) {
                match key.as_ref() {
                    "heading" => theme.heading = color,
                    "emphasis" => theme.emphasis = color,
                    "strong" => theme.strong = color,
                    "inline_code" => theme.inline_code = color,
                    "link" => theme.link = color,
                    "quote" => theme.quote = color,
                    "table_border" => theme.table_border = color,
                    "table_row_alt" => theme.table_row_alt = color,
                    "code_block_border" => theme.code_block_border = color,
                    "math_fraction" => theme.math_fraction = color,
                    "spinner_active" => theme.spinner_active = color,
                    "spinner_done" => theme.spinner_done = color,
                    "spinner_failed" => theme.spinner_failed = color,
                    "reasoning_header" => theme.reasoning_header = color,
                    "reasoning_body" => theme.reasoning_body = color,
                    _ => {}
                }
            }
        }
        theme
    }

    fn parse_color(s: &str) -> Result<Color, String> {
        if let Some(hex) = s.strip_prefix('#') {
            if hex.len() == 6 {
                let r =
                    u8::from_str_radix(&hex[0..2], 16).map_err(|_| "invalid hex".to_string())?;
                let g =
                    u8::from_str_radix(&hex[2..4], 16).map_err(|_| "invalid hex".to_string())?;
                let b =
                    u8::from_str_radix(&hex[4..6], 16).map_err(|_| "invalid hex".to_string())?;
                return Ok(Color::Rgb { r, g, b });
            }
            return Err("invalid hex length".to_string());
        }
        match s.to_lowercase().as_str() {
            "black" => Ok(Color::Black),
            "darkgrey" | "dark_grey" => Ok(Color::DarkGrey),
            "red" => Ok(Color::Red),
            "darkred" | "dark_red" => Ok(Color::DarkRed),
            "green" => Ok(Color::Green),
            "darkgreen" | "dark_green" => Ok(Color::DarkGreen),
            "yellow" => Ok(Color::Yellow),
            "darkyellow" | "dark_yellow" => Ok(Color::DarkYellow),
            "blue" => Ok(Color::Blue),
            "darkblue" | "dark_blue" => Ok(Color::DarkBlue),
            "magenta" => Ok(Color::Magenta),
            "darkmagenta" | "dark_magenta" => Ok(Color::DarkMagenta),
            "cyan" => Ok(Color::Cyan),
            "darkcyan" | "dark_cyan" => Ok(Color::DarkCyan),
            "white" => Ok(Color::White),
            "grey" | "gray" => Ok(Color::Grey),
            _ => Err(format!("unknown color: {s}")),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spinner {
    frame_index: usize,
}

impl Spinner {
    const FRAMES: [&str; 5] = ["\u{2802}", "\u{2810}", "\u{2818}", "\u{2830}", "\u{2838}"];

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index += 1;
        queue!(
            out,
            SavePosition,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_active),
            Print(format!("{label}")),
            ResetColor,
            RestorePosition
        )?;
        out.flush()
    }

    pub fn finish(
        &mut self,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        execute!(
            out,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
        )?;
        out.flush()
    }

    pub fn fail(
        &mut self,
        label: &str,
        theme: &ColorTheme,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.frame_index = 0;
        execute!(
            out,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_failed),
            Print(format!("✗ {label}\n")),
            ResetColor
        )?;
        out.flush()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ListKind {
    Unordered,
    Ordered { next_index: u64 },
}

#[derive(Debug, Default, Clone, PartialEq)]
struct TableState {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
    alignments: Vec<Alignment>,
}

impl TableState {
    fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    fn finish_row(&mut self) {
        if self.current_row.is_empty() {
            return;
        }
        let row = std::mem::take(&mut self.current_row);
        if self.in_head {
            self.headers = row;
        } else {
            self.rows.push(row);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RenderState {
    emphasis: usize,
    strong: usize,
    heading_level: Option<u8>,
    quote: usize,
    list_stack: Vec<ListKind>,
    link_stack: Vec<LinkState>,
    table: Option<TableState>,
    in_table_head: bool,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            emphasis: 0,
            strong: 0,
            heading_level: None,
            quote: 0,
            list_stack: Vec::new(),
            link_stack: Vec::new(),
            table: None,
            in_table_head: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkState {
    destination: String,
    text: String,
}

impl RenderState {
    fn style_text(&self, text: &str, theme: &ColorTheme) -> String {
        let mut style = text.stylize();

        if self.in_table_head {
            style = style.bold().with(theme.heading);
        }

        if self.quote > 0 {
            style = style.with(theme.quote);
        }

        if let Some(level) = self.heading_level {
            style = style.bold();
            style = match level {
                1 => style.with(theme.heading),
                2 => style.white(),
                3 => style.with(Color::Blue),
                _ => style.with(Color::Grey),
            };
        } else {
            if self.strong > 0 {
                style = style.bold().with(theme.strong);
            } else if self.emphasis > 0 {
                style = style.italic().with(theme.emphasis);
            }
        }

        format!("{style}")
    }

    fn append_raw(&mut self, output: &mut String, text: &str) {
        if let Some(link) = self.link_stack.last_mut() {
            link.text.push_str(text);
        } else if let Some(table) = self.table.as_mut() {
            table.current_cell.push_str(text);
        } else {
            output.push_str(text);
        }
    }

    fn append_styled(&mut self, output: &mut String, text: &str, theme: &ColorTheme) {
        let styled = self.style_text(text, theme);
        self.append_raw(output, &styled);
    }
}

#[derive(Debug)]
pub struct TerminalRenderer {
    syntax_set: SyntaxSet,
    syntax_theme: Theme,
    color_theme: ColorTheme,
    max_width: Option<usize>,
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax_theme = ThemeSet::load_defaults()
            .themes
            .remove("base16-ocean.dark")
            .unwrap_or_default();
        Self {
            syntax_set,
            syntax_theme,
            color_theme: ColorTheme::default(),
            max_width: None,
        }
    }
}

impl TerminalRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum rendering width (in columns) for tables and other
    /// layout.  When unset (the default) no truncation is performed.
    pub fn set_max_width(&mut self, width: usize) {
        self.max_width = Some(width);
    }

    #[must_use]
    pub fn color_theme(&self) -> &ColorTheme {
        &self.color_theme
    }

    #[must_use]
    pub fn render_markdown(&self, markdown: &str) -> String {
        let output = self.render_markdown_inner(markdown);
        output.trim_end().to_string()
    }

    #[must_use]
    pub fn render_markdown_streaming(&self, markdown: &str) -> String {
        self.render_markdown_inner(markdown)
    }

    #[must_use]
    pub fn markdown_to_ansi(&self, markdown: &str) -> String {
        self.render_markdown(markdown)
    }

    fn render_markdown_inner(&self, markdown: &str) -> String {
        let escaped = escape_pipes_in_spans(markdown);
        let with_tool_calls = preprocess_tool_call_markdown(&escaped);
        let degraded = degrade_latex(&self.color_theme, &with_tool_calls);
        let normalized = close_dangling_fence(&normalize_nested_fences(&degraded));
        let mut output = String::new();
        let mut state = RenderState::default();
        let mut code_language = String::new();
        let mut code_buffer = String::new();
        let mut in_code_block = false;

        for event in Parser::new_ext(&normalized, Options::all()) {
            self.render_event(
                event,
                &mut state,
                &mut output,
                &mut code_buffer,
                &mut code_language,
                &mut in_code_block,
            );
        }

        output.replace(MATH_PIPE_SENTINEL, "|")
    }

    /// Render a reasoning/thinking block as an ANSI-styled terminal
    /// string with a `┃` gutter and a `Thinking:` / `Thought:` label.
    ///
    /// Uses truecolour RGB for consistent dimming across terminals
    /// (avoids the `\x1b[2m` DIM attribute which renders inconsistently).
    ///
    /// `width` is the total terminal width in columns; the body is
    /// wrapped to `width - 2` to leave room for the `┃` gutter.
    /// `is_streaming` controls the label text (`Thinking:` while the
    /// block is still being received, `Thought:` for completed blocks).
    #[must_use]
    pub fn render_reasoning_block(
        &self,
        reasoning: &str,
        width: usize,
        is_streaming: bool,
    ) -> String {
        let lines = render_reasoning_lines(
            reasoning,
            width,
            self.color_theme.reasoning_header,
            self.color_theme.reasoning_body,
            is_streaming,
        );
        let mut out = String::new();
        for line in &lines {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    #[allow(clippy::too_many_lines)]
    fn render_event(
        &self,
        event: Event<'_>,
        state: &mut RenderState,
        output: &mut String,
        code_buffer: &mut String,
        code_language: &mut String,
        in_code_block: &mut bool,
    ) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                Self::start_heading(state, level as u8, output);
            }
            Event::End(TagEnd::Paragraph) => output.push_str("\n\n"),
            Event::Start(Tag::BlockQuote(..)) => self.start_quote(state, output),
            Event::End(TagEnd::BlockQuote(..)) => {
                state.quote = state.quote.saturating_sub(1);
                output.push('\n');
            }
            Event::End(TagEnd::Heading(..)) => {
                state.heading_level = None;
                output.push_str("\n\n");
            }
            Event::End(TagEnd::Item) | Event::SoftBreak | Event::HardBreak => {
                state.append_raw(output, "\n");
            }
            Event::Start(Tag::List(first_item)) => {
                let kind = match first_item {
                    Some(index) => ListKind::Ordered { next_index: index },
                    None => ListKind::Unordered,
                };
                state.list_stack.push(kind);
            }
            Event::End(TagEnd::List(..)) => {
                state.list_stack.pop();
                output.push('\n');
            }
            Event::Start(Tag::Item) => Self::start_item(state, output),
            Event::Start(Tag::CodeBlock(kind)) => {
                *in_code_block = true;
                *code_language = match kind {
                    CodeBlockKind::Indented => String::from("text"),
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                };
                code_buffer.clear();
                self.start_code_block(code_language, output);
            }
            Event::End(TagEnd::CodeBlock) => {
                self.finish_code_block(code_buffer, code_language, output);
                *in_code_block = false;
                code_language.clear();
                code_buffer.clear();
            }
            Event::Start(Tag::Emphasis) => state.emphasis += 1,
            Event::End(TagEnd::Emphasis) => state.emphasis = state.emphasis.saturating_sub(1),
            Event::Start(Tag::Strong) => state.strong += 1,
            Event::End(TagEnd::Strong) => state.strong = state.strong.saturating_sub(1),
            Event::Code(code) => {
                let rendered =
                    format!("{}", format!("`{code}`").with(self.color_theme.inline_code));
                state.append_raw(output, &rendered);
            }
            Event::Rule => output.push_str("---\n"),
            Event::Text(text) => {
                self.push_text(text.as_ref(), state, output, code_buffer, *in_code_block);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                state.append_raw(output, &html);
            }
            Event::FootnoteReference(reference) => {
                state.append_raw(output, &format!("[{reference}]"));
            }
            Event::TaskListMarker(done) => {
                let marker = if done {
                    format!("{} ", "[x]".with(Color::Green))
                } else {
                    format!("{} ", "[ ]".with(Color::DarkGrey))
                };
                state.append_raw(output, &marker);
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                state.append_raw(output, &math);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                state.link_stack.push(LinkState {
                    destination: dest_url.to_string(),
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Link) => {
                if let Some(link) = state.link_stack.pop() {
                    let label = if link.text.is_empty() {
                        link.destination.clone()
                    } else {
                        link.text
                    };
                    let rendered = format!(
                        "{}",
                        format!("[{label}]({})", link.destination)
                            .underlined()
                            .with(self.color_theme.link)
                    );
                    state.append_raw(output, &rendered);
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let rendered = format!(
                    "{}",
                    format!("[image:{dest_url}]").with(self.color_theme.link)
                );
                state.append_raw(output, &rendered);
            }
            Event::Start(Tag::Table(alignments)) => {
                let mut table = TableState::default();
                table.alignments = alignments.clone();
                state.table = Some(table);
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = state.table.take() {
                    output.push_str(&self.render_table(&table));
                    output.push_str("\n\n");
                }
            }
            Event::Start(Tag::TableHead) => {
                state.in_table_head = true;
                if let Some(table) = state.table.as_mut() {
                    table.in_head = true;
                }
            }
            Event::End(TagEnd::TableHead) => {
                state.in_table_head = false;
                if let Some(table) = state.table.as_mut() {
                    table.finish_row();
                    table.in_head = false;
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(table) = state.table.as_mut() {
                    table.current_row.clear();
                    table.current_cell.clear();
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = state.table.as_mut() {
                    table.finish_row();
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(table) = state.table.as_mut() {
                    table.current_cell.clear();
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = state.table.as_mut() {
                    table.push_cell();
                }
            }
            Event::Start(Tag::Paragraph | Tag::MetadataBlock(..) | _)
            | Event::End(TagEnd::Image | TagEnd::MetadataBlock(..) | _) => {}
        }
    }

    fn start_heading(state: &mut RenderState, level: u8, output: &mut String) {
        state.heading_level = Some(level);
        if !output.is_empty() {
            output.push('\n');
        }
    }

    fn start_quote(&self, state: &mut RenderState, output: &mut String) {
        state.quote += 1;
        let _ = write!(output, "{} ", "│".with(self.color_theme.quote));
    }

    fn start_item(state: &mut RenderState, output: &mut String) {
        let depth = state.list_stack.len().saturating_sub(1);
        output.push_str(&"  ".repeat(depth));

        let marker = match state.list_stack.last_mut() {
            Some(ListKind::Ordered { next_index }) => {
                let value = *next_index;
                *next_index += 1;
                format!("{value}. ")
            }
            _ => match depth {
                0 => "• ".to_string(),
                1 => "◦ ".to_string(),
                2 => "▪ ".to_string(),
                _ => "▸ ".to_string(),
            },
        };
        output.push_str(&marker);
    }

    fn start_code_block(&self, code_language: &str, output: &mut String) {
        let label = if code_language.is_empty() {
            "code".to_string()
        } else {
            code_language.to_string()
        };
        let _ = writeln!(
            output,
            "{}",
            format!("╭─ {label}")
                .bold()
                .with(self.color_theme.code_block_border)
        );
    }

    fn finish_code_block(&self, code_buffer: &str, code_language: &str, output: &mut String) {
        let highlighted = self.highlight_code(code_buffer, code_language);
        output.push_str(&self.add_line_numbers(&highlighted));
        let _ = write!(
            output,
            "{}",
            "╰─".bold().with(self.color_theme.code_block_border)
        );
        output.push_str("\n\n");
    }

    fn add_line_numbers(&self, code: &str) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let total = lines.len();
        let num_width = if total == 0 {
            2
        } else {
            (total as f64).log10().ceil() as usize
        }
        .max(2);
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            let num = format!("{:>width$}", i + 1, width = num_width);
            let gutter = format!(
                "{} {}",
                num.with(Color::DarkGrey),
                "│".with(self.color_theme.code_block_border)
            );
            result.push_str(&gutter);
            result.push(' ');
            result.push_str(line);
            result.push('\n');
        }
        result
    }

    fn push_text(
        &self,
        text: &str,
        state: &mut RenderState,
        output: &mut String,
        code_buffer: &mut String,
        in_code_block: bool,
    ) {
        if in_code_block {
            code_buffer.push_str(text);
            return;
        }
        if state.quote > 0 {
            // Re-prefix every line inside a blockquote with the gutter so
            // multi-line text keeps the │ visual instead of only the first line.
            let mut first = true;
            for part in text.split('\n') {
                if !first {
                    let _ = write!(output, "\n{} ", "│".with(self.color_theme.quote));
                }
                first = false;
                state.append_styled(output, part, &self.color_theme);
            }
        } else {
            state.append_styled(output, text, &self.color_theme);
        }
    }

    fn render_table(&self, table: &TableState) -> String {
        let mut rows = Vec::new();
        if !table.headers.is_empty() {
            rows.push(table.headers.clone());
        }
        rows.extend(table.rows.iter().cloned());

        if rows.is_empty() {
            return String::new();
        }

        let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        let desired_widths = (0..column_count)
            .map(|column| {
                rows.iter()
                    .filter_map(|row| row.get(column))
                    .map(|cell| visible_width(cell))
                    .max()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();

        let widths = self.fit_widths(desired_widths, column_count);

        let border = format!("{}", "│".with(self.color_theme.table_border));
        let separator = widths
            .iter()
            .map(|width| "─".repeat(*width + 2))
            .collect::<Vec<_>>()
            .join(&format!("{}", "│".with(self.color_theme.table_border)));
        let separator = format!("{border}{separator}{border}");

        let mut output = String::new();
        if !table.headers.is_empty() {
            output.push_str(&self
                .render_table_row(&table.headers, &widths, true, &table.alignments, false)
                .join("\n"));
            output.push('\n');
            output.push_str(&separator);
            if !table.rows.is_empty() {
                output.push('\n');
            }
        }

        for (index, row) in table.rows.iter().enumerate() {
            let alternate = index % 2 == 1;
            output.push_str(
                &self
                    .render_table_row(row, &widths, false, &table.alignments, alternate)
                    .join("\n"),
            );
            if index + 1 < table.rows.len() {
                output.push('\n');
            }
        }

        output
    }

    /// Scale column widths so the whole table fits within `self.max_width`
    /// terminal columns.  Columns are proportionally shrunk but always stay
    /// at least 3 characters wide.
    fn fit_widths(&self, desired: Vec<usize>, col_count: usize) -> Vec<usize> {
        let border_overhead = 1 + col_count * 3;
        let total_content: usize = desired.iter().sum();
        let total_needed = total_content + border_overhead;

        match self.max_width {
            Some(max) if total_needed > max => {
                let min_col_width = 3;
                let available = max.saturating_sub(border_overhead);
                let total_min = col_count * min_col_width;
                if total_min >= available {
                    return vec![min_col_width; col_count];
                }
                let extra = available - total_min;
                let mut scaled: Vec<usize> = desired
                    .iter()
                    .map(|&w| {
                        let proportion = (w as f64) / (total_content as f64);
                        let extra_for_col = (proportion * extra as f64) as usize;
                        min_col_width + extra_for_col
                    })
                    .collect();
                let sum: usize = scaled.iter().sum();
                if sum < available {
                    if let Some(max_col) = scaled.iter_mut().max() {
                        *max_col += available - sum;
                    }
                }
                scaled
            }
            _ => desired,
        }
    }

    fn render_table_row(
        &self,
        row: &[String],
        widths: &[usize],
        is_header: bool,
        alignments: &[Alignment],
        alternate: bool,
    ) -> Vec<String> {
        let border = format!("{}", "│".with(self.color_theme.table_border));

        // Wrap every cell to its column width so long content (which appears
        // when `max_width` constrains the table) stays inside the column
        // instead of overflowing the terminal.
        let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(widths.len());
        let mut max_lines = 1usize;
        for (index, width) in widths.iter().enumerate() {
            let cell = row.get(index).map_or("", String::as_str);
            let lines = wrap_ansi_text(cell, *width);
            max_lines = max_lines.max(lines.len());
            cell_lines.push(lines);
        }

        let mut rendered: Vec<String> = Vec::with_capacity(max_lines);
        for line_index in 0..max_lines {
            let mut line = String::new();
            line.push_str(&border);
            for (index, width) in widths.iter().enumerate() {
                let text = cell_lines[index].get(line_index).map_or("", String::as_str);
                let vis_width = visible_width(text);
                let padding = width.saturating_sub(vis_width);
                let align = alignments.get(index).copied().unwrap_or(Alignment::None);

                let (left_pad, right_pad) = match align {
                    Alignment::Right => (padding, 0),
                    Alignment::Center => (padding / 2, padding - padding / 2),
                    _ => (0, padding),
                };

                line.push(' ');
                line.push_str(&" ".repeat(left_pad));
                line.push_str(text);
                line.push_str(&" ".repeat(right_pad + 1));
                line.push_str(&border);
            }
            // Zebra striping via background tint: alternate rows get a subtle
            // dark background (background colour only — the border and text
            // foreground colours are left untouched), base rows stay on the
            // terminal's default background. The header is never tinted.
            if !is_header && alternate {
                rendered.push(apply_row_background(&line, self.color_theme.table_row_alt));
            } else {
                rendered.push(line);
            }
        }

        rendered
    }

    #[must_use]
    pub fn highlight_code(&self, code: &str, language: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut syntax_highlighter = HighlightLines::new(syntax, &self.syntax_theme);
        let mut colored_output = String::new();

        for line in LinesWithEndings::from(code) {
            match syntax_highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    colored_output.push_str(&apply_code_block_background(&escaped));
                }
                Err(_) => colored_output.push_str(&apply_code_block_background(line)),
            }
        }

        colored_output
    }

    pub fn stream_markdown(&self, markdown: &str, out: &mut impl Write) -> io::Result<()> {
        let rendered_markdown = self.markdown_to_ansi(markdown);
        write!(out, "{rendered_markdown}")?;
        if !rendered_markdown.ends_with('\n') {
            writeln!(out)?;
        }
        out.flush()
    }
}

// ---------------------------------------------------------------------------
// Reasoning / thinking-block render helpers
// ---------------------------------------------------------------------------

/// Word-wrap `reasoning` and return fully ANSI-styled lines with a `┃`
/// gutter.  Line 0 is the `Thinking:` / `Thought:` header; subsequent
/// lines are the body text dimmed to `body_color`.
fn render_reasoning_lines(
    reasoning: &str,
    width: usize,
    header_color: Color,
    body_color: Color,
    is_streaming: bool,
) -> Vec<String> {
    let label = if is_streaming { "Thinking:" } else { "Thought:" };
    let header_seq = color_to_ansi_fg(header_color);
    let body_seq = color_to_ansi_fg(body_color);
    let reset = "\x1b[0m";
    let gutter = format!("{header_seq}┃{reset} ");

    let mut output: Vec<String> = Vec::new();
    output.push(format!(
        "{header_seq}┃ \x1b[3m{label}{reset}"
    ));

    if reasoning.trim().is_empty() {
        return output;
    }

    let body_width = width.saturating_sub(2).max(1);

    for line in reasoning.split('\n') {
        if line.is_empty() {
            output.push(format!("{body_seq}┃{reset}"));
            continue;
        }
        for wrapped in wrap_reasoning_line(line, body_width) {
            output.push(format!("{body_seq}┃{reset} {wrapped}"));
        }
    }

    output
}

/// Word-wrap a single line into multiple lines each at most `width` columns.
fn wrap_reasoning_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_cols: usize = 0;

    for word in line.split_whitespace() {
        let word_cols = unicode_width::UnicodeWidthStr::width(word);
        if current.is_empty() {
            if word_cols > width {
                out.push(word.to_string());
                continue;
            }
            current.push_str(word);
            current_cols = word_cols;
        } else if current_cols + 1 + word_cols > width {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
            current_cols = word_cols;
        } else {
            current.push(' ');
            current.push_str(word);
            current_cols += 1 + word_cols;
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

/// ANSI true-colour foreground sequence for a [`Color`].
fn color_to_ansi_fg(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m"),
        Color::Black => "\x1b[30m".to_string(),
        Color::DarkGrey => "\x1b[90m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::DarkRed => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::DarkGreen => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::DarkYellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::DarkBlue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::DarkMagenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::DarkCyan => "\x1b[36m".to_string(),
        Color::White => "\x1b[97m".to_string(),
        Color::Grey => "\x1b[37m".to_string(),
        _ => "\x1b[37m".to_string(),
    }
}

/// Apply a background tint to a fully-rendered table row line.
///
/// Only the background colour is set — the cell text and border foreground
/// colours already present in the line are left untouched. Each reset
/// sequence is rewritten to re-apply the background so the tint survives
/// across styled spans.
fn apply_row_background(line: &str, color: Color) -> String {
    let bg_seq = color_to_ansi_bg(color);
    let reset = "\u{1b}[0m";
    let with_bg = line.replace(reset, &format!("{reset}{bg_seq}"));
    format!("{bg_seq}{with_bg}{reset}")
}

/// Return the ANSI 24-bit background sequence for a colour, falling back to
/// the terminal default background when the colour is not RGB.
fn color_to_ansi_bg(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("\x1b[48;2;{r};{g};{b}m"),
        Color::Black => "\x1b[40m".to_string(),
        Color::DarkGrey | Color::Grey => "\x1b[100m".to_string(),
        Color::Red | Color::DarkRed => "\x1b[41m".to_string(),
        Color::Green | Color::DarkGreen => "\x1b[42m".to_string(),
        Color::Yellow | Color::DarkYellow => "\x1b[43m".to_string(),
        Color::Blue | Color::DarkBlue => "\x1b[44m".to_string(),
        Color::Magenta | Color::DarkMagenta => "\x1b[45m".to_string(),
        Color::Cyan | Color::DarkCyan => "\x1b[46m".to_string(),
        Color::White => "\x1b[107m".to_string(),
        _ => "\x1b[49m".to_string(),
    }
}

/// Return the ANSI prefix for a streaming reasoning/thinking block.
///
/// Emits a dimmed, italic `▶ Thinking [` banner — compact and suitable for
/// inline streaming where the text accumulates character-by-character.
#[must_use]
pub fn reasoning_streaming_prefix(theme: &ColorTheme) -> String {
    let header_seq = color_to_ansi_fg(theme.reasoning_header);
    format!("{header_seq}\x1b[2m\x1b[3m▶ Thinking [")
}

/// Return the ANSI suffix that closes a streaming reasoning block.
#[must_use]
pub fn reasoning_streaming_suffix() -> &'static str {
    "]\x1b[0m\n"
}

/// Return a summary line for hidden or redacted reasoning blocks.
#[must_use]
pub fn reasoning_summary(
    char_count: Option<usize>,
    redacted: bool,
    theme: &ColorTheme,
) -> String {
    let header_seq = color_to_ansi_fg(theme.reasoning_header);
    if redacted {
        format!(
            "\n{header_seq}\x1b[2m\x1b[3m▶ Thinking block hidden by provider\x1b[0m\n"
        )
    } else if let Some(n) = char_count {
        format!("\n{header_seq}\x1b[2m\x1b[3m▶ Thinking ({n} chars hidden)\x1b[0m\n")
    } else {
        format!("\n{header_seq}\x1b[2m\x1b[3m▶ Thinking hidden\x1b[0m\n")
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MarkdownStreamState {
    pending: String,
}

impl MarkdownStreamState {
    #[must_use]
    pub fn push(&mut self, renderer: &TerminalRenderer, delta: &str) -> Option<String> {
        self.pending.push_str(delta);
        let split = find_stream_safe_boundary(&self.pending)?;
        let ready = self.pending[..split].to_string();
        self.pending.drain(..split);
        Some(renderer.render_markdown_streaming(&ready))
    }

    #[must_use]
    pub fn flush(&mut self, renderer: &TerminalRenderer) -> Option<String> {
        if self.pending.trim().is_empty() {
            self.pending.clear();
            None
        } else {
            let pending = std::mem::take(&mut self.pending);
            Some(renderer.render_markdown_streaming(&pending))
        }
    }
}

fn apply_code_block_background(line: &str) -> String {
    let trimmed = line.trim_end_matches('\n');
    let trailing_newline = if trimmed.len() == line.len() {
        ""
    } else {
        "\n"
    };
    let with_background = trimmed.replace("\u{1b}[0m", "\u{1b}[0;48;5;236m");
    format!("\u{1b}[48;5;236m{with_background}\u{1b}[0m{trailing_newline}")
}

/// Pre-process raw markdown so that fenced code blocks whose body contains
/// fence markers of equal or greater length are wrapped with a longer fence.
///
/// LLMs frequently emit triple-backtick code blocks that contain triple-backtick
/// examples.  `CommonMark` (and pulldown-cmark) treats the inner marker as the
/// closing fence, breaking the render.  This function detects the situation and
/// upgrades the outer fence to use enough backticks (or tildes) that the inner
/// markers become ordinary content.
#[allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::manual_repeat_n,
    clippy::manual_str_repeat
)]
fn normalize_nested_fences(markdown: &str) -> String {
    // A fence line is either "labeled" (has an info string, which is always an opener)
    // or "bare" (no info string, which could be opener or closer).
    #[derive(Debug, Clone)]
    struct FenceLine {
        char: char,
        len: usize,
        has_info: bool,
        indent: usize,
    }

    fn parse_fence_line(line: &str) -> Option<FenceLine> {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        let indent = trimmed.chars().take_while(|c| *c == ' ').count();
        if indent > 3 {
            return None;
        }
        let rest = &trimmed[indent..];
        let ch = rest.chars().next()?;
        if ch != '`' && ch != '~' {
            return None;
        }
        let len = rest.chars().take_while(|c| *c == ch).count();
        if len < 3 {
            return None;
        }
        let after = &rest[len..];
        if ch == '`' && after.contains('`') {
            return None;
        }
        let has_info = !after.trim().is_empty();
        Some(FenceLine {
            char: ch,
            len,
            has_info,
            indent,
        })
    }

    let lines: Vec<&str> = markdown.split_inclusive('\n').collect();
    // Handle final line that may lack trailing newline.
    // split_inclusive already keeps the original chunks, including a
    // final chunk without '\n' if the input doesn't end with one.

    // First pass: classify every line.
    let fence_info: Vec<Option<FenceLine>> = lines.iter().map(|l| parse_fence_line(l)).collect();

    // Second pass: pair openers with closers using a stack, recording
    // (opener_idx, closer_idx) pairs plus the max fence length found between
    // them.
    struct StackEntry {
        line_idx: usize,
        fence: FenceLine,
    }

    let mut stack: Vec<StackEntry> = Vec::new();
    // Paired blocks: (opener_line, closer_line, max_inner_fence_len)
    let mut pairs: Vec<(usize, usize, usize)> = Vec::new();

    for (i, fi) in fence_info.iter().enumerate() {
        let Some(fl) = fi else { continue };

        if fl.has_info {
            // Labeled fence, which is always an opener.
            stack.push(StackEntry {
                line_idx: i,
                fence: fl.clone(),
            });
        } else {
            // Bare fence, which tries to close the top of the stack if compatible.
            let closes_top = stack
                .last()
                .is_some_and(|top| top.fence.char == fl.char && fl.len >= top.fence.len);
            if closes_top {
                let opener = stack.pop().unwrap();
                // Find max fence length of any fence line strictly between
                // opener and closer (these are the nested fences).
                let inner_max = fence_info[opener.line_idx + 1..i]
                    .iter()
                    .filter_map(|fi| fi.as_ref().map(|f| f.len))
                    .max()
                    .unwrap_or(0);
                pairs.push((opener.line_idx, i, inner_max));
            } else {
                // Treat as opener.
                stack.push(StackEntry {
                    line_idx: i,
                    fence: fl.clone(),
                });
            }
        }
    }

    // Determine which lines need rewriting.  A pair needs rewriting when
    // its opener length <= max inner fence length.
    struct Rewrite {
        char: char,
        new_len: usize,
        indent: usize,
    }
    let mut rewrites: std::collections::HashMap<usize, Rewrite> = std::collections::HashMap::new();

    for (opener_idx, closer_idx, inner_max) in &pairs {
        let opener_fl = fence_info[*opener_idx].as_ref().unwrap();
        if opener_fl.len <= *inner_max {
            let new_len = inner_max + 1;
            let info_part = {
                let trimmed = lines[*opener_idx]
                    .trim_end_matches('\n')
                    .trim_end_matches('\r');
                let rest = &trimmed[opener_fl.indent..];
                rest[opener_fl.len..].to_string()
            };
            rewrites.insert(
                *opener_idx,
                Rewrite {
                    char: opener_fl.char,
                    new_len,
                    indent: opener_fl.indent,
                },
            );
            let closer_fl = fence_info[*closer_idx].as_ref().unwrap();
            rewrites.insert(
                *closer_idx,
                Rewrite {
                    char: closer_fl.char,
                    new_len,
                    indent: closer_fl.indent,
                },
            );
            // Store info string only in the opener; closer keeps the trailing
            // portion which is already handled through the original line.
            // Actually, we rebuild both lines from scratch below, including
            // the info string for the opener.
            let _ = info_part; // consumed in rebuild
        }
    }

    if rewrites.is_empty() {
        return markdown.to_string();
    }

    // Rebuild.
    let mut out = String::with_capacity(markdown.len() + rewrites.len() * 4);
    for (i, line) in lines.iter().enumerate() {
        if let Some(rw) = rewrites.get(&i) {
            let fence_str: String = std::iter::repeat(rw.char).take(rw.new_len).collect();
            let indent_str: String = std::iter::repeat(' ').take(rw.indent).collect();
            // Recover the original info string (if any) and trailing newline.
            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
            let fi = fence_info[i].as_ref().unwrap();
            let info = &trimmed[fi.indent + fi.len..];
            let trailing = &line[trimmed.len()..];
            out.push_str(&indent_str);
            out.push_str(&fence_str);
            out.push_str(info);
            out.push_str(trailing);
        } else {
            out.push_str(line);
        }
    }
    out
}

fn find_stream_safe_boundary(markdown: &str) -> Option<usize> {
    let mut open_fence: Option<FenceMarker> = None;
    let mut last_boundary = None;

    for (offset, line) in markdown.split_inclusive('\n').scan(0usize, |cursor, line| {
        let start = *cursor;
        *cursor += line.len();
        Some((start, line))
    }) {
        let line_without_newline = line.trim_end_matches('\n');
        if let Some(opener) = open_fence {
            if line_closes_fence(line_without_newline, opener) {
                open_fence = None;
                last_boundary = Some(offset + line.len());
            }
            continue;
        }

        if let Some(opener) = parse_fence_opener(line_without_newline) {
            open_fence = Some(opener);
            continue;
        }

        if line_without_newline.trim().is_empty() {
            last_boundary = Some(offset + line.len());
        }
    }

    last_boundary
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FenceMarker {
    character: char,
    length: usize,
}

fn parse_fence_opener(line: &str) -> Option<FenceMarker> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let character = rest.chars().next()?;
    if character != '`' && character != '~' {
        return None;
    }
    let length = rest.chars().take_while(|c| *c == character).count();
    if length < 3 {
        return None;
    }
    let info_string = &rest[length..];
    if character == '`' && info_string.contains('`') {
        return None;
    }
    Some(FenceMarker { character, length })
}

fn line_closes_fence(line: &str, opener: FenceMarker) -> bool {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > 3 {
        return false;
    }
    let rest = &line[indent..];
    let length = rest.chars().take_while(|c| *c == opener.character).count();
    if length < opener.length {
        return false;
    }
    rest[length..].chars().all(|c| c == ' ' || c == '\t')
}

fn visible_width(input: &str) -> usize {
    strip_ansi(input).width()
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            output.push(ch);
        }
    }

    output
}

/// Word-wrap plain (ANSI-free) text into lines of at most `width` visible
/// columns. Over-long single words are hard-broken and CJK wide characters
/// count as two columns.
fn wrap_plain_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for paragraph in text.split('\n') {
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        }
        for word in paragraph.split_whitespace() {
            let word_width = word.width();
            if current_width > 0 && current_width + 1 + word_width > width {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            if current_width == 0 && word_width > width {
                for chunk in split_word_by_width(word, width) {
                    if !current.is_empty() {
                        lines.push(std::mem::take(&mut current));
                    }
                    current.push_str(&chunk);
                    current_width = chunk.width();
                }
                continue;
            }
            if current_width > 0 {
                current.push(' ');
                current_width += 1;
            }
            current.push_str(word);
            current_width += word_width;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Break an over-long word into chunks of at most `width` visible columns.
fn split_word_by_width(word: &str, width: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for ch in word.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width > 0 && current_width + ch_width > width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Collect the leading ANSI escape sequences of `text` (every escape before
/// the first visible character) so styling can be re-applied after a wrap.
fn leading_ansi_prefix(text: &str) -> String {
    let mut prefix = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            break;
        }
        let mut seq = String::from('\u{1b}');
        if chars.peek() == Some(&'[') {
            chars.next();
            seq.push('[');
            for next in chars.by_ref() {
                seq.push(next);
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        }
        prefix.push_str(&seq);
    }
    prefix
}

/// Word-wrap text that may contain ANSI escape sequences into lines of at most
/// `width` visible columns. The leading ANSI styling is re-emitted at the
/// start of every wrapped line so whole-cell styling survives the wrap.
fn wrap_ansi_text(text: &str, width: usize) -> Vec<String> {
    let prefix = leading_ansi_prefix(text);
    let lines = wrap_plain_text(&strip_ansi(text), width);
    if prefix.is_empty() {
        return lines;
    }
    lines
        .into_iter()
        .map(|line| format!("{prefix}{line}\x1b[0m"))
        .collect()
}

/// Pre-process raw markdown so that `$...$` / `$$...$$` LaTeX fragments render
/// readably on a terminal.
///
/// pulldown-cmark's `ENABLE_MATH` extension surfaces math as an `InlineMath` /
/// `DisplayMath` event whose payload is the *raw* LaTeX source.  The renderer
/// prints that payload verbatim, so constructs such as `\text{energy}` or
/// `\frac{a}{b}` reach the terminal literally.  This pass rewrites only the
/// content between math delimiters into a Unicode approximation of the math
/// (unwrapping `\text{}` / `\mathrm{}`, converting `\frac{a}{b}` to a superscript
/// numerator over a fraction slash over a subscript denominator, promoting `^`
/// / `_` runs to Unicode super/sub-scripts, and mapping common symbols).  The
/// Private-use sentinel substituted for `|` inside math/code spans before
/// markdown parsing so that pulldown-cmark's table parser does not split
/// cells on pipes inside formulas.  Restored to `|` after rendering.
const MATH_PIPE_SENTINEL: char = '\u{e000}';

/// Escape raw `|` characters that appear inside inline/display math
/// (`$...$`, `$$...$$`) or inline code (`` `...` ``) spans so the markdown
/// table parser does not treat them as column separators.  A `|` that is
/// already escaped (`\|`) is left untouched.  Structural `|` outside such
/// spans (the actual table dividers) is preserved.
fn escape_pipes_in_spans(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();
    let mut in_code = false;
    let mut math_depth = 0usize; // 0 = none, 1 = inline `$`, 2 = display `$$`
    let mut prev_backslash = false;
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                in_code = !in_code;
                prev_backslash = false;
                out.push(c);
            }
            '$' => {
                prev_backslash = false;
                if math_depth == 0 {
                    if chars.peek() == Some(&'$') {
                        chars.next();
                        math_depth = 2;
                        out.push_str("$$");
                    } else {
                        math_depth = 1;
                        out.push('$');
                    }
                } else if math_depth == 2 {
                    if chars.peek() == Some(&'$') {
                        chars.next();
                        math_depth = 0;
                        out.push_str("$$");
                    } else {
                        out.push('$');
                    }
                } else {
                    math_depth = 0;
                    out.push('$');
                }
            }
            '|' => {
                if (in_code || math_depth > 0) && !prev_backslash {
                    out.push(MATH_PIPE_SENTINEL);
                } else {
                    out.push('|');
                }
                prev_backslash = false;
            }
            '\\' => {
                prev_backslash = true;
                out.push(c);
            }
            _ => {
                prev_backslash = false;
                out.push(c);
            }
        }
    }
    out
}

/// Convert `&lt;tool_call&gt;...&lt;/tool_call&gt;` blocks embedded in model output into
/// Markdown fragments so the rest of the rendering pipeline applies syntax
/// highlighting and styling automatically.
///
/// This does **not** strip the XML — it rewrites recognised tags into
/// formatted Markdown that renders inline with the surrounding text.
pub(crate) fn preprocess_tool_call_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let start = rest.find("<tool_call>");
        if start.is_none() {
            out.push_str(rest);
            break;
        }
        let start = start.unwrap();
        out.push_str(&rest[..start]);
        let after_start = &rest[start + "<tool_call>".len()..];
        let end = after_start.find("</tool_call>");
        if end.is_none() {
            break;
        }
        let inner = &after_start[..end.unwrap()];
        out.push_str(&tool_call_to_markdown(inner));
        rest = &after_start[end.unwrap() + "</tool_call>".len()..];
    }
    // Second pass: handle <invoke tool="Name">...</invoke> format
    let mut out2 = String::with_capacity(out.len());
    let mut rest2 = &out[..];
    loop {
        let Some(start) = rest2.find("<invoke tool=\"") else {
            out2.push_str(rest2);
            break;
        };
        out2.push_str(&rest2[..start]);
        let after_open = &rest2[start + "<invoke tool=\"".len()..];
        let Some(quote_end) = after_open.find('"') else {
            out2.push_str(&rest2[..start + 1]);
            rest2 = &rest2[start + 1..];
            continue;
        };
        let tool_name = after_open[..quote_end].trim();
        let body_start = start + "<invoke tool=\"".len() + quote_end + 1;
        let Some(close_rel) = rest2[body_start..].find("</invoke>") else {
            // Partial invoke tag — text before <invoke> was already
            // pushed; discard the unclosed tag.
            break;
        };
        let body = &rest2[body_start..body_start + close_rel];
        out2.push_str(&invoke_tool_call_to_markdown(tool_name, body));
        rest2 = &rest2[body_start + close_rel + "</invoke>".len()..];
    }
    out2
}

/// Parse the body of a single `<tool_call>` tag and produce a human-readable
/// Markdown snippet.
fn tool_call_to_markdown(raw: &str) -> String {
    let text = raw.trim();
    let lines: Vec<&str> = text.lines().collect();
    let mut func_name = String::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut current_param = String::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("<function=") && trimmed.ends_with('>') {
            func_name = trimmed["<function=".len()..trimmed.len() - 1]
                .trim()
                .to_string();
        } else if trimmed.starts_with("<parameter=") && trimmed.ends_with('>') {
            current_param = trimmed["<parameter=".len()..trimmed.len() - 1]
                .trim()
                .to_string();
        } else if trimmed == "</function>" || trimmed == "</parameter>" {
            // ignore closing tags
        } else if !current_param.is_empty() {
            let value = trimmed.to_string();
            params.push((current_param.clone(), value));
            current_param.clear();
        }
    }

    if func_name.is_empty() {
        return String::new();
    }

    let mut md = format!("**`{}`**\n", func_name);
    for (k, v) in params {
        md.push_str(&format!("*`{}`*: `{}`\n", k, v));
    }
    md
}

fn invoke_tool_call_to_markdown(tool_name: &str, body: &str) -> String {
    let params = runtime::thinking::extract::parse_invoke_parameters(body);

    let mut md = format!("**`{}`**\n", tool_name);
    for (k, v) in params {
        md.push_str(&format!("*`{}`*: `{}`\n", k, v));
    }
    md
}

/// `InlineMath` / `DisplayMath` handlers then emit this readable text.  Code-
/// fenced regions are skipped so that literal `$` inside source is never
/// mangled.
fn degrade_latex(theme: &ColorTheme, markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut idx = 0;
    let len = markdown.len();
    let mut in_code = false;
    let mut line_start = true;

    while idx < len {
        let rest = &markdown[idx..];
        let ch = rest.chars().next().unwrap();

        if line_start {
            let trimmed = rest.trim_start_matches(' ');
            let first = trimmed.chars().next();
            if let Some(fence_ch) = first {
                if fence_ch == '`' || fence_ch == '~' {
                    let run = trimmed.chars().take_while(|c| *c == fence_ch).count();
                    if run >= 3 {
                        in_code = !in_code;
                    }
                }
            }
            line_start = false;
        }
        if ch == '\n' {
            line_start = true;
        }

        if ch == '$' && !in_code {
            // An escaped dollar is literal: leave it untouched.
            if idx > 0 && markdown.as_bytes()[idx - 1] == b'\\' {
                out.push('$');
                idx += 1;
                continue;
            }
            let is_display = idx + 1 < len && markdown[idx + 1..].starts_with('$');
            let start = if is_display { idx + 2 } else { idx + 1 };
            let pat = if is_display { "$$" } else { "$" };
            if let Some(rel) = markdown[start..].find(pat) {
                let end = start + rel;
                let tail = end + pat.len();
                let content = &markdown[start..end];
                let transform =
                    content.contains('\\') || content.contains('^') || content.contains('_');
                if is_display {
                    // Display math becomes a vertical (stacked) block so that
                    // `\frac` reads as a real fraction; the `$$` delimiters are
                    // dropped because the block itself is the visual frame.
                    if transform {
                        out.push_str(&render_latex_vertical(content, theme));
                    } else {
                        out.push_str(content);
                    }
                } else {
                    out.push('$');
                    if transform {
                        out.push_str(&render_latex_unicode(content));
                    } else {
                        out.push_str(content);
                    }
                    out.push('$');
                }
                idx = tail;
                continue;
            }
            out.push('$');
            idx += 1;
            continue;
        }

        out.push(ch);
        idx += ch.len_utf8();
    }

    out
}

/// Rewrite a single LaTeX fragment into a Unicode approximation of the math.
/// Superscript/subscript runs are promoted to Unicode modifier characters and
/// `\frac` becomes a superscript numerator over a fraction slash (U+2044) over a
/// subscript denominator, so the result reads like typeset math on a terminal.
fn render_latex_unicode(latex: &str) -> String {
    let mut out = String::with_capacity(latex.len());
    let mut chars = latex.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Read the command name without consuming the delimiter that
            // follows it.  `take_while` would eat the first non-matching char
            // (e.g. the `{` of `\text{...}`), so peek and advance manually.
            let mut cmd = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_alphabetic() {
                    cmd.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if !cmd.is_empty() {
                match cmd.as_str() {
                    "frac" => {
                        let num =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        let den =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));

                        if let (Some(num_text), Some(den_text)) = (num, den) {
                            let num_trimmed = num_text.trim();
                            let den_trimmed = den_text.trim();
                            let is_diff = (num_trimmed == "d" || num_trimmed == "\\mathrm{d}")
                                && den_trimmed.starts_with('d');

                            if is_diff {
                                // Differential fraction: d/dx instead of ᵈ⁄ₓ
                                out.push_str("d/");
                                out.push_str(&render_latex_unicode(den_trimmed));
                            } else {
                                let num_rendered = render_latex_unicode(&num_text);
                                let den_rendered = render_latex_unicode(&den_text);
                                out.push_str(&num_rendered);
                                out.push('/');
                                out.push_str(&den_rendered);
                            }
                        } else {
                            out.push_str("frac");
                        }
                    }
                    "sqrt" => {
                        let opt_arg = read_optional_group(&mut chars);
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            if let Some(index) = opt_arg {
                                out.push_str(&to_superscript(&render_latex_unicode(&index)));
                            }
                            out.push('\u{221a}');
                            out.push_str(&render_latex_unicode(&inner));
                        } else {
                            out.push_str("sqrt");
                        }
                    }
                    "vec" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{20d7}');
                        } else {
                            out.push_str("vec");
                        }
                    }
                    "dot" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{0307}');
                        } else {
                            out.push_str("dot");
                        }
                    }
                    "ddot" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{0308}');
                            out.push('\u{0308}');
                        } else {
                            out.push_str("ddot");
                        }
                    }
                    "hat" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{0302}');
                        } else {
                            out.push_str("hat");
                        }
                    }
                    "bar" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{0304}');
                        } else {
                            out.push_str("bar");
                        }
                    }
                    "tilde" => {
                        let inner =
                            read_group(&mut chars).or_else(|| read_single_or_group(&mut chars));
                        if let Some(inner) = inner {
                            out.push_str(&render_latex_unicode(&inner));
                            out.push('\u{0303}');
                        } else {
                            out.push_str("tilde");
                        }
                    }
                    "text" | "mathrm" | "mathbf" | "mathit" | "textbf" | "textrm"
                    | "operatorname" | "texttt" | "mathsf" | "boldsymbol"
                    | "mathbb" | "mathcal" => {
                        if let Some(inner) = read_group(&mut chars) {
                            out.push_str(&render_latex_unicode(&inner));
                        }
                    }
                    other => {
                        if let Some(sym) = latex_symbol(other) {
                            out.push_str(sym);
                        } else {
                            out.push_str(other);
                        }
                    }
                }
            } else if let Some(nc) = chars.next() {
                // Backslash followed by a non-letter: spacing/escaping commands.
                match nc {
                    ',' | ';' | ':' | ' ' | '!' | '>' | '<' | '\'' => out.push(' '),
                    '|' => {
                        // Preserve escaped pipes so the markdown table parser
                        // does not treat `\|` (e.g. inside `$\ln|x|$`) as a
                        // column separator.
                        out.push('\\');
                        out.push('|');
                    }
                    _ => out.push(nc),
                }
            }
        } else if c == '^' || c == '_' {
            // Promote the following group (or single char) to Unicode super/sub.
            // When the content contains characters that lack a dedicated glyph,
            // fall back to explicit `^{...}` / `_{...}` notation so the display
            // does not mix modifier and plain characters (e.g. `h→₀` for `h→0`).
            let content = if chars.peek() == Some(&'{') {
                read_group(&mut chars).unwrap_or_default()
            } else {
                chars.next().map(String::from).unwrap_or_default()
            };
            let rendered = render_latex_unicode(&content);
            let all_convertible = |s: &str, superscript: bool| -> bool {
                s.chars().all(|ch| match (superscript, ch) {
                    (true, '0'..='9' | '+' | '-' | '=' | '(' | ')' | 'a' | 'n') => true,
                    (false, '0'..='9' | '+' | '-' | '=' | '(' | ')') => true,
                    _ => false,
                })
            };
            if rendered.chars().count() == 1 || all_convertible(&rendered, c == '^') {
                if c == '^' {
                    out.push_str(&to_superscript(&rendered));
                } else {
                    out.push_str(&to_subscript(&rendered));
                }
            } else {
                out.push(if c == '^' { '^' } else { '_' });
                out.push('{');
                out.push_str(&rendered);
                out.push('}');
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// A laid-out math expression as a stack of equal-width text rows.  `rows`
/// always share the same display width (monospace assumption, one cell per
/// `char`); visual alignment of fractions and surrounding text happens by
/// lining up the middle (baseline) row.
struct MathBlock {
    rows: Vec<String>,
}

impl MathBlock {
    fn width(&self) -> usize {
        self.rows.iter().map(|r| col_width(r)).max().unwrap_or(0)
    }
}

/// Render a LaTeX fragment as a possibly multi-row terminal block.  `\frac`
/// becomes a vertical fraction (numerator / rule / denominator); every other
/// construct is rendered inline via [`render_latex_unicode`] and stays on a
/// single row.  Nested `\frac` (in a numerator or denominator) recurses.
fn render_latex_vertical(latex: &str, theme: &ColorTheme) -> String {
    let block = layout_math(latex, theme);
    block.rows.join("\n")
}

fn layout_math(expr: &str, theme: &ColorTheme) -> MathBlock {
    let mut blocks: Vec<MathBlock> = Vec::new();
    let mut buf = String::new();
    let mut chars = expr.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut cmd = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_alphabetic() {
                    cmd.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if cmd == "frac" {
                if !buf.is_empty() {
                    blocks.push(leaf_block(&buf));
                    buf.clear();
                }
                let num = read_group(&mut chars).unwrap_or_default();
                let den = read_group(&mut chars).unwrap_or_default();
                blocks.push(frac_block(&num, &den, theme));
            } else {
                // Non-frac command stays in the leaf so render_latex_unicode
                // can handle it (e.g. \sqrt, \sum, symbols).
                buf.push('\\');
                buf.push_str(&cmd);
            }
        } else if c == '{' || c == '}' {
            // Stray braces outside \frac: treat as literal text.
            buf.push(c);
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        blocks.push(leaf_block(&buf));
    }
    match blocks.into_iter().reduce(|acc, b| side_by_side(&acc, &b)) {
        Some(block) => block,
        None => MathBlock {
            rows: vec![String::new()],
        },
    }
}

/// A single-line run of (inline-rendered) text.
fn leaf_block(s: &str) -> MathBlock {
    MathBlock {
        rows: vec![render_latex_unicode(s)],
    }
}

/// Build a vertical fraction: numerator rows, a rule, denominator rows — all
/// centered to the same width.
fn frac_block(num: &str, den: &str, theme: &ColorTheme) -> MathBlock {
    let nb = layout_math(num, theme);
    let db = layout_math(den, theme);
    let w = nb.width().max(db.width());
    let mut rows: Vec<String> = nb
        .rows
        .iter()
        .map(|r| format!("{}", center_pad(r, w).with(theme.heading)))
        .collect();
    rows.push(format!("{}", "─".repeat(w).with(theme.math_fraction)));
    rows.extend(
        db.rows
            .iter()
            .map(|r| format!("{}", center_pad(r, w).with(theme.link))),
    );
    MathBlock { rows }
}

/// Place two blocks side by side, aligning their middle (baseline) rows.  The
/// left block keeps its width; the right block starts at the next column.
fn side_by_side(a: &MathBlock, b: &MathBlock) -> MathBlock {
    let ha = a.rows.len();
    let hb = b.rows.len();
    let mid_a = ha / 2;
    let mid_b = hb / 2;
    let top = mid_b.saturating_sub(mid_a);
    let below = (hb - 1 - mid_b).saturating_sub(ha - 1 - mid_a);
    let h = top + ha + below;
    let wa = a.width();
    let wb = b.width();
    let mut rows = vec![String::new(); h];
    for i in 0..ha {
        rows[top + i] = a.rows[i].clone();
    }
    let b_abs_mid = top + mid_a;
    let b_top = b_abs_mid - mid_b;
    for i in 0..hb {
        let target = b_top + i;
        let b_row = &b.rows[i];
        if rows[target].is_empty() {
            rows[target] = format!("{}{}", "\u{00A0}".repeat(wa), b_row);
        } else {
            let current_width = col_width(&rows[target]);
            let pad = wa.saturating_sub(current_width);
            let mut line = std::mem::take(&mut rows[target]);
            line.push_str(&" ".repeat(pad));
            line.push_str(b_row);
            rows[target] = line;
        }
    }
    let total = wa + wb;
    for r in &mut rows {
        let pad = total.saturating_sub(col_width(r));
        if pad > 0 {
            if r.is_empty() {
                r.push_str(&"\u{00A0}".repeat(pad));
            } else {
                r.push_str(&" ".repeat(pad));
            }
        }
    }
    MathBlock { rows }
}

/// Display width in terminal columns (wraps `unicode_width::UnicodeWidthStr`).
fn col_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Center `s` within `w` cells using spaces (monospace assumption).
/// Left padding uses non-breaking spaces so pulldown-cmark does not
/// misinterpret centered math rows as indented code blocks.
fn center_pad(s: &str, w: usize) -> String {
    let cw = col_width(s);
    if cw >= w {
        return s.to_string();
    }
    let left = (w - cw) / 2;
    let right = w - cw - left;
    format!("{}{}{}", "\u{00A0}".repeat(left), s, " ".repeat(right))
}

/// Convert `<tool_call>...</tool_call>` blocks embedded in text into ANSI-
/// formatted tool call representations for direct terminal output (skips the
/// Markdown pipeline).  Used by the streaming thinking-delta path so raw XML
/// never flashes on screen.
pub(crate) fn format_tool_calls_ansi(text: &str, theme: &ColorTheme) -> String {
    use crossterm::style::Stylize;
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let start = rest.find("<tool_call>");
        if start.is_none() {
            out.push_str(rest);
            break;
        }
        let start = start.unwrap();
        out.push_str(&rest[..start]);
        let after_start = &rest[start + "<tool_call>".len()..];
        let end = after_start.find("</tool_call>");
        if end.is_none() {
            break;
        }
        let inner = &after_start[..end.unwrap()];
        out.push_str(&format_single_tool_call_ansi(inner, theme));
        rest = &after_start[end.unwrap() + "</tool_call>".len()..];
    }
    // Second pass: handle <invoke tool="Name">...</invoke> format
    let mut rest2 = &out[..];
    let mut out2 = String::with_capacity(out.len());
    loop {
        let Some(start) = rest2.find("<invoke tool=\"") else {
            out2.push_str(rest2);
            break;
        };
        out2.push_str(&rest2[..start]);
        let after_open = &rest2[start + "<invoke tool=\"".len()..];
        let Some(quote_end) = after_open.find('"') else {
            out2.push_str(&rest2[..start + 1]);
            rest2 = &rest2[start + 1..];
            continue;
        };
        let tool_name = after_open[..quote_end].trim();
        let body_start = start + "<invoke tool=\"".len() + quote_end + 1;
        let Some(close_rel) = rest2[body_start..].find("</invoke>") else {
            // Partial invoke tag — text before <invoke> was already
            // pushed; discard the unclosed tag.
            break;
        };
        let close_end = body_start + close_rel + "</invoke>".len();
        let body = &rest2[body_start..body_start + close_rel];
        out2.push_str(&format_invoke_tool_call_ansi(tool_name, body, theme));
        rest2 = &rest2[close_end..];
    }
    out2
}

fn format_single_tool_call_ansi(raw: &str, theme: &ColorTheme) -> String {
    use crossterm::style::Stylize;
    let text = raw.trim();
    let lines: Vec<&str> = text.lines().collect();
    let mut func_name = String::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut current_param = String::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("<function=") && trimmed.ends_with('>') {
            func_name = trimmed["<function=".len()..trimmed.len() - 1]
                .trim()
                .to_string();
        } else if trimmed.starts_with("<parameter=") && trimmed.ends_with('>') {
            current_param = trimmed["<parameter=".len()..trimmed.len() - 1]
                .trim()
                .to_string();
        } else if trimmed == "</function>" || trimmed == "</parameter>" {
        } else if !current_param.is_empty() {
            params.push((current_param.clone(), trimmed.to_string()));
            current_param.clear();
        }
    }

    if func_name.is_empty() {
        return String::new();
    }

    let mut s = String::new();
    s.push_str(&format!(
        "{}\n",
        format_args!(
            "{} {}",
            "⚙".with(theme.spinner_active),
            func_name.bold().with(theme.inline_code)
        )
    ));
    for (k, v) in params {
        s.push_str(&format!("  {} {}\n", k.with(theme.emphasis), v));
    }
    s
}

fn format_invoke_tool_call_ansi(tool_name: &str, body: &str, theme: &ColorTheme) -> String {
    use crossterm::style::Stylize;
    let params = runtime::thinking::extract::parse_invoke_parameters(body);

    let mut s = format!(
        "{}\n",
        format_args!(
            "{} {}",
            "⚙".with(theme.spinner_active),
            tool_name.bold().with(theme.inline_code)
        )
    );
    for (k, v) in params {
        s.push_str(&format!("  {} {}\n", k.with(theme.emphasis), v));
    }
    s
}

/// Map an already-rendered math string to Unicode superscripts.  Characters
/// without a superscript glyph pass through unchanged.
fn to_superscript(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '\u{2070}',
            '1' => '\u{00b9}',
            '2' => '\u{00b2}',
            '3' => '\u{00b3}',
            '4' => '\u{2074}',
            '5' => '\u{2075}',
            '6' => '\u{2076}',
            '7' => '\u{2077}',
            '8' => '\u{2078}',
            '9' => '\u{2079}',
            'a' => '\u{1d43}',
            'b' => '\u{1d47}',
            'c' => '\u{1d9c}',
            'd' => '\u{1d48}',
            'e' => '\u{1d49}',
            'f' => '\u{1da0}',
            'g' => '\u{1d4d}',
            'h' => '\u{02b0}',
            'i' => '\u{2071}',
            'j' => '\u{02b2}',
            'k' => '\u{1d4f}',
            'l' => '\u{02e1}',
            'm' => '\u{1d50}',
            'n' => '\u{207f}',
            'o' => '\u{1d52}',
            'p' => '\u{1d56}',
            'q' => '\u{02e0}',
            'r' => '\u{02b3}',
            's' => '\u{02e2}',
            't' => '\u{1d57}',
            'u' => '\u{1d58}',
            'v' => '\u{1d5b}',
            'w' => '\u{02b7}',
            'x' => '\u{02e3}',
            'y' => '\u{02b8}',
            'z' => '\u{1dbb}',
            '+' => '\u{207a}',
            '-' => '\u{207b}',
            '=' => '\u{207c}',
            '(' => '\u{207d}',
            ')' => '\u{207e}',
            other => other,
        })
        .collect()
}

/// Map an already-rendered math string to Unicode subscripts.  Characters
/// without a subscript glyph pass through unchanged.
fn to_subscript(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '\u{2080}',
            '1' => '\u{2081}',
            '2' => '\u{2082}',
            '3' => '\u{2083}',
            '4' => '\u{2084}',
            '5' => '\u{2085}',
            '6' => '\u{2086}',
            '7' => '\u{2087}',
            '8' => '\u{2088}',
            '9' => '\u{2089}',
            'a' => '\u{2090}',
            'e' => '\u{2091}',
            'h' => '\u{2095}',
            'i' => '\u{1d62}',
            'j' => '\u{2c7c}',
            'k' => '\u{2096}',
            'l' => '\u{2097}',
            'm' => '\u{2098}',
            'n' => '\u{2099}',
            'o' => '\u{2092}',
            'p' => '\u{209a}',
            'r' => '\u{1d63}',
            's' => '\u{209b}',
            't' => '\u{209c}',
            'u' => '\u{1d64}',
            'v' => '\u{1d65}',
            'x' => '\u{2093}',
            '+' => '\u{208a}',
            '-' => '\u{208b}',
            '=' => '\u{208c}',
            '(' => '\u{208d}',
            ')' => '\u{208e}',
            other => other,
        })
        .collect()
}
fn read_group<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<String> {
    if chars.peek() != Some(&'{') {
        return None;
    }
    chars.next();
    let mut depth = 1usize;
    let mut buf = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                depth += 1;
                buf.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                buf.push(ch);
            }
            _ => buf.push(ch),
        }
    }
    Some(buf)
}

/// Read a `{...}` group, or, when no brace follows the command, skip leading
/// spaces and take the next single character as the argument.  This keeps
/// braceless forms like `\dot x` from swallowing the space as the operand.
fn read_single_or_group<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Option<String> {
    if chars.peek() == Some(&'{') {
        return read_group(chars);
    }
    while chars.peek() == Some(&' ') {
        chars.next();
    }
    chars.next().map(String::from)
}

/// Read an optional `[...]` group (e.g. `\sqrt[3]{x}`), returning the interior
/// text or [`None`] when no `[` follows.
fn read_optional_group<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Option<String> {
    if chars.peek() != Some(&'[') {
        return None;
    }
    chars.next();
    let mut buf = String::new();
    let mut depth = 1usize;
    while let Some(ch) = chars.next() {
        match ch {
            '[' => {
                depth += 1;
                buf.push(ch);
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                buf.push(ch);
            }
            _ => buf.push(ch),
        }
    }
    Some(buf)
}

/// Common LaTeX command → ASCII/Unicode substitutions.  Empty strings drop the
/// command entirely (e.g. `\left`, `\right` sizing markers).

static LATEX_SYMBOLS: phf::Map<&'static str, &'static str> = phf_map! {
    "alpha" => "α",
    "beta" => "β",
    "gamma" => "γ",
    "delta" => "δ",
    "epsilon" => "ε",
    "zeta" => "ζ",
    "eta" => "η",
    "theta" => "θ",
    "kappa" => "κ",
    "lambda" => "λ",
    "mu" => "μ",
    "nu" => "ν",
    "xi" => "ξ",
    "pi" => "π",
    "rho" => "ρ",
    "sigma" => "σ",
    "tau" => "τ",
    "phi" => "φ",
    "chi" => "χ",
    "psi" => "ψ",
    "omega" => "ω",
    "Gamma" => "Γ",
    "Delta" => "Δ",
    "Theta" => "Θ",
    "Lambda" => "Λ",
    "Xi" => "Ξ",
    "Pi" => "Π",
    "Sigma" => "Σ",
    "Phi" => "Φ",
    "Psi" => "Ψ",
    "Omega" => "Ω",
    "leq" => "<=",
    "le" => "<=",
    "geq" => ">=",
    "ge" => ">=",
    "neq" => "!=",
    "approx" => "~=",
    "equiv" => "==",
    "sim" => "~",
    "propto" => "∝",
    "times" => "×",
    "cdot" => "·",
    "div" => "÷",
    "pm" => "±",
    "mp" => "∓",
    "ast" => "*",
    "star" => "*",
    "to" => "→",
    "rightarrow" => "→",
    "leftarrow" => "←",
    "Rightarrow" => "⇒",
    "Leftarrow" => "⇐",
    "leftrightarrow" => "↔",
    "Leftrightarrow" => "⇔",
    "mapsto" => "↦",
    "infty" => "∞",
    "nabla" => "∇",
    "partial" => "∂",
    "sum" => "∑",
    "prod" => "∏",
    "int" => "∫",
    "oint" => "∮",
    "in" => "∈",
    "notin" => "∉",
    "subset" => "⊂",
    "supset" => "⊃",
    "subseteq" => "⊆",
    "supseteq" => "⊇",
    "cup" => "∪",
    "cap" => "∩",
    "emptyset" => "∅",
    "forall" => "∀",
    "exists" => "∃",
    "nexists" => "∄",
    "neg" => "¬",
    "land" => "∧",
    "lor" => "∨",
    "wedge" => "∧",
    "vee" => "∨",
    "oplus" => "⊕",
    "otimes" => "⊗",
    "angle" => "∠",
    "perp" => "⊥",
    "parallel" => "∥",
    "varepsilon" => "ε",
    "prime" => "'",
    "circ" => "°",
    "deg" => "°",
    "left" => "",
    "right" => "",
    "bigl" => "",
    "bigr" => "",
    "Bigl" => "",
    "Bigr" => "",
    "big" => "",
    "Big" => "",
    "bigg" => "",
    "Bigg" => "",
    "quad" => "  ",
    "qquad" => "    ",
    "mathrm" => "",
    "triangle" => "\u{25b3}",
    "ell" => "\u{2113}",
    "hbar" => "\u{210f}",
    "Re" => "\u{211c}",
    "Im" => "\u{2111}",
    "mathbb" => "",
    "mathcal" => "",
    "sin" => "sin ",
    "cos" => "cos ",
    "tan" => "tan ",
    "log" => "log ",
    "ln" => "ln ",
    "exp" => "exp ",
    "lim" => "lim ",
    "max" => "max ",
    "min" => "min ",
    "gcd" => "gcd ",
    "lcm" => "lcm ",
    "longrightarrow" => "→",
    "Longrightarrow" => "⇒",
    "longleftarrow" => "←",
    "Longleftarrow" => "⇐",
    "iff" => "⇔",
    "ll" => "\u{226a}",
    "gg" => "\u{226b}",
    "simeq" => "\u{2243}",
    "cong" => "\u{2245}",
    "doteq" => "\u{2250}",
};

fn latex_symbol(cmd: &str) -> Option<&'static str> {
    LATEX_SYMBOLS.get(cmd).copied()
}

/// Pre-process raw markdown so that a code-fenced block the model left open
/// (no closing fence) still renders as a closed `╭─╰─` box instead of a
/// dangling `╭─` at end-of-stream.
///
/// Streaming keeps an open code fence intact inside `pending` until its closer
/// arrives, so this only bites when the final flush truncates mid-block.  The
/// same truncation can occur in a non-streamed final render, so this runs on
/// every pass.  We replicate pulldown-cmark's fence matching: a labeled fence
/// always opens, a bare fence of length, the open fence closes it.
fn close_dangling_fence(markdown: &str) -> String {
    fn fence_line(line: &str) -> Option<(char, usize, bool)> {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let indent = trimmed.chars().take_while(|c| *c == ' ').count();
        if indent > 3 {
            return None;
        }
        let rest = &trimmed[indent..];
        let ch = rest.chars().next()?;
        if ch != '`' && ch != '~' {
            return None;
        }
        let len = rest.chars().take_while(|c| *c == ch).count();
        if len < 3 {
            return None;
        }
        let after = &rest[len..];
        if ch == '`' && after.contains('`') {
            return None;
        }
        let has_info = !after.trim().is_empty();
        Some((ch, len, has_info))
    }

    let mut stack: Vec<(char, usize)> = Vec::new();
    for line in markdown.split_inclusive('\n') {
        if let Some((ch, len, has_info)) = fence_line(line) {
            if !has_info {
                if let Some(&(tch, tlen)) = stack.last() {
                    if tch == ch && len >= tlen {
                        stack.pop();
                        continue;
                    }
                }
            }
            stack.push((ch, len));
        }
    }

    if stack.is_empty() {
        return markdown.to_string();
    }

    let mut out = markdown.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    for (ch, len) in &stack {
        let fence: String = std::iter::repeat(*ch).take(*len).collect();
        out.push_str(&fence);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        close_dangling_fence, color_to_ansi_bg, color_to_ansi_fg, degrade_latex, escape_pipes_in_spans,
        strip_ansi, visible_width, wrap_plain_text, ColorTheme, MarkdownStreamState, Spinner,
        TerminalRenderer,
    };

    #[test]
    fn renders_markdown_with_styling_and_lists() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer
            .render_markdown("# Heading\n\nThis is **bold** and *italic*.\n\n- item\n\n`code`");

        assert!(markdown_output.contains("Heading"));
        assert!(markdown_output.contains("• item"));
        assert!(markdown_output.contains("code"));
        assert!(markdown_output.contains('\u{1b}'));
    }

    #[test]
    fn renders_links_as_colored_markdown_labels() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.render_markdown("See [Claw](https://example.com/docs) now.");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("[Claw](https://example.com/docs)"));
        assert!(markdown_output.contains('\u{1b}'));
    }

    #[test]
    fn highlights_fenced_code_blocks() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.markdown_to_ansi("```rust\nfn hi() { println!(\"hi\"); }\n```");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("╭─ rust"));
        assert!(plain_text.contains("fn hi"));
        assert!(markdown_output.contains('\u{1b}'));
        assert!(markdown_output.contains("[48;5;236m"));
    }

    #[test]
    fn renders_ordered_and_nested_lists() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.render_markdown("1. first\n2. second\n   - nested\n   - child");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("1. first"));
        assert!(plain_text.contains("2. second"));
        assert!(plain_text.contains("  ◦ nested"));
        assert!(plain_text.contains("  ◦ child"));
    }

    #[test]
    fn renders_tables_with_alignment() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer
            .render_markdown("| Name | Value |\n| ---- | ----- |\n| alpha | 1 |\n| beta | 22 |");
        let plain_text = strip_ansi(&markdown_output);
        let lines = plain_text.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "│ Name  │ Value │");
        assert_eq!(lines[1], "│───────│───────│");
        assert_eq!(lines[2], "│ alpha │ 1     │");
        assert_eq!(lines[3], "│ beta  │ 22    │");
        assert!(markdown_output.contains('\u{1b}'));
    }

    #[test]
    fn renders_tables_with_right_center_alignment() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer.render_markdown(
            "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |\n| alpha | beta | gamma |",
        );
        let plain_text = strip_ansi(&markdown_output);
        let lines = plain_text.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "│ Left  │ Center │ Right │");
        assert_eq!(lines[1], "│───────│────────│───────│");
        assert_eq!(lines[2], "│ a     │   b    │     c │");
        assert_eq!(lines[3], "│ alpha │  beta  │ gamma │");
        assert!(markdown_output.contains('\u{1b}'));
    }

    #[test]
    fn renders_table_rows_with_alternating_font_color() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output = terminal_renderer.render_markdown(
            "| Name | Value |\n| ---- | ----- |\n| alpha | 1 |\n| beta | 22 |\n| gamma | 333 |",
        );
        let alt_seq = color_to_ansi_bg(terminal_renderer.color_theme().table_row_alt);
        let lines = markdown_output.lines().collect::<Vec<_>>();

        // Header + separator keep their default styling (no background tint).
        assert!(!lines[0].contains(&alt_seq));
        assert!(!lines[1].contains(&alt_seq));
        // Zebra striping: base data rows (alpha, gamma) have no background
        // tint; the alternate row (beta) carries the subtle dark background.
        assert!(!lines[2].contains(&alt_seq));
        assert!(lines[3].contains(&alt_seq));
        assert!(!lines[4].contains(&alt_seq));
    }

    #[test]
    fn wrap_plain_text_wraps_words_and_breaks_long_tokens() {
        assert_eq!(
            wrap_plain_text("one two three", 5),
            vec!["one", "two", "three"]
        );
        assert_eq!(
            wrap_plain_text("supercalifragilistic", 6),
            vec!["superc", "alifra", "gilist", "ic"]
        );
        // CJK wide characters count as two columns.
        assert_eq!(
            wrap_plain_text("中文测试内容", 4),
            vec!["中文", "测试", "内容"]
        );
        // A single long token in a narrow column is hard-broken.
        assert_eq!(
            wrap_plain_text("prefix https://example.com/very/long/url suffix", 12),
            vec!["prefix", "https://exam", "ple.com/very", "/long/url", "suffix"]
        );
    }

    #[test]
    fn renders_table_cells_wrapped_to_fit_terminal_width() {
        let mut renderer = TerminalRenderer::new();
        renderer.set_max_width(20);
        let output = renderer.render_markdown(
            "| Header One | Second Header |\n| :-- | :-- |\n| long value | other |",
        );
        let plain = strip_ansi(&output);
        let lines: Vec<&str> = plain.lines().collect();
        for line in &lines {
            assert!(
                visible_width(line) <= 20,
                "table line exceeds max width: {line:?} ({})",
                visible_width(line)
            );
        }
        // Wrapped content must still be fully visible across lines.
        let joined = plain;
        for expected in ["Header", "One", "Second", "long", "value", "other"] {
            assert!(
                joined.contains(expected),
                "table should contain wrapped {expected:?}: {joined:?}"
            );
        }
    }

    #[test]
    fn streaming_state_waits_for_complete_blocks() {
        let renderer = TerminalRenderer::new();
        let mut state = MarkdownStreamState::default();

        assert_eq!(state.push(&renderer, "# Heading"), None);
        let flushed = state
            .push(&renderer, "\n\nParagraph\n\n")
            .expect("completed block");
        let plain_text = strip_ansi(&flushed);
        assert!(plain_text.contains("Heading"));
        assert!(plain_text.contains("Paragraph"));

        assert_eq!(state.push(&renderer, "```rust\nfn main() {}\n"), None);
        let code = state
            .push(&renderer, "```\n")
            .expect("closed code fence flushes");
        assert!(strip_ansi(&code).contains("fn main()"));
    }

    #[test]
    fn streaming_state_holds_outer_fence_with_nested_inner_fence() {
        let renderer = TerminalRenderer::new();
        let mut state = MarkdownStreamState::default();

        assert_eq!(
            state.push(&renderer, "````markdown\n```rust\nfn inner() {}\n"),
            None,
            "inner triple backticks must not close the outer four-backtick fence"
        );
        assert_eq!(
            state.push(&renderer, "```\n"),
            None,
            "closing the inner fence must not flush the outer fence"
        );
        let flushed = state
            .push(&renderer, "````\n")
            .expect("closing the outer four-backtick fence flushes the buffered block");
        let plain_text = strip_ansi(&flushed);
        assert!(plain_text.contains("fn inner()"));
        assert!(plain_text.contains("```rust"));
    }

    #[test]
    fn streaming_state_distinguishes_backtick_and_tilde_fences() {
        let renderer = TerminalRenderer::new();
        let mut state = MarkdownStreamState::default();

        assert_eq!(state.push(&renderer, "~~~text\n"), None);
        assert_eq!(
            state.push(&renderer, "```\nstill inside tilde fence\n"),
            None,
            "a backtick fence cannot close a tilde-opened fence"
        );
        assert_eq!(state.push(&renderer, "```\n"), None);
        let flushed = state
            .push(&renderer, "~~~\n")
            .expect("matching tilde marker closes the fence");
        let plain_text = strip_ansi(&flushed);
        assert!(plain_text.contains("still inside tilde fence"));
    }

    #[test]
    fn renders_nested_fenced_code_block_preserves_inner_markers() {
        let terminal_renderer = TerminalRenderer::new();
        let markdown_output =
            terminal_renderer.markdown_to_ansi("````markdown\n```rust\nfn nested() {}\n```\n````");
        let plain_text = strip_ansi(&markdown_output);

        assert!(plain_text.contains("╭─ markdown"));
        assert!(plain_text.contains("```rust"));
        assert!(plain_text.contains("fn nested()"));
    }

    #[test]
    fn spinner_advances_frames() {
        let terminal_renderer = TerminalRenderer::new();
        let mut spinner = Spinner::new();
        let mut out = Vec::new();
        spinner
            .tick("Working", terminal_renderer.color_theme(), &mut out)
            .expect("tick succeeds");
        spinner
            .tick("Working", terminal_renderer.color_theme(), &mut out)
            .expect("tick succeeds");

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("Working"));
    }

    #[test]
    fn degrade_latex_unwraps_text_and_frac() {
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\text{energy}$"),
            "$energy$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\frac{a}{b}$"),
            "$a/b$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$E = mc^2$"),
            "$E = mc²$"
        );
    }

    #[test]
    fn degrade_latex_promotes_scripts_and_limits() {
        assert_eq!(degrade_latex(&ColorTheme::default(), "$x_i^2$"), "$xᵢ²$");
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\sum_{i=1}^{n} i$"),
            "$∑_{i=1}ⁿ i$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\int_{0}^{\\infty} e^{-x}$"),
            "$∫₀∞ e^{-x}$"
        );
    }

    #[test]
    fn degrade_latex_leaves_currency_untouched() {
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "it costs $5 and $10 here"),
            "it costs $5 and $10 here"
        );
    }

    #[test]
    fn escapes_pipes_inside_math_spans_only() {
        // A pipe inside `$...$` must be escaped so the table parser does not
        // split the cell; structural pipes (table dividers) stay literal.
        let escaped = escape_pipes_in_spans("| $\\ln|x| + C$ |");
        assert!(
            escaped.contains("$\\ln\u{e000}x\u{e000} + C$"),
            "pipe inside math must use sentinel: {escaped}"
        );
        assert!(
            escaped.starts_with('|'),
            "structural divider must stay: {escaped}"
        );
        // An already-escaped pipe must not be double-escaped.
        assert_eq!(escape_pipes_in_spans("$\\ln\\|x\\|$"), "$\\ln\\|x\\|$");
    }

    #[test]
    fn renders_table_cell_with_pipe_inside_math() {
        let renderer = TerminalRenderer::new();
        let md = "| 函数 | 积分 |\n|------|------|\n| $\\frac{1}{x}$ | $\\ln|x| + C$ |\n";
        let out = renderer.render_markdown(md);
        assert!(
            out.contains("ln |x| + C"),
            "pipe inside math must not split the cell: {out:?}"
        );
        assert!(
            !out.contains("$ln"),
            "stray dollar must not leak into the cell: {out:?}"
        );
    }

    #[test]
    fn degrade_latex_handles_braceless_frac_and_sqrt() {
        assert_eq!(degrade_latex(&ColorTheme::default(), "$\\frac12$"), "$1/2$");
        assert_eq!(degrade_latex(&ColorTheme::default(), "$\\sqrt2$"), "$√2$");
    }

    #[test]
    fn degrade_latex_skips_code_blocks() {
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "```\nlet x = $not_math$;\n```"),
            "```\nlet x = $not_math$;\n```"
        );
    }

    #[test]
    fn degrade_latex_display_frac_is_vertical_block() {
        let got = degrade_latex(
            &ColorTheme::default(),
            "$$x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}$$",
        );
        // No `$$` delimiters remain; a fraction rule is present.
        assert!(!got.contains("$$"));
        assert!(got.contains('─'), "expected a fraction bar");
        assert!(got.contains("x = "), "expected the prefix on the rule line");
        assert!(got.contains("2a"), "expected the denominator");
        // Exactly three rows: numerator / rule / denominator.
        assert_eq!(got.lines().count(), 3);
        // The middle row carries both the prefix and the rule.
        let middle = got.lines().nth(1).unwrap();
        assert!(middle.contains("x = "));
        assert!(middle.contains('─'));
    }

    #[test]
    fn degrade_latex_inline_frac_stays_single_line() {
        let got = degrade_latex(&ColorTheme::default(), "$\\frac12$");
        assert!(!got.contains('\n'));
        assert_eq!(got, "$1/2$");
    }

    #[test]
    fn degrade_latex_accent_commands_use_combining_marks() {
        // vec/dot/hat/bar/tilde apply a combining mark to the argument.
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\vec{F}$"),
            "$\u{0046}\u{20d7}$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\dot{x}$"),
            "$\u{0078}\u{0307}$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\hat{x}$"),
            "$\u{0078}\u{0302}$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\bar{x}$"),
            "$\u{0078}\u{0304}$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\tilde{x}$"),
            "$\u{0078}\u{0303}$"
        );
        // braceless single char (with or without a space) still works.
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\dot x$"),
            "$\u{0078}\u{0307}$"
        );
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\dot{x}$"),
            "$\u{0078}\u{0307}$"
        );
        // ddot stacks two combining diaereses.
        assert_eq!(
            degrade_latex(&ColorTheme::default(), "$\\ddot{x}$"),
            "$\u{0078}\u{0308}\u{0308}$"
        );
    }

    #[test]
    fn close_dangling_fence_appends_closer() {
        let input = "```rust\nfn main() {}\n";
        let output = close_dangling_fence(input);
        assert!(output.ends_with("```\n"));

        let closed = "```rust\nfn main() {}\n```";
        assert_eq!(close_dangling_fence(closed), closed);
    }

    #[test]
    fn renders_display_frac_with_greek_prefix() {
        // Reproduce the actual Maxwell-equation rendering.
        let renderer = TerminalRenderer::new();
        let md = concat!("$$\\nabla \\cdot \\mathbf{E} = \\frac{\\rho}{\\varepsilon_0}$$");
        let out = renderer.render_markdown(md);
        // No fenced/indented code-block wrapper from leading whitespace.
        assert!(!out.contains("╭─"), "no code block wrapper: {out:?}");
        // The prefix before the fraction must survive.
        assert!(out.contains("∇ · E = "), "nabla must survive: {out:?}");
        // The fraction must have a rule and both numerator/denominator.
        assert!(out.contains('─'), "fraction rule must be present: {out:?}");
        assert!(out.contains('ρ'), "numerator ρ must be present: {out:?}");
        assert!(out.contains("ε₀"), "varepsilon must render as ε: {out:?}");
    }
}

#[cfg(test)]
mod spinner_frame_width_tests {
    use super::Spinner;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    #[test]
    fn every_spinner_frame_is_one_cell_wide() {
        for frame in Spinner::FRAMES {
            assert_eq!(
                UnicodeWidthStr::width(frame),
                1,
                "frame {frame:?} is wider than 1 cell; replace with a narrower glyph"
            );
        }
    }
}
