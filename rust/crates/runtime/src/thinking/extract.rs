//! `extract_embedded_tools` — parses `<tool_call>` blocks emitted by
//! reasoning models inside their thinking text and converts them into
//! structured `(id, name, input)` tuples that can be dispatched as
//! regular tool calls.
//!
//! Some models (notably Anthropic Claude in certain configurations)
//! emit tool calls as XML inside the thinking block instead of (or in
//! addition to) the structured `ToolUse` content block. The agent
//! runtime must intercept these embedded tool calls before the thinking
//! text is rendered to the user.
//!
//! Supported XML format:
//! ```xml
//! <tool_call>
//! <function=ToolName>
//! <parameter=Key>Value</parameter>
//! </function>
//! </tool_call>
//! ```

/// ASCII `<DSML>` marker prefix used by DeepSeek-family endpoints that emit
/// tool calls as text: `<DSML>tool_calls`, `<DSML>invoke name=...>` and
/// `<DSML>parameter name=...>`.
const DSML_MARKER_ASCII: &str = "<DSML>";

/// Fullwidth variant of the DSML marker: the endpoint renders the pipe glyph
/// as U+FF5C FULLWIDTH VERTICAL LINE (`｜`) instead of ASCII `|`, yielding
/// `<\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}...>`.
const DSML_MARKER_FULLWIDTH: &str = "<\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}";

/// Parse `<tool_call>` blocks embedded in thinking text.
///
/// Returns `(clean_text, tool_calls)`:
/// - `clean_text` is the input with all `<tool_call>…</tool_call>` blocks
///   removed.
/// - `tool_calls` is a list of `(id, name, input)` tuples, where `id` is
///   `toolu_thinking_<N>`, `name` is the function name, and `input` is a
///   `serde_json::Value` built from the `<parameter>` key/value pairs.
///
/// Malformed blocks emit a warning to stderr and are silently dropped
/// from the output (the partial text before the malformed block is
/// still kept in `clean_text`).
#[must_use]
pub fn extract_embedded_tools(text: &str) -> (String, Vec<(String, String, serde_json::Value)>) {
    let mut clean = String::with_capacity(text.len());
    let mut tools: Vec<(String, String, serde_json::Value)> = Vec::new();
    let mut remaining = text;
    let mut tool_counter: u64 = 0;

    while let Some(tc_start) = remaining.find("<tool_call") {
        let after_tc = &remaining[tc_start + "<tool_call".len()..];
        let Some(tc_open_end) = after_tc.find('>') else {
            // Not a real <tool_call> tag — push past the prefix and continue.
            clean.push_str(&remaining[..tc_start + "<tool_call".len()]);
            remaining = &remaining[tc_start + "<tool_call".len()..];
            continue;
        };
        let body_start = tc_start + "<tool_call".len() + tc_open_end + 1;
        let Some(close_rel) = remaining[body_start..].find("</tool_call>") else {
            // Partial / malformed block — keep the text as-is and carry on.
            clean.push_str(&remaining[..body_start]);
            remaining = &remaining[body_start..];
            continue;
        };
        let close_end = body_start + close_rel + "</tool_call>".len();
        let body = &remaining[body_start..body_start + close_rel];

        clean.push_str(&remaining[..tc_start]);

        if let Some((name, params)) = parse_embedded_tool_body(body) {
            let input = build_tool_json_input(&params);
            let id = format!("toolu_thinking_{tool_counter}");
            tool_counter += 1;
            tools.push((id, name, input));
        } else {
            eprintln!(
                "[tool_extract] failed to parse <tool_call> body; tool silently dropped. body={}",
                body.chars().take(200).collect::<String>()
            );
        }

        remaining = &remaining[close_end..];
    }

    clean.push_str(remaining);

    // Second pass: also handle <invoke tool="Name">...</invoke> format
    remaining = &clean[..];
    let mut clean2 = String::with_capacity(clean.len());
    loop {
        let Some(invoke_start) = remaining.find("<invoke tool=\"") else {
            clean2.push_str(remaining);
            break;
        };
        let after_open = &remaining[invoke_start + "<invoke tool=\"".len()..];
        let Some(quote_end) = after_open.find('"') else {
            clean2.push_str(&remaining[..invoke_start + 1]);
            remaining = &remaining[invoke_start + 1..];
            continue;
        };
        let tool_name = after_open[..quote_end].trim().to_string();
        let body_start = invoke_start + "<invoke tool=\"".len() + quote_end + 1;
        let Some(close_rel) = remaining[body_start..].find("</invoke>") else {
            clean2.push_str(&remaining[..body_start]);
            remaining = &remaining[body_start..];
            continue;
        };
        let close_end = body_start + close_rel + "</invoke>".len();
        let body = &remaining[body_start..body_start + close_rel];
        clean2.push_str(&remaining[..invoke_start]);

        let params = parse_invoke_parameters(body);
        let input = build_tool_json_input(&params);
        let id = format!("toolu_thinking_{tool_counter}");
        tool_counter += 1;
        tools.push((id, tool_name, input));

        remaining = &remaining[close_end..];
    }

    // Third pass: DeepSeek-family endpoints emit tool calls wrapped in
    // `<DSML>`-prefixed XML (`<DSML>tool_calls>`, `<DSML>invoke name=...>`,
    // `<DSML>parameter name=...>value</DSML>parameter>`). Without this pass
    // those calls stay as literal text and the agent loop exits early with a
    // transitional narration instead of executing the tool.
    let (clean3, dsml_tools) = extract_dsml_tools(&clean2, tool_counter);
    tools.extend(dsml_tools);

    (clean3, tools)
}

