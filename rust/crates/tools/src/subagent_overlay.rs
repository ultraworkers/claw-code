use std::io::{self, Write};

use agents::{AgentProgress, AgentStatus, SubagentProgressEvent};
use crossterm::cursor::MoveUp;
use crossterm::style::{Color, Stylize};
use crossterm::terminal::{Clear, ClearType};
use crossterm::queue;
use unicode_width::UnicodeWidthChar;

const AGENT_LABEL_WIDTH: usize = 28;

pub fn render_subagent_inline(
    agents: &[AgentProgress],
    terminal_width: usize,
    last_lines: &mut usize,
) -> io::Result<()> {
    let active: Vec<&AgentProgress> = agents
        .iter()
        .filter(|a| !matches!(a.status, AgentStatus::Completed | AgentStatus::Failed))
        .collect();

    if active.is_empty() {
        return Ok(());
    }

    let width = terminal_width.max(40).min(120);
    let mut out = io::stderr();

    if *last_lines > 0 {
        queue!(out, MoveUp(*last_lines as u16))?;
    }
    *last_lines = active.len();

    queue!(out, Clear(ClearType::FromCursorDown))?;

    for entry in active {
        let line = format_agent_line(entry, width);
        writeln!(out, "{}", line)?;
    }

    out.flush()?;
    Ok(())
}

pub fn finalize_subagent_inline(
    agents: &[AgentProgress],
    terminal_width: usize,
    last_lines: &mut usize,
) -> io::Result<()> {
    if agents.is_empty() {
        return Ok(());
    }

    let width = terminal_width.max(40).min(120);
    let mut out = io::stderr();

    if *last_lines > 0 {
        queue!(out, MoveUp(*last_lines as u16))?;
    }

    queue!(out, Clear(ClearType::FromCursorDown))?;

    let mut total_lines = 0usize;
    for entry in agents {
        let line = format_agent_line(entry, width);
        writeln!(out, "{}", line)?;
        total_lines += 1;

        let result_line = entry
            .final_event
            .as_ref()
            .and_then(|e| format_result_line(e))
            .or_else(|| {
                entry.events.iter().rev().find_map(|e| format_result_line(e))
            })
            .unwrap_or_default();

        if !result_line.is_empty() {
            writeln!(out, "{}", result_line)?;
            total_lines += result_line.lines().count();
        }
    }

    *last_lines = total_lines;
    out.flush()?;
    Ok(())
}

fn format_result_line(event: &SubagentProgressEvent) -> Option<String> {
    match event {
        SubagentProgressEvent::Completed { result_preview } => {
            Some(format!("  {}", result_preview.clone().with(Color::Rgb { r: 142, g: 142, b: 147 })))
        }
        SubagentProgressEvent::Failed { error } => {
            Some(format!("  {}", error.to_string().red()))
        }
        _ => None,
    }
}

fn format_agent_line(entry: &AgentProgress, width: usize) -> String {
    let elapsed = entry.started_at.elapsed();
    let status = status_style(entry.status);
    let elapsed_str = format_elapsed(elapsed);
    let label = format!("{} ({})", entry.name, entry.subagent_type);
    let truncated: String = label.chars().take(AGENT_LABEL_WIDTH).collect();

    let detail = entry
        .current_activity
        .as_ref()
        .map(|activity| {
            let clean = activity.replace('\r', " ").replace('\n', " ");
            format!(" {}", clean.dim())
        })
        .or_else(|| {
            entry.events.iter().rev().find_map(|e| match e {
                SubagentProgressEvent::Thinking { text } => {
                    let preview: String = text.chars().take(45).collect();
                    let clean = preview.replace('\r', " ").replace('\n', " ");
                    Some(format!(" {}", clean.dim()))
                }
                SubagentProgressEvent::ToolCall { tool_name, .. } => {
                    Some(format!(" {}", tool_name.to_string().yellow()))
                }
                SubagentProgressEvent::ToolResult { tool_name, .. } => {
                    Some(format!(" \u{2713} {}", tool_name.clone().green()))
                }
                SubagentProgressEvent::Completed { .. } => {
                    Some(" \u{2714}".green().to_string())
                }
                SubagentProgressEvent::Failed { .. } => {
                    Some(" \u{2716}".red().to_string())
                }
                SubagentProgressEvent::StatusChange { .. } => None,
            })
        })
        .unwrap_or_default();

    let iteration = if entry.iteration_count > 0 {
        format!(" [{}]", entry.iteration_count)
    } else {
        String::new()
    };

    let raw = format!("{status} {truncated}{iteration} [{elapsed_str}]{detail}");
    visual_truncate(&raw, width)
}

