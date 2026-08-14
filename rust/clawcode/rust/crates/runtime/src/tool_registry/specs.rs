use serde_json::{json, Value};

use crate::permissions::PermissionMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
    pub internal: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_spec<'a>(specs: &'a [ToolSpec], name: &str) -> &'a ToolSpec {
        specs
            .iter()
            .find(|spec| spec.name == name)
            .unwrap_or_else(|| panic!("tool spec `{name}` should be present"))
    }

    #[test]
    fn read_file_schema_advertises_full_flag() {
        let specs = mvp_tool_specs();
        let spec = find_spec(&specs, "read_file");
        let properties = spec
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("read_file schema should have properties");
        assert!(
            properties.contains_key("full"),
            "read_file schema must declare a `full` boolean so the LLM can request content; \
             current properties: {properties:?}"
        );
        let full = &properties["full"];
        assert_eq!(full.get("type").and_then(Value::as_str), Some("boolean"));
    }

    #[test]
    fn read_file_schema_remains_closed() {
        let specs = mvp_tool_specs();
        let spec = find_spec(&specs, "read_file");
        assert_eq!(
            spec.input_schema.get("additionalProperties"),
            Some(&Value::Bool(false)),
            "read_file schema must keep `additionalProperties: false` to reject unknown fields"
        );
    }

    #[test]
    fn agent_schema_advertises_mode_and_reasoning_effort() {
        let specs = mvp_tool_specs();
        let spec = find_spec(&specs, "Agent");
        let properties = spec
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("Agent schema should have properties");
        // The forced @agent tool_input sends `mode` and `reasoning_effort` (and
        // AgentInput deserializes them). If the schema hides them, the LLM
        // invents a malformed shape for the Agent tool (e.g. a `raw` wrapper).
        for field in ["mode", "reasoning_effort"] {
            assert!(
                properties.contains_key(field),
                "Agent schema must declare `{field}` so the LLM knows the parameter exists; \
                 current properties: {properties:?}"
            );
        }
        assert_eq!(
            spec.input_schema.get("additionalProperties"),
            Some(&Value::Bool(false)),
            "Agent schema must keep `additionalProperties: false` to reject unknown fields"
        );
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "bash",
            description: "Execute a shell command in Git Bash. Bash syntax: double quotes for interpolation, single quotes for literals, && for chaining, $(...) for subcommands. \
PowerShell via Git Bash: powershell -Command \"...\". Inner strings use single quotes '...' to avoid double-quote nesting conflicts.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "run_in_background": { "type": "boolean" },
                    "dangerouslyDisableSandbox": { "type": "boolean" },
                    "namespaceRestrictions": { "type": "boolean" },
                    "isolateNetwork": { "type": "boolean" },
                    "filesystemMode": { "type": "string", "enum": ["off", "workspace-only", "allow-list"] },
                    "allowedMounts": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "read_file",
            description: "Reads a text file from the workspace.Prioritize task completion and only display file contents when explicitly requested. By default, it returns file content alongside metadata including filePath, checksum, byte count, and line count. \
Set `full: false` to return metadata only for a token-efficient payload. For large files, use `offset` and `limit` to read a specific line window. \
Process all source code files internally, including HTML, CSS, TypeScript, JavaScript, C#, C, C++, Rust, Java, and other common formats. Share only concise summaries and extracted insights. \
For large .txt and .md files exceeding 20,000 bytes, analyze content internally and return only explicitly requested information.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Required. Path of the target file."
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Starting line offset for partial reading."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum number of lines to retrieve."
                    },
                    "full": {
                        "type": "boolean",
                        "default": true,
                        "description": "Returns full content and metadata when true; returns only metadata to reduce token usage when false."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "new_file",
            description: "Create a file. Set `force:true` to overwrite an existing one; use `edit_file` for partial edits.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                    "force": { "type": "boolean", "default": false, "description": "If true, overwrite existing file. Default false rejects existing files." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::WorkspaceWrite,
        },
        ToolSpec {
            name: "edit_file",
            description: "Always call `read_file` first and extract `old_string` verbatim from its output. \
Use this method for all in-place file modifications including edit, append, insert and delete. \
Use `new_file` only for creating new files. \
**Contract:** \
(1) Extract verbatim `old_string` from `read_file` results, with at least 3 unique context lines \
to locate the target accurately. Only the first matching substring is replaced unless `replace_all` \
is enabled. \
(2) The operation fails and leaves the file unchanged if `old_string` is not found. \
`old_string` and `new_string` must have different content. \
(3) For append operations, set `old_string` to the file's trailing content and `new_string` to the \
trailing content plus new lines. For prepend operations, set `old_string` to the file's leading \
content and `new_string` to new content plus the original leading content. \
(4) Verify `contentPreview`, `linesChanged` and `occurrencesMatched` after each modification. \
For code edits, include the full target code block in `old_string` and provide complete, correctly \
indented replacement in `new_string`. Set `new_string` to an empty string to delete matched content. \
(5) Enable `replace_all` only for highly specific `old_string` with at least 3 unique lines. \
If `new_string` is shorter than `old_string`, confirm you are intentionally removing content.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path of the target file. The target file must exist."
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact verbatim substring to replace in the file. Only the first match is replaced by default. Include sufficient surrounding context to ensure unique matching."
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Content used to replace the matched substring. Setting this to an empty string will permanently delete the matched content - use this only when you intend to remove it. For appending content, pair this with the file's original tail content as old_string."
                    },
                    "replace_all": {
                        "type": "boolean",
                        "default": false,
                        "description": "Replace all matching substrings when enabled. Avoid using with short or generic substrings to prevent accidental file damage."
                    },
                    "expected_checksum": {
                        "type": "string",
                        "description": "Optional pre-edit xxh3-64 checksum of the target file. The operation fails if the checksum mismatches, preventing race condition conflicts in multi-agent scenarios."
                    }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::WorkspaceWrite,
        },
        ToolSpec {
            name: "undo",
            description: "Undo prior edit_file operations. Uses non-destructive multi-step rollback: dry-runs all reverse application before modifying any file. Supports undoing all changes to a single file back to a specific patch, or undoing the single most recent patch across all files. Patch files are never deleted; each is marked 'reverted: true' on success.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "diff_path": {
                        "type": "string",
                        "description": "One of: (empty) undo latest patch globally; (file path) undo latest patch for that file; (patch name like '202608010300.patch') undo all changes to that file from that patch forward; ('<file> <patch_name>') explicit file + patch combination."
                    }
                },
                "required": ["diff_path"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::WorkspaceWrite,
        },
        ToolSpec {
            name: "glob_search",
            description: "Fast file search by glob pattern. Supports ** for recursive, {a,b} for alternatives, ? for single char. Returns filenames sorted by modification time. Use `path` to restrict search to a subdirectory.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "grep_search",
            description: "Fast content search by regex pattern. Supports: `glob` to filter files (e.g. \"*.rs\"), `output_mode` (\"files_with_matches\"|\"content\"|\"count\"), `-B`/`-A`/`-C` for context lines, `-n` for line numbers, `-i` for case-insensitive, `type` for file type (e.g. \"rust\"), `multiline` for dot-matches-newline. Returns matching files or content with line numbers.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" },
                    "glob": { "type": "string" },
                    "output_mode": { "type": "string" },
                    "-B": { "type": "integer", "minimum": 0 },
                    "-A": { "type": "integer", "minimum": 0 },
                    "-C": { "type": "integer", "minimum": 0 },
                    "context": { "type": "integer", "minimum": 0 },
                    "-n": { "type": "boolean" },
                    "-i": { "type": "boolean" },
                    "type": { "type": "string" },
                    "head_limit": { "type": "integer", "minimum": 1 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "multiline": { "type": "boolean" }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WebFetch",
            description:
                "Fetch a URL, convert it into readable text, and answer a prompt about it.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri" },
                    "prompt": { "type": "string" }
                },
                "required": ["url", "prompt"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WebFind",
            description:
                "Fetch a URL and return only lines matching a substring, with line/column and trimmed context. Prefer over WebFetch when the answer is a string already on the page (version, token, error code, identifier).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri" },
                    "pattern": { "type": "string", "minLength": 1 },
                    "ignoreCase": { "type": "boolean", "default": true },
                    "maxMatches": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                    "contextChars": { "type": "integer", "minimum": 0, "maximum": 500, "default": 100 }
                },
                "required": ["url", "pattern"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WebSearch",
            description: "Search the web for current info.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "description": "Search query." },
                    "max_results": { "type": "integer", "minimum": 1, "description": "Maximum number of search results to return. Default is provider-dependent." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "Skill",
            description: "Load a skill's instructions from SKILL.md. Call when the user references $name. Use ListSkills to discover available skills.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "skill": { "type": "string", "description": "Skill name to load, e.g. 'frontend-ui-engineering' or '$frontend-ui-engineering'" },
                    "args": { "type": "string", "description": "Optional arguments passed to the skill" }
                },
                "required": ["skill"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "Agent",
            description: "Delegate to a sub-agent referenced by @name. Blocks until the sub-agent completes. subagent_type: 'general-purpose' (full tools), 'Explore' (read-only), 'Plan' (read+StructuredOutput), or 'Verification' (bash+read). 'model' overrides the default. \
For MCP, plugin, or skill work that is multi-step or benefits from isolated context, delegate to a 'general-purpose' sub-agent — it can invoke those tools itself.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string" },
                    "prompt": { "type": "string" },
                    "subagent_type": { "type": "string" },
                    "name": { "type": "string" },
                    "model": { "type": "string" },
                    "mode": { "type": "string", "description": "Agent mode from the definition. Display-only; not consumed by the runtime." },
                    "reasoning_effort": { "type": "string", "description": "Reasoning-effort level (e.g. low/medium/high) forwarded to the provider." },
                    "system_prompt": { "type": "array", "items": { "type": "string" }, "description": "Optional explicit system prompt lines." },
                    "allowed_tools": { "type": "array", "items": { "type": "string" }, "description": "Optional tool allowlist override for the sub-agent." }
                },
                "required": ["description", "prompt"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "Question",
            description: "Ask the user a question to gather information or make a decision. Use when you need clarification, preferences, or additional information. Supports optional multiple choice options. The tool pauses and waits for the user's response. Returns the answer as plain text (single choice/free text) or a JSON array (multiple choice).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string", "description": "The question to ask the user" },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional multiple choice list. User selects one if present."
                    },
                    "allow_multiple": {
                        "type": "boolean",
                        "description": "Allow selecting multiple options (only valid when options are provided)"
                    }
                },
                "required": ["question"],
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "StructuredOutput",
            description: "Return structured output in the requested format.",
            input_schema: json!({
                "type": "object",
                "additionalProperties": true
            }),
            internal: true,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "ListAgents",
            description: "List all available agents (.claude/agents/ and plugin agents). Use this to discover agents you can reference with @name.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "ListSkills",
            description: "List all available skills (.claude/skills/). Use this to discover skills you can load.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "ListPlugins",
            description: "List all installed plugins (.claude/plugins/). Use this to discover available plugins and their capabilities.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            internal: false,
            required_permission: PermissionMode::ReadOnly,
        },
    ]
}