/// Parse a `<DSML>parameter name="Key">Value</DSML>parameter>` block body.
///
/// DeepSeek-family endpoints emit tool calls wrapped in `<DSML>`-prefixed tags:
/// the opening tag is `<DSML>parameter name="command" string="true">value` and
/// the closing tag is `</DSML>parameter>`. `parse_invoke_parameters` already
/// handles the `<parameter name="...">value</parameter>` shape; this variant
/// recognises the same `name="..."` attribute when a `<DSML>` prefix and the
/// `</DSML>parameter>` closer are present.
///
/// In practice the marker is written as `<\uff5c\uff5cDSML\uff5c\uff5c...>`
/// (U+FF5C FULLWIDTH VERTICAL LINE padding rather than ASCII `|`), so both the
/// ASCII and the fullwidth forms are matched.
fn parse_dsml_parameters(body: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut rest = body;
    let open_patterns = [
        format!("{DSML_MARKER_ASCII}parameter name=\""),
        format!("{DSML_MARKER_FULLWIDTH}parameter name=\""),
    ];
    let close_patterns = [
        "</DSML>parameter>".to_string(),
        "</\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}parameter>".to_string(),
    ];
    loop {
        let p_start = open_patterns
            .iter()
            .filter_map(|pat| rest.find(pat.as_str()))
            .min();
        let Some(p_start) = p_start else { break };
        let matched = open_patterns
            .iter()
            .find(|pat| rest[p_start..].starts_with(pat.as_str()))
            .expect("a marker pattern matched, one must own the offset");
        let after_open = &rest[p_start + matched.len()..];
        let Some(name_end_rel) = after_open.find('"') else { break };
        let key = after_open[..name_end_rel].trim().to_string();
        let val_start = p_start + matched.len() + name_end_rel + 1;
        if rest[val_start..].starts_with("/>") {
            params.push((key, String::new()));
            rest = &rest[val_start + 2..];
            continue;
        }
        let Some(gt) = rest[val_start..].find('>') else { break };
        let content_start = val_start + gt + 1;
        let (cp_rel, cp_len) = close_patterns
            .iter()
            .filter_map(|pat| rest[content_start..].find(pat.as_str()).map(|idx| (idx, pat.len())))
            .min_by_key(|(idx, _)| *idx)
            .unwrap_or((0, 0));
        if cp_len == 0 {
            break;
        }
        let value = rest[content_start..][..cp_rel].trim().to_string();
        params.push((key, value));
        rest = &rest[content_start + cp_rel + cp_len..];
    }
    params
}