fn status_style(status: AgentStatus) -> String {
    match status {
        AgentStatus::Running => "\u{25cf}".blue().to_string(),
        AgentStatus::Thinking => "\u{25cf}".cyan().to_string(),
        AgentStatus::UsingTool => "\u{25cf}".yellow().to_string(),
        AgentStatus::Completed => "\u{25cf}".green().to_string(),
        AgentStatus::Failed => "\u{25cf}".red().to_string(),
    }
}

fn visual_truncate(s: &str, max_visual_width: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut visual_w = 0usize;
    let mut truncated = false;

    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            let start = out.len();
            out.push(c);
            if chars.next_if_eq(&'[').is_some() {
                out.push('[');
                while let Some(&n) = chars.peek() {
                    out.push(n);
                    chars.next();
                    if n.is_ascii() && (0x40..=0x7e).contains(&(n as u8)) {
                        break;
                    }
                }
            } else if chars.next_if_eq(&']').is_some() {
                out.push(']');
                loop {
                    match chars.next() {
                        Some('\x07') | None => break,
                        Some('\x1b') => {
                            if chars.next_if_eq(&'\\').is_some() {
                                out.push_str("\x1b\\");
                            }
                            break;
                        }
                        Some(n) => out.push(n),
                    }
                }
            }
            if out[start..].chars().last().is_some_and(|last| {
                !last.is_ascii() || !(0x40..=0x7e).contains(&(last as u8))
            }) {
                out.truncate(start);
            }
        } else {
            let w = c.width().unwrap_or(0);
            if w == 0 {
                if c == '\n' || c == '\r' {
                    out.push(' ');
                } else {
                    out.push(c);
                }
                continue;
            }
            if visual_w + w > max_visual_width {
                truncated = true;
                break;
            }
            visual_w += w;
            out.push(c);
        }
    }

    if truncated {
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                let start = out.len();
                out.push(c);
                if chars.next_if_eq(&'[').is_some() {
                    out.push('[');
                    while let Some(&n) = chars.peek() {
                        out.push(n);
                        chars.next();
                        if n.is_ascii() && (0x40..=0x7e).contains(&(n as u8)) {
                            break;
                        }
                    }
                } else if chars.next_if_eq(&']').is_some() {
                    out.push(']');
                    loop {
                        match chars.next() {
                            Some('\x07') | None => break,
                            Some('\x1b') => {
                                if chars.next_if_eq(&'\\').is_some() {
                                    out.push_str("\x1b\\");
                                }
                                break;
                            }
                            Some(n) => out.push(n),
                        }
                    }
                }
                if out[start..].chars().last().is_some_and(|last| {
                    !last.is_ascii() || !(0x40..=0x7e).contains(&(last as u8))
                }) {
                    out.truncate(start);
                }
            }
        }
        if out.contains('\x1b') {
            let has_open_sgr = out[..out.len().saturating_sub(4)].contains('\x1b');
            if has_open_sgr && !out.ends_with("\x1b[0m") {
                out.push_str("\x1b[0m");
            }
        }
    }

    out
}

fn format_elapsed(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_truncate_passes_ascii() {
        assert_eq!(visual_truncate("hello", 10), "hello");
    }

    #[test]
    fn visual_truncate_cuts_ascii() {
        assert_eq!(visual_truncate("hello world", 5), "hello");
    }

    #[test]
    fn visual_truncate_cjk_double_width() {
        assert_eq!(visual_truncate("你好", 3), "你");
    }

    #[test]
    fn visual_truncate_mixed() {
        assert_eq!(visual_truncate("a你好", 4), "a你");
    }

    #[test]
    fn visual_truncate_preserves_ansi() {
        let s = "\x1b[32mok\x1b[0m";
        assert_eq!(visual_truncate(s, 2), s);
    }

    #[test]
    fn visual_truncate_cuts_after_ansi() {
        let s = "\x1b[32mhello\x1b[0m";
        assert_eq!(visual_truncate(s, 3), "\x1b[32mhel\x1b[0m");
    }

    #[test]
    fn visual_truncate_zero_width() {
        assert_eq!(visual_truncate("hello", 0), "");
        assert_eq!(visual_truncate("", 10), "");
    }
}
