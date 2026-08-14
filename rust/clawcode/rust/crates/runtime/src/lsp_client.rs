#![allow(clippy::should_implement_trait, clippy::must_use_candidate)]
//! LSP (Language Server Protocol) client registry for tool dispatch.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// Supported LSP actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspAction {
    Diagnostics,
    Hover,
    Definition,
    References,
    Completion,
    Symbols,
    Format,
}

impl LspAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "diagnostics" => Some(Self::Diagnostics),
            "hover" => Some(Self::Hover),
            "definition" | "goto_definition" => Some(Self::Definition),
            "references" | "find_references" => Some(Self::References),
            "completion" | "completions" => Some(Self::Completion),
            "symbols" | "document_symbols" => Some(Self::Symbols),
            "format" | "formatting" => Some(Self::Format),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub severity: String,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub end_line: Option<u32>,
    pub end_character: Option<u32>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspHoverResult {
    pub content: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: Option<String>,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSymbol {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspServerStatus {
    Connected,
    Disconnected,
    Starting,
    Error,
}

impl std::fmt::Display for LspServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => write!(f, "connected"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Starting => write!(f, "starting"),
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerState {
    pub language: String,
    pub status: LspServerStatus,
    pub root_path: Option<String>,
    pub capabilities: Vec<String>,
    pub diagnostics: Vec<LspDiagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct LspRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    servers: HashMap<String, LspServerState>,
}

impl LspRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        language: &str,
        status: LspServerStatus,
        root_path: Option<&str>,
        capabilities: Vec<String>,
    ) {
        let mut inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner.servers.insert(
            language.to_owned(),
            LspServerState {
                language: language.to_owned(),
                status,
                root_path: root_path.map(str::to_owned),
                capabilities,
                diagnostics: Vec::new(),
            },
        );
    }

    pub fn get(&self, language: &str) -> Option<LspServerState> {
        let inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner.servers.get(language).cloned()
    }

    /// Find the appropriate server for a file path based on extension.
    pub fn find_server_for_path(&self, path: &str) -> Option<LspServerState> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let language = match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" => "cpp",
            "rb" => "ruby",
            "lua" => "lua",
            _ => return None,
        };

        self.get(language)
    }

    /// List all registered servers.
    pub fn list_servers(&self) -> Vec<LspServerState> {
        let inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner.servers.values().cloned().collect()
    }

    /// Add diagnostics to a server.
    pub fn add_diagnostics(
        &self,
        language: &str,
        diagnostics: Vec<LspDiagnostic>,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().expect("lsp registry lock poisoned");
        let server = inner
            .servers
            .get_mut(language)
            .ok_or_else(|| format!("LSP server not found for language: {language}"))?;
        server.diagnostics.extend(diagnostics);
        Ok(())
    }

    /// Get diagnostics for a specific file path.
    pub fn get_diagnostics(&self, path: &str) -> Vec<LspDiagnostic> {
        let inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner
            .servers
            .values()
            .flat_map(|s| &s.diagnostics)
            .filter(|d| d.path == path)
            .cloned()
            .collect()
    }

    /// Clear diagnostics for a language server.
    pub fn clear_diagnostics(&self, language: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().expect("lsp registry lock poisoned");
        let server = inner
            .servers
            .get_mut(language)
            .ok_or_else(|| format!("LSP server not found for language: {language}"))?;
        server.diagnostics.clear();
        Ok(())
    }

    /// Disconnect a server.
    pub fn disconnect(&self, language: &str) -> Option<LspServerState> {
        let mut inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner.servers.remove(language)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().expect("lsp registry lock poisoned");
        inner.servers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Dispatch an LSP action and return a structured result.
    pub fn dispatch(
        &self,
        action: &str,
        path: Option<&str>,
        line: Option<u32>,
        character: Option<u32>,
        _query: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let lsp_action =
            LspAction::from_str(action).ok_or_else(|| format!("unknown LSP action: {action}"))?;

        // For diagnostics, we can check existing cached diagnostics
        if lsp_action == LspAction::Diagnostics {
            if let Some(path) = path {
                let diags = self.get_diagnostics(path);
                return Ok(serde_json::json!({
                    "action": "diagnostics",
                    "path": path,
                    "diagnostics": diags,
                    "count": diags.len()
                }));
            }
            // All diagnostics across all servers
            let inner = self.inner.lock().expect("lsp registry lock poisoned");
            let all_diags: Vec<_> = inner
                .servers
                .values()
                .flat_map(|s| &s.diagnostics)
                .collect();
            return Ok(serde_json::json!({
                "action": "diagnostics",
                "diagnostics": all_diags,
                "count": all_diags.len()
            }));
        }

        // For other actions, we need a connected server for the given file
        let path = path.ok_or("path is required for this LSP action")?;
        let server = self
            .find_server_for_path(path)
            .ok_or_else(|| format!("no LSP server available for path: {path}"))?;

        if server.status != LspServerStatus::Connected {
            return Err(format!(
                "LSP server for '{}' is not connected (status: {})",
                server.language, server.status
            ));
        }

        // Return structured placeholder — actual LSP JSON-RPC calls would
        // go through the real LSP process here.
        Ok(serde_json::json!({
            "action": action,
            "path": path,
            "line": line,
            "character": character,
            "language": server.language,
            "status": "dispatched",
            "message": format!("LSP {} dispatched to {} server", action, server.language)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_retrieves_server() {
        let registry = LspRegistry::new();
        registry.register(
            "rust",
            LspServerStatus::Connected,
            Some("/workspace"),
            vec!["hover".into(), "completion".into()],
        );

        let server = registry.get("rust").expect("should exist");
        assert_eq!(server.language, "rust");
        assert_eq!(server.status, LspServerStatus::Connected);
        assert_eq!(server.capabilities.len(), 2);
    }

    #[test]
    fn finds_server_by_file_extension() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        registry.register("typescript", LspServerStatus::Connected, None, vec![]);

        let rs_server = registry.find_server_for_path("src/main.rs").unwrap();
        assert_eq!(rs_server.language, "rust");

        let ts_server = registry.find_server_for_path("src/index.ts").unwrap();
        assert_eq!(ts_server.language, "typescript");

        assert!(registry.find_server_for_path("data.csv").is_none());
    }

    #[test]
    fn manages_diagnostics() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);

        registry
            .add_diagnostics(
                "rust",
                vec![LspDiagnostic {
                    path: "src/main.rs".into(),
                    line: 10,
                    character: 5,
                    severity: "error".into(),
                    message: "mismatched types".into(),
                    source: Some("rust-analyzer".into()),
                }],
            )
            .unwrap();

        let diags = registry.get_diagnostics("src/main.rs");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "mismatched types");

        registry.clear_diagnostics("rust").unwrap();
        assert!(registry.get_diagnostics("src/main.rs").is_empty());
    }

    #[test]
    fn dispatches_diagnostics_action() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        registry
            .add_diagnostics(
                "rust",
                vec![LspDiagnostic {
                    path: "src/lib.rs".into(),
                    line: 1,
                    character: 0,
                    severity: "warning".into(),
                    message: "unused import".into(),
                    source: None,
                }],
            )
            .unwrap();

        let result = registry
            .dispatch("diagnostics", Some("src/lib.rs"), None, None, None)
            .unwrap();
        assert_eq!(result["count"], 1);
    }

    #[test]
    fn dispatches_hover_action() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);

        let result = registry
            .dispatch("hover", Some("src/main.rs"), Some(10), Some(5), None)
            .unwrap();
        assert_eq!(result["action"], "hover");
        assert_eq!(result["language"], "rust");
    }

    #[test]
    fn rejects_action_on_disconnected_server() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Disconnected, None, vec![]);

        assert!(registry
            .dispatch("hover", Some("src/main.rs"), Some(1), Some(0), None)
            .is_err());
    }

    #[test]
    fn rejects_unknown_action() {
        let registry = LspRegistry::new();
        assert!(registry
            .dispatch("unknown_action", Some("file.rs"), None, None, None)
            .is_err());
    }

    #[test]
    fn disconnects_server() {
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        assert_eq!(registry.len(), 1);

        let removed = registry.disconnect("rust");
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn lsp_action_from_str_all_aliases() {
        // given
        let cases = [
            ("diagnostics", Some(LspAction::Diagnostics)),
            ("hover", Some(LspAction::Hover)),
            ("definition", Some(LspAction::Definition)),
            ("goto_definition", Some(LspAction::Definition)),
            ("references", Some(LspAction::References)),
            ("find_references", Some(LspAction::References)),
            ("completion", Some(LspAction::Completion)),
            ("completions", Some(LspAction::Completion)),
            ("symbols", Some(LspAction::Symbols)),
            ("document_symbols", Some(LspAction::Symbols)),
            ("format", Some(LspAction::Format)),
            ("formatting", Some(LspAction::Format)),
            ("unknown", None),
        ];

        // when
        let resolved: Vec<_> = cases
            .into_iter()
            .map(|(input, expected)| (input, LspAction::from_str(input), expected))
            .collect();

        // then
        for (input, actual, expected) in resolved {
            assert_eq!(actual, expected, "unexpected action resolution for {input}");
        }
    }

    #[test]
    fn lsp_server_status_display_all_variants() {
        // given
        let cases = [
            (LspServerStatus::Connected, "connected"),
            (LspServerStatus::Disconnected, "disconnected"),
            (LspServerStatus::Starting, "starting"),
            (LspServerStatus::Error, "error"),
        ];

        // when
        let rendered: Vec<_> = cases
            .into_iter()
            .map(|(status, expected)| (status.to_string(), expected))
            .collect();

        // then
        assert_eq!(
            rendered,
            vec![
                ("connected".to_string(), "connected"),
                ("disconnected".to_string(), "disconnected"),
                ("starting".to_string(), "starting"),
                ("error".to_string(), "error"),
            ]
        );
    }

    #[test]
    fn dispatch_diagnostics_without_path_aggregates() {
        // given
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        registry.register("python", LspServerStatus::Connected, None, vec![]);
        registry
            .add_diagnostics(
                "rust",
                vec![LspDiagnostic {
                    path: "src/lib.rs".into(),
                    line: 1,
                    character: 0,
                    severity: "warning".into(),
                    message: "unused import".into(),
                    source: Some("rust-analyzer".into()),
                }],
            )
            .expect("rust diagnostics should add");
        registry
            .add_diagnostics(
                "python",
                vec![LspDiagnostic {
                    path: "script.py".into(),
                    line: 2,
                    character: 4,
                    severity: "error".into(),
                    message: "undefined name".into(),
                    source: Some("pyright".into()),
                }],
            )
            .expect("python diagnostics should add");

        // when
        let result = registry
            .dispatch("diagnostics", None, None, None, None)
            .expect("aggregate diagnostics should work");

        // then
        assert_eq!(result["action"], "diagnostics");
        assert_eq!(result["count"], 2);
        assert_eq!(result["diagnostics"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn dispatch_non_diagnostics_requires_path() {
        // given
        let registry = LspRegistry::new();

        // when
        let result = registry.dispatch("hover", None, Some(1), Some(0), None);

        // then
        assert_eq!(
            result.expect_err("path should be required"),
            "path is required for this LSP action"
        );
    }

    #[test]
    fn dispatch_no_server_for_path_errors() {
        // given
        let registry = LspRegistry::new();

        // when
        let result = registry.dispatch("hover", Some("notes.md"), Some(1), Some(0), None);

        // then
        let error = result.expect_err("missing server should fail");
        assert!(error.contains("no LSP server available for path: notes.md"));
    }

    #[test]
    fn dispatch_disconnected_server_error_payload() {
        // given
        let registry = LspRegistry::new();
        registry.register("typescript", LspServerStatus::Disconnected, None, vec![]);

        // when
        let result = registry.dispatch("hover", Some("src/index.ts"), Some(3), Some(2), None);

        // then
        let error = result.expect_err("disconnected server should fail");
        assert!(error.contains("typescript"));
        assert!(error.contains("disconnected"));
    }

    #[test]
    fn find_server_for_all_extensions() {
        // given
        let registry = LspRegistry::new();
        for language in [
            "rust",
            "typescript",
            "javascript",
            "python",
            "go",
            "java",
            "c",
            "cpp",
            "ruby",
            "lua",
        ] {
            registry.register(language, LspServerStatus::Connected, None, vec![]);
        }
        let cases = [
            ("src/main.rs", "rust"),
            ("src/index.ts", "typescript"),
            ("src/view.tsx", "typescript"),
            ("src/app.js", "javascript"),
            ("src/app.jsx", "javascript"),
            ("script.py", "python"),
            ("main.go", "go"),
            ("Main.java", "java"),
            ("native.c", "c"),
            ("native.h", "c"),
            ("native.cpp", "cpp"),
            ("native.hpp", "cpp"),
            ("native.cc", "cpp"),
            ("script.rb", "ruby"),
            ("script.lua", "lua"),
        ];

        // when
        let resolved: Vec<_> = cases
            .into_iter()
            .map(|(path, expected)| {
                (
                    path,
                    registry
                        .find_server_for_path(path)
                        .map(|server| server.language),
                    expected,
                )
            })
            .collect();

        // then
        for (path, actual, expected) in resolved {
            assert_eq!(
                actual.as_deref(),
                Some(expected),
                "unexpected mapping for {path}"
            );
        }
    }

    #[test]
    fn find_server_for_path_no_extension() {
        // given
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);

        // when
        let result = registry.find_server_for_path("Makefile");

        // then
        assert!(result.is_none());
    }

    #[test]
    fn list_servers_with_multiple() {
        // given
        let registry = LspRegistry::new();
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        registry.register("typescript", LspServerStatus::Starting, None, vec![]);
        registry.register("python", LspServerStatus::Error, None, vec![]);

        // when
        let servers = registry.list_servers();

        // then
        assert_eq!(servers.len(), 3);
        assert!(servers.iter().any(|server| server.language == "rust"));
        assert!(servers.iter().any(|server| server.language == "typescript"));
        assert!(servers.iter().any(|server| server.language == "python"));
    }

    #[test]
    fn get_missing_server_returns_none() {
        // given
        let registry = LspRegistry::new();

        // when
        let server = registry.get("missing");

        // then
        assert!(server.is_none());
    }

    #[test]
    fn add_diagnostics_missing_language_errors() {
        // given
        let registry = LspRegistry::new();

        // when
        let result = registry.add_diagnostics("missing", vec![]);

        // then
        let error = result.expect_err("missing language should fail");
        assert!(error.contains("LSP server not found for language: missing"));
    }

    #[test]
    fn get_diagnostics_across_servers() {
        // given
        let registry = LspRegistry::new();
        let shared_path = "shared/file.txt";
        registry.register("rust", LspServerStatus::Connected, None, vec![]);
        registry.register("python", LspServerStatus::Connected, None, vec![]);
        registry
            .add_diagnostics(
                "rust",
                vec![LspDiagnostic {
                    path: shared_path.into(),
                    line: 4,
                    character: 1,
                    severity: "warning".into(),
                    message: "warn".into(),
                    source: None,
                }],
            )
            .expect("rust diagnostics should add");
        registry
            .add_diagnostics(
                "python",
                vec![LspDiagnostic {
                    path: shared_path.into(),
                    line: 8,
                    character: 3,
                    severity: "error".into(),
                    message: "err".into(),
                    source: None,
                }],
            )
            .expect("python diagnostics should add");

        // when
        let diagnostics = registry.get_diagnostics(shared_path);

        // then
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "warn"));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "err"));
    }

    #[test]
    fn clear_diagnostics_missing_language_errors() {
        // given
        let registry = LspRegistry::new();

        // when
        let result = registry.clear_diagnostics("missing");

        // then
        let error = result.expect_err("missing language should fail");
        assert!(error.contains("LSP server not found for language: missing"));
    }
}