/// Parse `<DSML>invoke name="Name">...</DSML>invoke>` blocks embedded in text.
/// Returns `(clean_text, tools)` where `clean_text` is the input with all
/// `<DSML>tool_calls>…</DSML>tool_calls>` blocks removed and `tools` is the
/// extracted `(id, name, input)` list. This is the third extraction pass used
/// by [`extract_embedded_tools`] for DeepSeek-family endpoints that wrap tool
/// calls in `<DSML>`-prefixed XML instead of plain `<tool_call>` / `<invoke>`.
fn extract_dsml_tools(text: &str, start_counter: u64) -> (String, Vec<(String, String, serde_json::Value)>) {
    let mut clean = String::with_capacity(text.len());
    let mut tools = Vec::new();
    let mut remaining = text;
    let mut counter = start_counter;

    let invoke_open = [
        format!("{DSML_MARKER_ASCII}invoke name=\""),
        format!("{DSML_MARKER_FULLWIDTH}invoke name=\""),
    ];
    let wrapper_open = [
        "<DSML>tool_calls>".to_string(),
        "<\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}tool_calls>".to_string(),
    ];
    // Closing tags: `</DSML>...>` in ASCII form, or `</\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}...>`.
    let invoke_close = [
        "</DSML>invoke>".to_string(),
        "</\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}invoke>".to_string(),
    ];
    let tool_calls_close = [
        "</DSML>tool_calls>".to_string(),
        "</\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}tool_calls>".to_string(),
    ];

    while let Some((invoke_start, matched_len)) = invoke_open
        .iter()
        .filter_map(|pat| remaining.find(pat.as_str()).map(|idx| (idx, pat.len())))
        .min_by_key(|(idx, _)| *idx)
    {
        let after_open = &remaining[invoke_start + matched_len..];
        let Some(quote_end) = after_open.find('"') else {
            clean.push_str(&remaining[..invoke_start + 1]);
            remaining = &remaining[invoke_start + 1..];
            continue;
        };
        let tool_name = after_open[..quote_end].trim().to_string();
        let body_start = invoke_start + matched_len + quote_end + 1;
        let (close_rel, close_len) = invoke_close
            .iter()
            .filter_map(|pat| remaining[body_start..].find(pat.as_str()).map(|idx| (idx, pat.len())))
            .min_by_key(|(idx, _)| *idx)
            .unwrap_or((0, 0));
        if close_len == 0 {
            clean.push_str(&remaining[..body_start]);
            remaining = &remaining[body_start..];
            continue;
        }
        let close_end = body_start + close_rel + close_len;
        let body = &remaining[body_start..body_start + close_rel];

        // Reconstruct the prefix so a surrounding `<DSML>tool_calls>`
        // wrapper is dropped without leaving stray markers.
        let mut prefix = String::from(&remaining[..invoke_start]);
        for wrapper in &wrapper_open {
            if let Some(wrapper_start) = prefix.rfind(wrapper.as_str()) {
                let head = &prefix[..wrapper_start];
                let between = prefix[wrapper_start + wrapper.len()..].trim();
                if between.is_empty() {
                    prefix = head.trim_end_matches('\n').to_string();
                }
            }
        }
        clean.push_str(&prefix);

        let params = parse_dsml_parameters(body);
        let input = build_tool_json_input(&params);
        let id = format!("toolu_thinking_{counter}");
        counter += 1;
        tools.push((id, tool_name, input));

        remaining = &remaining[close_end..];

        // Drop a trailing `</DSML>tool_calls>` closer that belongs to the
        // wrapper we peeled above.
        for closer in &tool_calls_close {
            if let Some(tc_end_rel) = remaining.find(closer.as_str()) {
                let before = remaining[..tc_end_rel].trim_end();
                let after = &remaining[tc_end_rel + closer.len()..];
                if after.trim().is_empty() {
                    remaining = before;
                    if remaining.ends_with('\n') {
                        remaining = &remaining[..remaining.len() - 1];
                    }
                    break;
                }
            }
        }
    }

    clean.push_str(remaining);
    (clean, tools)
}

/// Parse `<parameter name="Key">Value</parameter>` blocks from an invoke body.
/// Supports both `<parameter name="k">v</parameter>` and self-closing `<parameter name="k"/>`.
pub fn parse_invoke_parameters(body: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut rest = body;
    loop {
        let Some(p_start) = rest.find("<parameter name=\"") else { break };
        let after_name = &rest[p_start + "<parameter name=\"".len()..];
        let Some(name_end) = after_name.find('"') else { break };
        let key = after_name[..name_end].trim().to_string();
        let val_start = p_start + "<parameter name=\"".len() + name_end + 1;
        // Check for self-closing tag: <parameter name="x"/>
        if rest[val_start..].starts_with("/>") {
            params.push((key, String::new()));
            rest = &rest[val_start + 2..];
            continue;
        }
        let Some(gt) = rest[val_start..].find('>') else { break };
        let content_start = val_start + gt + 1;
        let Some(cp_rel) = rest[content_start..].find("</parameter>") else { break };
        let value = rest[content_start..][..cp_rel].trim().to_string();
        params.push((key, value));
        rest = &rest[content_start + cp_rel + "</parameter>".len()..];
    }
    params
}

