#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub when_to_use: Option<String>,
    pub tools: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub mode: Option<String>,
    /// Optional sub-agent kind (e.g. `explorer` / `plan` / `general-purpose`).
    /// When present it steers the spawned sub-agent's tool set instead of the
    /// hardcoded general-purpose default.
    pub subagent_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown<'a> {
    pub frontmatter: Frontmatter,
    pub body: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterError {
    MissingDelimiter,
    InvalidFrontmatter { reason: &'static str },
    MissingField(&'static str),
    InvalidName(String),
}

pub fn parse_frontmatter(content: &str) -> Result<ParsedMarkdown<'_>, FrontmatterError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(FrontmatterError::MissingDelimiter);
    }

    let after_opener = &trimmed[3..];
    let end = match after_opener.find("\n---") {
        Some(pos) => pos,
        None => {
            return Err(FrontmatterError::InvalidFrontmatter {
                reason: "missing closing frontmatter delimiter",
            });
        }
    };

    let yaml_section = &after_opener[..end];
    let body = after_opener[end + 4..].trim_start();

    let mut name = None;
    let mut description = None;
    let mut model = None;
    let mut reasoning_effort = None;
    let mut when_to_use = None;
    let mut mode = None;
    let mut subagent_type = None;
    let mut tools: Option<Vec<String>> = None;
    let mut skills: Option<Vec<String>> = None;
    let mut multiline_key: Option<&str> = None;
    let mut multiline_lines: Vec<String> = Vec::new();

    for line in yaml_section.lines() {
        if let Some(val) = line.strip_prefix("name:") {
            multiline_key = None;
            let v = val.trim();
            if !v.is_empty() {
                name = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("description:") {
            multiline_key = Some("description");
            multiline_lines.clear();
            let v = val.trim();
            if !v.is_empty() && !v.starts_with('|') && !v.starts_with('>') {
                description = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("model:") {
            multiline_key = None;
            let v = val.trim();
            if !v.is_empty() && !v.starts_with('|') {
                model = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("reasoning_effort:") {
            multiline_key = None;
            let v = val.trim();
            if !v.is_empty() && !v.starts_with('|') {
                reasoning_effort = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("when_to_use:") {
            multiline_key = Some("when_to_use");
            multiline_lines.clear();
            let v = val.trim();
            if !v.is_empty() && !v.starts_with('|') && !v.starts_with('>') {
                when_to_use = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("mode:") {
            multiline_key = None;
            let v = val.trim();
            if !v.is_empty() {
                mode = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("subagent_type:") {
            multiline_key = None;
            let v = val.trim();
            if !v.is_empty() && !v.starts_with('|') {
                subagent_type = Some(v.to_string());
            }
        } else if let Some(val) = line.strip_prefix("tools:") {
            multiline_key = None;
            let v = val.trim();
            if v.starts_with('[') {
                let list: Vec<String> = v
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !list.is_empty() {
                    tools = Some(list);
                }
            }
        } else if let Some(val) = line.strip_prefix("skills:") {
            multiline_key = None;
            let v = val.trim();
            if v.starts_with('[') {
                let list: Vec<String> = v
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !list.is_empty() {
                    skills = Some(list);
                }
            }
        } else if let Some(mk) = multiline_key {
            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                multiline_lines.push(trimmed_line.to_string());
            } else {
                multiline_key = None;
                flush_multiline(mk, &multiline_lines, &mut description, &mut when_to_use);
                multiline_lines.clear();
            }
        }
    }

    if let Some(mk) = multiline_key {
        flush_multiline(mk, &multiline_lines, &mut description, &mut when_to_use);
    }

    if name.is_none() {
        return Err(FrontmatterError::MissingField("name"));
    }
    if description.is_none() {
        return Err(FrontmatterError::MissingField("description"));
    }

    Ok(ParsedMarkdown {
        frontmatter: Frontmatter {
            name,
            description,
            model,
            reasoning_effort,
            when_to_use,
            tools,
            skills,
            mode,
            subagent_type,
        },
        body,
    })
}

/// Parse `permission:` directives from an agent file's frontmatter **without**
/// requiring `name`/`description`. The strict [`parse_frontmatter`] rejects
/// files that omit `name` (e.g. `~/.claw/agents/architect.md`), which would
/// silently discard the file's deny rules and let the sub-agent run with the
/// full tool set. This lenient pass extracts the block so delegation can honor
/// it regardless of `name`/`description` presence.
///
/// Expected block form:
/// ```text
/// permission:
///   read: allow
///   write: deny
///   bash: deny
/// ```
/// The map is `tool-category → decision` (`allow`/`deny`/`ask`). A line that
/// is not an indented `key: value` pair (blank line or a fresh top-level key)
/// ends the block.
#[must_use]
pub fn parse_permission_from_content(
    content: &str,
) -> Option<std::collections::BTreeMap<String, String>> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_opener = &trimmed[3..];
    let end = after_opener.find("\n---")?;
    let yaml_section = &after_opener[..end];

    let mut map = std::collections::BTreeMap::new();
    let mut in_permission = false;
    for line in yaml_section.lines() {
        if !in_permission {
            let stripped = line.trim_start();
            if let Some(rest) = stripped.strip_prefix("permission:") {
                if rest.trim().is_empty() {
                    in_permission = true;
                }
            }
            continue;
        }
        if line.trim().is_empty() {
            in_permission = false;
            continue;
        }
        // A fresh top-level key (no leading whitespace) ends the block.
        if !line.starts_with(|c: char| c.is_whitespace()) {
            in_permission = false;
            continue;
        }
        if let Some((key, value)) = line.trim().split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !key.is_empty() && !value.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn flush_multiline(
    key: &str,
    lines: &[String],
    description: &mut Option<String>,
    when_to_use: &mut Option<String>,
) {
    if lines.is_empty() {
        return;
    }
    let joined = lines.join(" ");
    match key {
        "description" => match description {
            Some(ref mut existing) => {
                existing.push(' ');
                existing.push_str(&joined);
            }
            None => *description = Some(joined),
        },
        "when_to_use" => match when_to_use {
            Some(ref mut existing) => {
                existing.push(' ');
                existing.push_str(&joined);
            }
            None => *when_to_use = Some(joined),
        },
        _ => {}
    }
}

/// Frontmatter understood by Claude Code markdown **slash commands**.
/// Unlike [`Frontmatter`] (agent-oriented, requires `name`/`description`),
/// command frontmatter is lenient: `name` comes from the file, and most
/// fields are optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandFrontmatter {
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub disable_model_invocation: bool,
    pub user_invocable: bool,
    pub shell: Option<String>,
    pub when_to_use: Option<String>,
}

/// Parse the frontmatter of a Claude Code slash-command markdown file.
/// Returns the parsed fields and a borrowed view of the body (after the
/// closing delimiter). A document without a `---` block is accepted: the
/// whole content is treated as the body with empty frontmatter.
pub fn parse_command_frontmatter(content: &str) -> Result<(CommandFrontmatter, &str), FrontmatterError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Ok((CommandFrontmatter::default(), content));
    }

    let after_opener = &trimmed[3..];
    let end = match after_opener.find("\n---") {
        Some(pos) => pos,
        None => {
            return Err(FrontmatterError::InvalidFrontmatter {
                reason: "missing closing frontmatter delimiter",
            });
        }
    };

    let yaml_section = &after_opener[..end];
    let body = after_opener[end + 4..].trim_start();

    let mut fm = CommandFrontmatter::default();
    for line in yaml_section.lines() {
        let value = |prefix: &str| line.strip_prefix(prefix).map(|v| v.trim());
        if let Some(v) = value("description:") {
            fm.description = non_empty(v);
        } else if let Some(v) = value("argument-hint:") {
            fm.argument_hint = non_empty(v);
        } else if let Some(v) = value("when_to_use:") {
            fm.when_to_use = non_empty(v);
        } else if let Some(v) = value("shell:") {
            fm.shell = non_empty(v);
        } else if let Some(v) = value("model:") {
            // `inherit` means "use the active model" -> no override.
            fm.model = non_empty(v).filter(|s| s != "inherit");
        } else if let Some(v) = value("effort:") {
            fm.effort = non_empty(v);
        } else if let Some(v) = value("allowed-tools:") {
            fm.allowed_tools = parse_tool_list(v);
        } else if let Some(v) = value("disable-model-invocation:") {
            fm.disable_model_invocation = v == "true";
        } else if let Some(v) = value("user-invocable:") {
            fm.user_invocable = v != "false";
        }
    }

    Ok((fm, body))
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() || value.starts_with('|') || value.starts_with('>') {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_tool_list(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with('[') {
        let list: Vec<String> = value
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return if list.is_empty() { None } else { Some(list) };
    }
    Some(vec![value.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let result = parse_frontmatter("# Hello\n\nWorld");
        assert!(matches!(result, Err(FrontmatterError::MissingDelimiter)));
    }

    #[test]
    fn test_name_and_description() {
        let content = "---\nname: my-agent\ndescription: A test agent\n---\n\n# Body text";
        let parsed = parse_frontmatter(content).expect("valid input");
        assert_eq!(parsed.frontmatter.name, Some("my-agent".into()));
        assert_eq!(parsed.frontmatter.description, Some("A test agent".into()));
        assert_eq!(parsed.body, "# Body text");
    }

    #[test]
    fn test_only_body() {
        let result = parse_frontmatter("Some content");
        assert!(matches!(result, Err(FrontmatterError::MissingDelimiter)));
    }

    #[test]
    fn test_empty_frontmatter_delimiters() {
        let content = "---\n---\n\nBody";
        let result = parse_frontmatter(content);
        assert!(matches!(result, Err(FrontmatterError::MissingField("name"))));
    }

    #[test]
    fn test_multiline_description() {
        let content = "---\nname: my-agent\ndescription: |\n  A longer\n  description\n---\n\nBody";
        let parsed = parse_frontmatter(content).expect("valid input");
        assert_eq!(parsed.frontmatter.name, Some("my-agent".into()));
        assert_eq!(parsed.frontmatter.description, Some("A longer description".into()));
        assert_eq!(parsed.body, "Body");
    }

    #[test]
    fn test_description_without_name() {
        let content = "---\ndescription: Just a description\n---\n\nBody";
        let result = parse_frontmatter(content);
        assert!(matches!(result, Err(FrontmatterError::MissingField("name"))));
    }

    #[test]
    fn test_agent_fields() {
        let content = "---\nname: my-agent\ndescription: A test agent\nmodel: claude-sonnet-4\nreasoning_effort: high\nwhen_to_use: Use for testing\ntools: [\"read\", \"write\"]\nskills: [\"skill1\", \"skill2\"]\n---\n\nBody";
        let parsed = parse_frontmatter(content).expect("valid input");
        assert_eq!(parsed.frontmatter.name, Some("my-agent".into()));
        assert_eq!(parsed.frontmatter.model, Some("claude-sonnet-4".into()));
        assert_eq!(parsed.frontmatter.reasoning_effort, Some("high".into()));
        assert_eq!(parsed.frontmatter.when_to_use, Some("Use for testing".into()));
        assert_eq!(parsed.frontmatter.tools, Some(vec!["read".into(), "write".into()]));
        assert_eq!(parsed.frontmatter.skills, Some(vec!["skill1".into(), "skill2".into()]));
    }

    #[test]
    fn test_agent_fields_single_values() {
        let content = "---\nmodel: claude-sonnet-4\nreasoning_effort: high\n---\n\nBody";
        let result = parse_frontmatter(content);
        assert!(matches!(result, Err(FrontmatterError::MissingField("name"))));
    }

    #[test]
    fn test_agent_multiline_when_to_use() {
        let content = "---\nname: my-agent\ndescription: An agent\nwhen_to_use: |\n  Use this when you need\n  to test something\n---\n\nBody";
        let parsed = parse_frontmatter(content).expect("valid input");
        assert_eq!(parsed.frontmatter.name, Some("my-agent".into()));
        assert_eq!(parsed.frontmatter.when_to_use, Some("Use this when you need to test something".into()));
    }

    #[test]
    fn parse_frontmatter_returns_result_with_missing_field_error() {
        // No `name:` field in frontmatter
        let content = "---\ndescription: foo\n---\nbody";
        let result = parse_frontmatter(content);
        assert!(matches!(
            result.map(|p| p.frontmatter.name),
            Err(FrontmatterError::MissingField("name"))
        ));
    }

    #[test]
    fn parse_permission_block_without_name() {
        // architect.md-style file: no `name:`, but a `permission:` block that
        // the strict parser would reject wholesale.
        let content = "---\ndescription: An architect\npermission:\n  read: allow\n  write: deny\n  bash: deny\n---\nbody";
        let permission = parse_permission_from_content(content).expect("block parsed");
        assert_eq!(permission.get("read").map(String::as_str), Some("allow"));
        assert_eq!(permission.get("write").map(String::as_str), Some("deny"));
        assert_eq!(permission.get("bash").map(String::as_str), Some("deny"));
        assert_eq!(permission.len(), 3);
    }

    #[test]
    fn parse_permission_stops_at_next_top_level_key() {
        let content = "---\npermission:\n  read: allow\nname: my-agent\ndescription: d\n---\nbody";
        let permission = parse_permission_from_content(content).expect("block parsed");
        assert_eq!(permission.len(), 1);
        assert_eq!(permission.get("read").map(String::as_str), Some("allow"));
    }

    #[test]
    fn parse_permission_none_when_absent_or_broken() {
        assert!(parse_permission_from_content("no frontmatter").is_none());
        assert!(parse_permission_from_content("---\nname: x\ndescription: y\n---\nbody").is_none());
        // `permission:` with an inline value (not the block form) yields nothing.
        assert!(parse_permission_from_content("---\npermission: deny\nname: x\ndescription: y\n---\nbody").is_none());
    }
}