/// Parse a `<tool_call>` body to extract function name and parameter pairs.
/// Returns `None` if no valid `<function=Name>` is found.
fn parse_embedded_tool_body(body: &str) -> Option<(String, Vec<(String, String)>)> {
    let mut rest = body.trim();
    let tool_name: String;
    let mut params: Vec<(String, String)> = Vec::new();

    if let Some(f_start) = rest.find("<function") {
        let after_f = &rest[f_start + "<function".len()..];
        let Some(f_close) = after_f.find('>') else {
            eprintln!("[tool_extract] malformed <function> tag: no closing '>'");
            return None;
        };
        let tag_content = after_f[..f_close].trim();

        if let Some(eq) = tag_content.find('=') {
            tool_name = tag_content[eq + 1..].trim().to_string();
        } else {
            eprintln!(
                "[tool_extract] malformed <function> tag: no '=' in attribute: <{tag_content}>"
            );
            return None;
        }

        rest = &after_f[f_close + 1..];
        loop {
            rest = rest.trim_start();
            if rest.starts_with("</function>") || rest.is_empty() {
                break;
            }
            if let Some(p_start) = rest.find("<parameter") {
                let after_p = &rest[p_start + "<parameter".len()..];
                let Some(p_close) = after_p.find('>') else { break };
                let p_tag = after_p[..p_close].trim();
                let Some(eq) = p_tag.find('=') else { break };
                let key = p_tag[eq + 1..].trim();
                let val_start = p_start + "<parameter".len() + p_close + 1;
                let Some(cp_rel) = rest[val_start..].find("</parameter>") else {
                    break;
                };
                let value = rest[val_start..][..cp_rel].trim();
                params.push((key.to_string(), value.to_string()));
                rest = &rest[val_start + cp_rel + "</parameter>".len()..];
            } else {
                break;
            }
        }
    } else {
        eprintln!(
            "[tool_extract] <tool_call> body has no <function> tag; body_snippet={}",
            body.chars().take(120).collect::<String>()
        );
        return None;
    }

    Some((tool_name, params))
}

/// Build a JSON object from parameter key-value pairs.
/// Tries to parse each value as JSON first (supports numbers, booleans,
/// arrays, objects); falls back to string if JSON parsing fails.
fn build_tool_json_input(params: &[(String, String)]) -> serde_json::Value {
    use serde_json::Value;
    if params.is_empty() {
        return Value::Object(serde_json::Map::new());
    }
    let mut map = serde_json::Map::new();
    for (key, value) in params {
        let val = if let Ok(json_val) = serde_json::from_str::<Value>(value) {
            json_val
        } else {
            Value::String(value.clone())
        };
        map.insert(key.clone(), val);
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::{extract_embedded_tools, parse_invoke_parameters};

    #[test]
    fn extract_embedded_tools_unchanged() {
        let text = r"before <tool_call><function=foo><parameter=arg>value</parameter></function></tool_call> after";
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "before  after");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "foo");
        assert_eq!(tools[0].2, json!({"arg": "value"}));
    }

    #[test]
    fn extract_embedded_tools_numeric_params() {
        let text = r"<tool_call><function=compute><parameter=a>1</parameter><parameter=b>2</parameter></function></tool_call>";
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "compute");
        assert_eq!(tools[0].2, json!({"a": 1, "b": 2}));
    }

    #[test]
    fn extract_embedded_tools_no_tool_call() {
        let (clean, tools) = extract_embedded_tools("plain text");
        assert_eq!(clean, "plain text");
        assert!(tools.is_empty());
    }

    #[test]
    fn extract_dsml_invoke_tool_format() {
        // DeepSeek-family endpoints wrap tool calls in `<DSML>`-prefixed XML
        // instead of the plain `<tool_call>` / `<invoke tool=...>` forms. The
        // marker uses the ASCII `<DSML>` prefix.
        let text = concat!(
            "before\n",
            "<DSML>tool_calls>\n",
            "<DSML>invoke name=\"bash\">\n",
            "<DSML>parameter name=\"command\" string=\"true\">ls</DSML>parameter>\n",
            "</DSML>invoke>\n",
            "</DSML>tool_calls>",
        );
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "before");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "bash");
        assert_eq!(tools[0].2, json!({ "command": "ls" }));
    }

    #[test]
    fn extract_dsml_tool_call_format() {
        let text = concat!(
            "<DSML>tool_calls>\n",
            "<DSML>invoke name=\"read_file\">\n",
            "<DSML>parameter name=\"path\" string=\"true\">/tmp/x.txt</DSML>parameter>\n",
            "</DSML>invoke>\n",
            "</DSML>tool_calls>",
        );
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "read_file");
        assert_eq!(tools[0].2, json!({ "path": "/tmp/x.txt" }));
    }

    #[test]
    fn extract_dsml_fullwidth_marker_format() {
        // Observed live from a DeepSeek-family endpoint: the pipe glyph inside
        // the DSML marker is emitted as U+FF5C FULLWIDTH VERTICAL LINE rather
        // than ASCII `|`, producing `<\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}...>`.
        let full = "\u{ff5c}\u{ff5c}";
        let text = format!(
            "before\n<{full}DSML{full}tool_calls>\n\
             <{full}DSML{full}invoke name=\"bash\">\n\
             <{full}DSML{full}parameter name=\"command\" string=\"true\">ls</{full}DSML{full}parameter>\n\
             </{full}DSML{full}invoke>\n\
             </{full}DSML{full}tool_calls>"
        );
        let (clean, tools) = extract_embedded_tools(&text);
        assert_eq!(clean, "before");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "bash");
        assert_eq!(tools[0].2, json!({ "command": "ls" }));
    }

    #[test]
    fn extract_dsml_live_session_text() {
        // Verbatim output captured from a real session where the sub-agent's
        // DeepSeek endpoint wrapped a bash tool call in fullwidth DSML markers.
        let full = "\u{ff5c}\u{ff5c}";
        let text = format!(
            "Now let me also look at the workspace architecture to understand the codebase patterns (since this is the Code Architect role context):\n\
             \n\
             <{full}DSML{full}tool_calls>\n\
             <{full}DSML{full}invoke name=\"bash\">\n\
             <{full}DSML{full}parameter name=\"command\" string=\"true\">ls C:/Users/Incredible/Code/clawcode/rust/crates/</{full}DSML{full}parameter>\n\
             <{full}DSML{full}parameter name=\"description\" string=\"true\">List crate structure for architecture awareness</{full}DSML{full}parameter>\n\
             </{full}DSML{full}invoke>\n\
             </{full}DSML{full}tool_calls>"
        );
        let (clean, tools) = extract_embedded_tools(&text);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "bash");
        assert_eq!(
            tools[0].2,
            json!({
                "command": "ls C:/Users/Incredible/Code/clawcode/rust/crates/",
                "description": "List crate structure for architecture awareness",
            })
        );
        assert!(
            !clean.contains("DSML"),
            "raw DSML XML must be stripped from clean text: {clean:?}"
        );
        assert!(
            clean.contains("Now let me also look at the workspace architecture"),
            "narration text must survive extraction: {clean:?}"
        );
    }

    #[test]
    fn extract_invoke_tool_format() {
        let text = r#"before <invoke tool="read_file"><parameter name="path">/tmp/x.txt</parameter></invoke> after"#;
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "before  after");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "read_file");
        assert_eq!(tools[0].2, json!({"path": "/tmp/x.txt"}));
    }

    #[test]
    fn extract_invoke_tool_multiple_params() {
        let text = r#"<invoke tool="bash"><parameter name="command">ls -la</parameter><parameter name="timeout">30</parameter></invoke>"#;
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "bash");
        assert_eq!(tools[0].2, json!({"command": "ls -la", "timeout": 30}));
    }

    #[test]
    fn extract_invoke_tool_mixed_with_tool_call() {
        let text = r#"<tool_call><function=foo><parameter=arg>1</parameter></function></tool_call> and <invoke tool="bar"><parameter name="x">hello</parameter></invoke>"#;
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, " and ");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].1, "foo");
        assert_eq!(tools[1].1, "bar");
    }

    #[test]
    fn extract_invoke_tool_empty_body() {
        let text = r#"before <invoke tool="noop"></invoke> after"#;
        let (clean, tools) = extract_embedded_tools(text);
        assert_eq!(clean, "before  after");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].1, "noop");
        assert_eq!(tools[0].2, json!({}));
    }

    #[test]
    fn parse_invoke_parameters_empty() {
        let params = parse_invoke_parameters("");
        assert!(params.is_empty());
    }

    #[test]
    fn parse_invoke_parameters_self_closing() {
        let params = parse_invoke_parameters(r#"<parameter name="flag"/>"#);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "flag");
        assert_eq!(params[0].1, "");
    }
}
