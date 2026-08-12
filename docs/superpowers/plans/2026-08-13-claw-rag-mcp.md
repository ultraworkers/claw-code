# claw-rag-mcp Standalone MCP Server — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone, PATH-installable MCP stdio server (`claw-rag-mcp`) that exposes RAG `rag_query` / `rag_stats` / `rag_ingest` / `rag_ingest_status` tools over a SQLite index shared with `claw-rag-service`.

**Architecture:** New independent Rust project at `D:\tempo\claw-rag-mcp` (own Cargo workspace, own git repo, not inside claw-code). It path-depends on the existing `claw-rag-service` lib for indexing/embedding/search logic. Transport is a self-written minimal stdio MCP server (LSP `Content-Length` framing + JSON-RPC `initialize`/`tools/list`/`tools/call`), zero MCP SDK dependency, honoring the repo-wide `forbid(unsafe_code)` lint. Ingest runs as an in-memory async job so `rag_ingest` returns immediately and `rag_ingest_status` polls progress.

**Tech Stack:** Rust 2021, tokio (macros/rt-multi-thread/io-std/io-util/sync/time), serde + serde_json, reqwest 0.12 (json, rustls-tls), `claw-rag-service` (path dep), rusqlite (transitive via claw-rag-service), tempfile (dev).

## Global Constraints

- Project lives at `D:\tempo\claw-rag-mcp` — **not** inside the claw-code repository. It is its own git repo (`git init` in Task 2). The only file changed inside claw-code is `claw-rag-service` (Task 1).
- Path dependency: `claw-rag-service = { path = "D:/tempo/claw-code/rust/crates/claw-rag-service" }`. Use forward slashes.
- Must compile with `cargo build --release` and `cargo test --release`. On this machine every cargo invocation needs MSVC: run via `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo ..."`.
- `#![forbid(unsafe_code)]` applies to every crate compiled (standalone crate sets its own `[lints.rust] unsafe_code = "forbid"`).
- MCP protocol subset only: `initialize`, `tools/list`, `tools/call`. Protocol version `2025-03-26`. Capabilities `{"tools": {}}`. serverInfo name = `claw-rag`, version = `env!("CARGO_PKG_VERSION")`.
- Tool names: `rag_query`, `rag_stats`, `rag_ingest`, `rag_ingest_status`. **No coupling with the original `retrieve_context` naming or behavior.**
- Shared index: env `CLAW_RAG_DB` (default `.claw-rag/index.sqlite`). Embedding env vars: `CLAW_RAG_OPENAI_API_KEY`/`OPENAI_API_KEY`, `CLAW_RAG_EMBEDDING_BASE_URL` (default `https://api.openai.com/v1`), `CLAW_RAG_EMBEDDING_MODEL` (default `text-embedding-3-small`), `CLAW_RAG_MOCK_PROVIDERS=1` for deterministic mock vectors in tests.
- `rag_query`: `top_k` default 8, clamped 1..=32.
- JSON-RPC error codes: `-32700` parse, `-32600` invalid request, `-32601` method not found, `-32602` invalid params.
- Tool-level failures → `isError: true` + text message.
- Ingest jobs: in-memory registry, serialized through a global async mutex, `job_id` is an incrementing integer string (`"1"`, `"2"`, …). No persistence, no HTTP/SSE, no resources/prompts/auth.

---

### Task 1: claw-rag-service — add progress reporting to ingest

**Files:**
- Modify: `D:\tempo\claw-code\rust\crates\claw-rag-service\src\ingest.rs:30-207`
- Modify: `D:\tempo\claw-code\rust\crates\claw-rag-service\src\lib.rs:14`

**Interfaces:**
- Consumes: existing `IngestStats` (files_indexed, chunks_total, embeddings_written).
- Produces: `pub struct IngestProgress { pub files_done: usize, pub files_total: usize, pub chunks_total: usize }` and `pub async fn run_ingest_with_progress<F>(workspaces: &[PathBuf], db_path: &Path, cfg: &EmbedConfig, client: &Client, progress: F) -> Result<IngestStats, String> where F: FnMut(IngestProgress)`. `run_ingest` is preserved as a delegating wrapper (zero caller changes).

- [ ] **Step 1: Write the failing test**

Append a `#[cfg(test)] mod tests` at the end of `ingest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use tempfile::tempdir;

    #[tokio::test]
    async fn run_ingest_with_progress_reports_all_files() {
        std::env::set_var("CLAW_RAG_MOCK_PROVIDERS", "1");
        let dir = tempdir().unwrap();
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("a.rs"), "alpha beta").unwrap();
        std::fs::write(ws.join("b.rs"), "gamma delta").unwrap();
        let db = dir.path().join("idx.sqlite");
        let client = Client::new();
        let cfg = EmbedConfig::mock_from_env().expect("mock embed config");
        let mut seen = Vec::new();
        let st = run_ingest_with_progress(&[ws.clone()], &db, &cfg, &client, |p| seen.push(p))
            .await
            .expect("ingest");
        assert_eq!(st.files_indexed, 2);
        let last = seen.last().expect("progress emitted");
        assert_eq!(last.files_total, 2);
        assert_eq!(last.files_done, 2);
        assert!(last.chunks_total > 0);
        std::env::remove_var("CLAW_RAG_MOCK_PROVIDERS");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-service run_ingest_with_progress"`
Expected: FAIL — `cannot find function run_ingest_with_progress`.

- [ ] **Step 3: Implement `IngestProgress` and `run_ingest_with_progress`**

In `ingest.rs`, after the `IngestStats` struct definition (line 35), add:

```rust
#[derive(Debug, Clone, Copy)]
pub struct IngestProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub chunks_total: usize,
}
```

Replace the entire `run_ingest` function (lines 92-207) with:

```rust
pub async fn run_ingest(
    workspaces: &[PathBuf],
    db_path: &Path,
    cfg: &EmbedConfig,
    client: &Client,
) -> Result<IngestStats, String> {
    run_ingest_with_progress(workspaces, db_path, cfg, client, |_| {}).await
}

pub async fn run_ingest_with_progress<F>(
    workspaces: &[PathBuf],
    db_path: &Path,
    cfg: &EmbedConfig,
    client: &Client,
    mut progress: F,
) -> Result<IngestStats, String>
where
    F: FnMut(IngestProgress),
{
    let conn = open_db(db_path)?;

    let mut all_files: Vec<(String, PathBuf)> = Vec::new();
    let mut seen_paths: Vec<String> = Vec::new();

    for ws in workspaces {
        let workspace = ws
            .canonicalize()
            .map_err(|e| format!("workspace: {}: {e}", ws.display()))?;
        let ws_prefix = workspace.clone();
        let repo_id = repo_id_for_workspace(&workspace);

        for entry in WalkDir::new(&workspace)
            .into_iter()
            .filter_entry(|e| !should_skip_dir(e.path()))
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_text_extension(path) {
                continue;
            }
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.len() > DEFAULT_MAX_FILE_BYTES {
                continue;
            }
            let rel = path
                .strip_prefix(&ws_prefix)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let key = format!("{repo_id}:{rel}");
            seen_paths.push(key.clone());
            all_files.push((key, path.to_path_buf()));
        }
    }

    all_files.sort_by(|a, b| a.0.cmp(&b.0));
    seen_paths.sort();

    let mut stats = IngestStats {
        files_indexed: all_files.len(),
        ..Default::default()
    };

    for (idx, (rel, file)) in all_files.iter().enumerate() {
        progress(IngestProgress {
            files_done: idx + 1,
            files_total: all_files.len(),
            chunks_total: stats.chunks_total,
        });

        let Ok(meta) = std::fs::metadata(file) else {
            continue;
        };
        let size_bytes =
            i64::try_from(meta.len()).map_err(|_| "file size too large".to_string())?;
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0);

        let Ok(raw) = std::fs::read_to_string(file) else {
            continue;
        };

        let content_hash = blake3::hash(raw.as_bytes()).to_hex().to_string();
        if file_is_unchanged(&conn, rel, &content_hash, size_bytes, mtime_ms)? {
            continue;
        }

        // Re-index this file: delete previous chunks (and embeddings) for path.
        delete_file_and_chunks(&conn, rel)?;

        let pieces = chunk_text(&raw, CHUNK_CHARS, CHUNK_OVERLAP);
        if pieces.is_empty() {
            continue;
        }

        let mut batch: Vec<(i32, String)> = Vec::new();
        for (ord, piece) in pieces.into_iter().enumerate() {
            stats.chunks_total += 1;
            let ord_i32 =
                i32::try_from(ord).map_err(|_| "file produced too many chunks".to_string())?;
            batch.push((ord_i32, piece));
            if batch.len() >= EMBED_BATCH {
                flush_path_batch(&conn, rel, &mut batch, client, cfg, &mut stats).await?;
            }
        }
        flush_path_batch(&conn, rel, &mut batch, client, cfg, &mut stats).await?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(0))
            .unwrap_or(0);
        upsert_file_meta(&conn, rel, &content_hash, size_bytes, mtime_ms, now_ms)?;
    }

    // Delete entries for files that no longer exist.
    // (We compare against file list from DB to avoid needing a SQL "NOT IN" temp table.)
    let mut seen_set = std::collections::BTreeSet::new();
    for p in &seen_paths {
        seen_set.insert(p.as_str());
    }
    for p in list_all_files(&conn)? {
        if !seen_set.contains(p.as_str()) {
            delete_file_and_chunks(&conn, &p)?;
        }
    }

    Ok(stats)
}
```

Note: the loop body now borrows `rel`/`file` (`&rel`, `&file`) because `all_files` is iterated by reference; `flush_path_batch` takes `&str` and `&PathBuf` params, so pass `rel` and `file` (auto-deref) as in the code above.

- [ ] **Step 4: Export from lib.rs**

In `D:\tempo\claw-code\rust\crates\claw-rag-service\src\lib.rs`, change line 14:

```rust
pub use ingest::{run_ingest, run_ingest_with_progress, IngestProgress, IngestStats};
```

- [ ] **Step 5: Run the new test and the existing ingest roundtrip test**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-service"`
Expected: PASS (all tests including `run_ingest_with_progress_reports_all_files` and existing `ingest_and_query_roundtrip_mock`).

- [ ] **Step 6: Commit (in claw-code repo)**

```bash
cd D:\tempo\claw-code
git add rust/crates/claw-rag-service/src/ingest.rs rust/crates/claw-rag-service/src/lib.rs
git commit -m "feat(rag-service): add run_ingest_with_progress with IngestProgress"
```

---

### Task 2: Scaffold the standalone project

**Files:**
- Create: `D:\tempo\claw-rag-mcp\Cargo.toml`
- Create: `D:\tempo\claw-rag-mcp\.gitignore`
- Create: `D:\tempo\claw-rag-mcp\src\lib.rs`
- Create: `D:\tempo\claw-rag-mcp\src\main.rs`
- Create: `D:\tempo\claw-rag-mcp\src\framing.rs`
- Create: `D:\tempo\claw-rag-mcp\src\protocol.rs`
- Create: `D:\tempo\claw-rag-mcp\src\server.rs`
- Create: `D:\tempo\claw-rag-mcp\src\tools.rs`

**Interfaces:**
- Consumes: `claw-rag-service` lib from Task 1.
- Produces: compilable crate skeleton with module stubs. Later tasks fill each module.

- [ ] **Step 1: Create the directory and Cargo.toml**

Create `D:\tempo\claw-rag-mcp\Cargo.toml`:

```toml
[package]
name = "claw-rag-mcp"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Standalone MCP server exposing RAG query/stats/ingest over a shared SQLite index"

[workspace]

[dependencies]
claw-rag-service = { path = "D:/tempo/claw-code/rust/crates/claw-rag-service" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std", "io-util", "sync", "time"] }

[dev-dependencies]
tempfile = "3"

[lints.rust]
unsafe_code = "forbid"
```

Create `D:\tempo\claw-rag-mcp\.gitignore`:

```
/target
```

- [ ] **Step 2: Create module stubs**

Create `D:\tempo\claw-rag-mcp\src\framing.rs`:

```rust
//! LSP `Content-Length` framing for MCP stdio transport.
```

Create `D:\tempo\claw-rag-mcp\src\protocol.rs`:

```rust
//! JSON-RPC 2.0 and MCP message types.
```

Create `D:\tempo\claw-rag-mcp\src\server.rs`:

```rust
//! Minimal stdio MCP server: dispatch over LSP-framed JSON-RPC.
```

Create `D:\tempo\claw-rag-mcp\src\tools.rs`:

```rust
//! RAG tool handlers and the async ingest job registry.
```

Create `D:\tempo\claw-rag-mcp\src\lib.rs`:

```rust
//! Standalone RAG MCP server (stdio).
#![forbid(unsafe_code)]

pub mod framing;
pub mod protocol;
pub mod server;
pub mod tools;
```

Create `D:\tempo\claw-rag-mcp\src\main.rs`:

```rust
use std::sync::Arc;

use claw_rag_mcp::tools::{build_server, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = Arc::new(AppState::from_env()?);
    let server = build_server(state);
    server.run(tokio::io::stdin(), tokio::io::stdout()).await?;
    Ok(())
}
```

(These reference `AppState`, `build_server`, and `McpServer::run` which Tasks 4-6 implement; the crate will not compile until then — that is expected and resolved in Task 6.)

- [ ] **Step 3: git init and verify the crate starts building**

Run:
```bash
cd D:\tempo\claw-rag-mcp
git init
git add -A
git commit -m "chore: scaffold claw-rag-mcp crate"
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo build --release"
```
Expected: build fails with "cannot find function `build_server`" — confirming the path dependency on `claw-rag-service` resolves (its deps, incl. bundled rusqlite, compile successfully). If it fails earlier on `claw-rag-service`, the path in `Cargo.toml` is wrong.

- [ ] **Step 4: Commit scaffold**

```bash
git add -A
git commit -m "chore: verify claw-rag-service path dependency compiles"
```
(Only run after confirming the dependency resolved; if the crate compiles fully, fine — commit either way.)

---

### Task 3: protocol module — JSON-RPC and MCP types + framing

**Files:**
- Modify: `D:\tempo\claw-rag-mcp\src\protocol.rs`
- Modify: `D:\tempo\claw-rag-mcp\src\framing.rs`

**Interfaces:**
- Consumes: serde, serde_json.
- Produces: `JsonRpcId` (untagged enum Null/Number/String), `JsonRpcRequest<T>`, `JsonRpcResponse<T>`, `JsonRpcError`, `McpTool`, `McpInitializeResult`, `McpServerInfo`, `McpListToolsResult`, `McpToolCallParams`, `McpToolCallResult`, `McpToolCallContent`, `PROTOCOL_VERSION: &str = "2025-03-26"`. Plus `framing::read_frame<R: AsyncBufRead + Unpin>(&mut R) -> io::Result<Option<Vec<u8>>>` and `framing::write_frame<W: AsyncWrite + Unpin>(&mut W, &[u8]) -> io::Result<()>`.

- [ ] **Step 1: Write the failing tests**

Append to `protocol.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serialize_roundtrip() {
        let req = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(7),
            method: "tools/list".to_string(),
            params: None,
        };
        let s = serde_json::to_string(&req).expect("serialize");
        assert!(s.contains("\"id\":7"));
        let back: JsonRpcRequest<JsonValue> = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.method, "tools/list");
        assert_eq!(back.id, JsonRpcId::Number(7));
    }

    #[test]
    fn tool_call_result_uses_standard_text_shape() {
        let result = McpToolCallResult {
            content: vec![McpToolCallContent::Text {
                text: "hello".to_string(),
            }],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };
        let v = serde_json::to_value(&result).expect("serialize");
        assert_eq!(v["content"][0]["type"], "text");
        assert_eq!(v["content"][0]["text"], "hello");
        assert_eq!(v["isError"], false);
    }

    #[test]
    fn id_supports_string_and_null() {
        let id = JsonRpcId::String("abc".to_string());
        let v = serde_json::to_value(&id).expect("serialize");
        assert_eq!(v, json!("abc"));
        assert_eq!(
            serde_json::from_value::<JsonRpcId>(json!(null)).expect("null id"),
            JsonRpcId::Null
        );
    }
}
```

Append to `framing.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{BufReader, Cursor};

    #[tokio::test]
    async fn frame_roundtrip() {
        let body = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}";
        let mut buf = Vec::new();
        write_frame(&mut buf, body).await.expect("write");
        assert!(buf.starts_with(b"Content-Length: "));
        let mut reader = BufReader::new(Cursor::new(&buf[..]));
        let got = read_frame(&mut reader).await.expect("read").expect("frame present");
        assert_eq!(got, body);
    }

    #[tokio::test]
    async fn read_frame_eof_returns_none() {
        let mut reader = BufReader::new(Cursor::new(&b""[..]));
        assert!(read_frame(&mut reader).await.expect("read").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib protocol::tests framing::tests"`
Expected: FAIL — types/functions not defined (compile error).

- [ ] **Step 3: Implement protocol.rs**

Replace the content of `protocol.rs` with:

```rust
//! JSON-RPC 2.0 and MCP message types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Protocol version advertised during `initialize`.
pub const PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Null,
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    pub params: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpInitializeResult {
    pub protocol_version: String,
    pub capabilities: JsonValue,
    pub server_info: McpServerInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpListToolsResult {
    pub tools: Vec<McpTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpToolCallResult {
    pub content: Vec<McpToolCallContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpToolCallContent {
    Text { text: String },
}

#[allow(dead_code)]
fn _assert_send_sync(_: &dyn std::marker::Send) {}

/// Build a `BTreeMap` used as the tool-result text payload (compat helper).
#[allow(dead_code)]
pub fn text_content_map(text: String) -> BTreeMap<String, JsonValue> {
    let mut map = BTreeMap::new();
    map.insert("text".to_string(), JsonValue::String(text));
    map
}
```

- [ ] **Step 4: Implement framing.rs**

Replace the content of `framing.rs` with:

```rust
//! LSP `Content-Length` framing for MCP stdio transport.

use std::io;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Read one framed JSON-RPC payload.
///
/// Returns `Ok(None)` on clean EOF before any header bytes have been read.
pub async fn read_frame<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut first_header = true;
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            if first_header {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MCP stdio stream closed while reading headers",
            ));
        }
        first_header = false;
        if line == "\r\n" || line == "\n" {
            break;
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("Content-Length") {
                let parsed = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                content_length = Some(parsed);
            }
        }
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;
    let mut payload = vec![0_u8; content_length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Write a single LSP-framed payload.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    body: &[u8],
) -> io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib protocol::tests framing::tests"`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add src/protocol.rs src/framing.rs
git commit -m "feat: JSON-RPC/MCP types and LSP framing"
```

---

### Task 4: server module — dispatch + run loop

**Files:**
- Modify: `D:\tempo\claw-rag-mcp\src\server.rs`

**Interfaces:**
- Consumes: `protocol` module from Task 3.
- Produces: `pub type ToolCallHandler = Box<dyn Fn(&str, &JsonValue) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>`; `pub struct McpServerSpec { server_name, server_version, tools: Vec<McpTool>, tool_handler: ToolCallHandler }`; `pub struct McpServer` with `new(spec)`, `async fn dispatch(&self, request: JsonRpcRequest<JsonValue>) -> JsonRpcResponse<JsonValue>`, and `async fn run<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(&self, reader: R, writer: W) -> io::Result<()>`.

- [ ] **Step 1: Write the failing tests**

Append to `server.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{McpTool, PROTOCOL_VERSION};
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

    fn echo_server() -> McpServer {
        let tool = McpTool {
            name: "echo".to_string(),
            description: Some("Echo".to_string()),
            input_schema: Some(json!({"type": "object"})),
            annotations: None,
            meta: None,
        };
        let spec = McpServerSpec {
            server_name: "claw-rag".to_string(),
            server_version: "0.0.0".to_string(),
            tools: vec![tool],
            tool_handler: Box::new(|name, args| {
                Box::pin(async move { Ok(format!("called {name} with {args}")) })
            }),
        };
        McpServer::new(spec)
    }

    #[tokio::test]
    async fn dispatch_initialize_returns_server_info() {
        let request = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(1),
            method: "initialize".to_string(),
            params: None,
        };
        let response = echo_server().dispatch(request).await;
        assert!(response.error.is_none());
        let result = response.result.expect("result");
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], "claw-rag");
    }

    #[tokio::test]
    async fn dispatch_tools_list_returns_tools() {
        let request = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(2),
            method: "tools/list".to_string(),
            params: None,
        };
        let response = echo_server().dispatch(request).await;
        let result = response.result.expect("result");
        assert_eq!(result["tools"][0]["name"], "echo");
    }

    #[tokio::test]
    async fn dispatch_tools_call_wraps_handler_output() {
        let request = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(3),
            method: "tools/call".to_string(),
            params: Some(json!({"name": "echo", "arguments": {"text": "hi"}})),
        };
        let response = echo_server().dispatch(request).await;
        let result = response.result.expect("result");
        assert_eq!(result["isError"], false);
        assert_eq!(result["content"][0]["type"], "text");
        assert!(result["content"][0]["text"].as_str().unwrap().starts_with("called echo"));
    }

    #[tokio::test]
    async fn dispatch_tools_call_surfaces_handler_error() {
        let tool = McpTool {
            name: "broken".to_string(),
            description: None,
            input_schema: None,
            annotations: None,
            meta: None,
        };
        let spec = McpServerSpec {
            server_name: "x".to_string(),
            server_version: "0.0.0".to_string(),
            tools: vec![tool],
            tool_handler: Box::new(|_, _| Box::pin(async move { Err("boom".to_string()) })),
        };
        let server = McpServer::new(spec);
        let request = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(4),
            method: "tools/call".to_string(),
            params: Some(json!({"name": "broken"})),
        };
        let response = server.dispatch(request).await;
        let result = response.result.expect("result");
        assert_eq!(result["isError"], true);
        assert_eq!(result["content"][0]["text"], "boom");
    }

    #[tokio::test]
    async fn dispatch_unknown_method_returns_error() {
        let request = JsonRpcRequest::<JsonValue> {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(5),
            method: "nonsense".to_string(),
            params: None,
        };
        let response = echo_server().dispatch(request).await;
        let error = response.error.expect("error");
        assert_eq!(error.code, -32601);
    }

    #[tokio::test]
    async fn run_roundtrip_over_duplex() {
        let server = echo_server();
        let (client, srv) = tokio::io::duplex(1 << 16);
        let server_task = tokio::spawn(server.run(srv.clone(), srv));

        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut c = client;
        c.write_all(header.as_bytes()).await.expect("write header");
        c.write_all(body.as_bytes()).await.expect("write body");

        let mut line = String::new();
        let mut reader = BufReader::new(&mut c);
        reader.read_line(&mut line).await.expect("read len header");
        let cl: usize = line.trim().split(':').nth(1).unwrap().trim().parse().expect("parse len");
        line.clear();
        reader.read_line(&mut line).await.expect("read blank");
        let mut payload = vec![0_u8; cl];
        reader.read_exact(&mut payload).await.expect("read body");
        let v: JsonValue = serde_json::from_slice(&payload).expect("json");
        assert_eq!(v["result"]["serverInfo"]["name"], "claw-rag");

        drop(c);
        let _ = server_task.await;
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib server::tests"`
Expected: FAIL — `McpServer`/`McpServerSpec`/`ToolCallHandler` undefined.

- [ ] **Step 3: Implement server.rs**

Replace the content of `server.rs` with:

```rust
//! Minimal stdio MCP server: dispatch over LSP-framed JSON-RPC.

use std::future::Future;
use std::io;
use std::pin::Pin;

use serde_json::{json, Value as JsonValue};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader};

use crate::framing;
use crate::protocol::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, McpInitializeResult,
    McpListToolsResult, McpServerInfo, McpTool, McpToolCallContent, McpToolCallParams,
    McpToolCallResult, PROTOCOL_VERSION,
};

/// Synchronous-triggering, async-returning handler for `tools/call`.
///
/// `Ok(text)` yields a single text content block with `isError: false`;
/// `Err(message)` yields text with `isError: true`.
pub type ToolCallHandler =
    Box<dyn Fn(&str, &JsonValue) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>;

pub struct McpServerSpec {
    pub server_name: String,
    pub server_version: String,
    pub tools: Vec<McpTool>,
    pub tool_handler: ToolCallHandler,
}

pub struct McpServer {
    spec: McpServerSpec,
}

impl McpServer {
    #[must_use]
    pub fn new(spec: McpServerSpec) -> Self {
        Self { spec }
    }

    /// Dispatch one JSON-RPC request, returning the response.
    pub async fn dispatch(
        &self,
        request: JsonRpcRequest<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let id = request.id.clone();
        match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, request.params).await,
            other => error_response(id, -32601, &format!("method not found: {other}")),
        }
    }

    /// Read frames from `reader`, dispatch, write responses to `writer`.
    pub async fn run<R, W>(&self, reader: R, writer: W) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut reader = BufReader::new(reader);
        let mut writer = writer;
        loop {
            let Some(payload) = framing::read_frame(&mut reader).await? else {
                return Ok(());
            };
            let value: JsonValue = match serde_json::from_slice(&payload) {
                Ok(value) => value,
                Err(error) => {
                    let response = JsonRpcResponse::<JsonValue> {
                        jsonrpc: "2.0".to_string(),
                        id: JsonRpcId::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("parse error: {error}"),
                            data: None,
                        }),
                    };
                    let body = serde_json::to_vec(&response)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                    framing::write_frame(&mut writer, &body).await?;
                    continue;
                }
            };

            if value.get("id").is_none() {
                // Notification: no reply.
                continue;
            }

            let request: JsonRpcRequest<JsonValue> = match serde_json::from_value(value) {
                Ok(request) => request,
                Err(error) => {
                    let response = JsonRpcResponse::<JsonValue> {
                        jsonrpc: "2.0".to_string(),
                        id: JsonRpcId::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32600,
                            message: format!("invalid request: {error}"),
                            data: None,
                        }),
                    };
                    let body = serde_json::to_vec(&response)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                    framing::write_frame(&mut writer, &body).await?;
                    continue;
                }
            };

            let response = self.dispatch(request).await;
            let body = serde_json::to_vec(&response)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            framing::write_frame(&mut writer, &body).await?;
        }
    }

    fn handle_initialize(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result = McpInitializeResult {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: json!({ "tools": {} }),
            server_info: McpServerInfo {
                name: self.spec.server_name.clone(),
                version: self.spec.server_version.clone(),
            },
        };
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: serde_json::to_value(result).ok(),
            error: None,
        }
    }

    fn handle_tools_list(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result = McpListToolsResult {
            tools: self.spec.tools.clone(),
            next_cursor: None,
        };
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: serde_json::to_value(result).ok(),
            error: None,
        }
    }

    async fn handle_tools_call(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for tools/call");
        };
        let call: McpToolCallParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(id, &format!("invalid tools/call params: {error}"));
            }
        };
        let arguments = call.arguments.unwrap_or_else(|| json!({}));
        let tool_result = (self.spec.tool_handler)(&call.name, &arguments).await;
        let (text, is_error) = match tool_result {
            Ok(text) => (text, false),
            Err(message) => (message, true),
        };
        let call_result = McpToolCallResult {
            content: vec![McpToolCallContent::Text { text }],
            structured_content: None,
            is_error: Some(is_error),
            meta: None,
        };
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: serde_json::to_value(call_result).ok(),
            error: None,
        }
    }
}

fn invalid_params_response(id: JsonRpcId, message: &str) -> JsonRpcResponse<JsonValue> {
    error_response(id, -32602, message)
}

fn error_response(id: JsonRpcId, code: i32, message: &str) -> JsonRpcResponse<JsonValue> {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}

#[allow(dead_code)]
fn _assert_async_read<B: AsyncBufRead>() {}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib server::tests"`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/server.rs
git commit -m "feat: minimal MCP stdio server dispatch and run loop"
```

---

### Task 5: tools module — query, stats, and formatting

**Files:**
- Modify: `D:\tempo\claw-rag-mcp\src\tools.rs`

**Interfaces:**
- Consumes: `protocol::McpTool`; claw-rag-service `open_db`, `chunk_count`, `query_index`, `QueryRequest`, `QueryResponse`, `RagHit`, `EmbedConfig`.
- Produces: `pub struct AppState { pub db_path: PathBuf, pub client: reqwest::Client, pub cfg: EmbedConfig, pub jobs: Arc<Mutex<HashMap<String, JobState>>>, pub ingest_lock: Arc<tokio::sync::Mutex<()>>, pub next_job_id: AtomicU64 }`; `impl AppState { pub fn from_env() -> Result<Self, String> }`; `pub fn rag_tools() -> Vec<McpTool>`; `pub fn handle_tool(state: Arc<AppState>, name: &str, args: JsonValue) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>`; `pub fn build_server(state: Arc<AppState>) -> McpServer`; `fn format_query_result(&QueryResponse) -> String`; `fn format_job_status(job_id: &str, Option<&JobStatus>) -> String`.

- [ ] **Step 1: Write the failing tests**

Append to `tools.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_query_result_renders_hits() {
        let resp = QueryResponse {
            hits: vec![RagHit {
                path: "repo-ab:src/main.rs".to_string(),
                snippet: "line one\nline two".to_string(),
                score: Some(0.912_345_6),
            }],
            phase: "1-sqlite",
        };
        let s = format_query_result(&resp);
        assert!(s.contains("phase: 1-sqlite"));
        assert!(s.contains("score=0.9123"));
        assert!(s.contains("repo-ab:src/main.rs"));
        assert!(s.contains("line one"));
    }

    #[test]
    fn format_query_result_empty_reports_no_hits() {
        let resp = QueryResponse {
            hits: Vec::new(),
            phase: "1-sqlite-no-db",
        };
        let s = format_query_result(&resp);
        assert!(s.contains("phase: 1-sqlite-no-db"));
        assert!(s.contains("(no hits)"));
    }

    #[test]
    fn format_job_status_covers_all_states() {
        let running = format_job_status(
            "1",
            Some(&JobStatus::Running {
                files_done: 3,
                files_total: 10,
                chunks_total: 12,
            }),
        );
        assert!(running.contains("status: running"));
        assert!(running.contains("3/10"));
        assert!(running.contains("chunks_total: 12"));

        let done = format_job_status(
            "2",
            Some(&JobStatus::Done {
                files_indexed: 5,
                chunks_total: 40,
                embeddings_written: 40,
            }),
        );
        assert!(done.contains("status: done"));
        assert!(done.contains("files_indexed: 5"));

        let failed = format_job_status("3", Some(&JobStatus::Failed("disk full".to_string())));
        assert!(failed.contains("status: failed"));
        assert!(failed.contains("disk full"));

        let unknown = format_job_status("99", None);
        assert!(unknown.contains("status: unknown"));
    }

    #[tokio::test]
    async fn rag_tools_list_has_four_tools() {
        let tools = rag_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["rag_query", "rag_stats", "rag_ingest", "rag_ingest_status"]);
    }

    #[tokio::test]
    async fn handle_tool_unknown_tool_errors() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState {
            db_path: dir.path().join("idx.sqlite"),
            client: reqwest::Client::new(),
            cfg: EmbedConfig {
                api_key: "mock".into(),
                base_url: "mock://".into(),
                model: "mock-embedding".into(),
            },
            jobs: Arc::new(Mutex::new(HashMap::new())),
            ingest_lock: Arc::new(tokio::sync::Mutex::new(())),
            next_job_id: AtomicU64::new(0),
        });
        let out = handle_tool(state, "nope", json!({})).await;
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("unknown tool"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib tools::tests"`
Expected: FAIL — types/functions undefined.

- [ ] **Step 3: Implement tools.rs (query/stats/schemas; ingest stays stub for Task 6)**

Replace the content of `tools.rs` with:

```rust
//! RAG tool handlers and the async ingest job registry.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use claw_rag_service::{
    chunk_count, open_db, query_index, EmbedConfig, QueryRequest, QueryResponse, RagHit,
};
use serde_json::{json, Value as JsonValue};
use tokio::sync::Mutex as AsyncMutex;

use crate::protocol::McpTool;
use crate::server::{McpServer, McpServerSpec};

const DB_ENV: &str = "CLAW_RAG_DB";
const DEFAULT_DB: &str = ".claw-rag/index.sqlite";
const TOP_K_MAX: u32 = 32;

#[derive(Debug, Clone)]
pub enum JobStatus {
    Running {
        files_done: usize,
        files_total: usize,
        chunks_total: usize,
    },
    Done {
        files_indexed: usize,
        chunks_total: usize,
        embeddings_written: usize,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct JobState {
    pub status: JobStatus,
}

pub struct AppState {
    pub db_path: PathBuf,
    pub client: reqwest::Client,
    pub cfg: EmbedConfig,
    pub jobs: Arc<Mutex<HashMap<String, JobState>>>,
    pub ingest_lock: Arc<AsyncMutex<()>>,
    pub next_job_id: AtomicU64,
}

impl AppState {
    pub fn from_env() -> Result<Self, String> {
        let cfg = if let Some(c) = EmbedConfig::mock_from_env() {
            c
        } else {
            EmbedConfig::from_env()?
        };
        let db_path = std::env::var(DB_ENV).unwrap_or_else(|_| DEFAULT_DB.to_string());
        Ok(Self {
            db_path: PathBuf::from(db_path),
            client: reqwest::Client::new(),
            cfg,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            ingest_lock: Arc::new(AsyncMutex::new(())),
            next_job_id: AtomicU64::new(0),
        })
    }
}

pub fn rag_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "rag_query".to_string(),
            description: Some(
                "Semantic search over the workspace RAG index. Returns ranked file paths and snippets with scores."
                    .to_string(),
            ),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural-language query" },
                    "top_k": { "type": "integer", "description": "Max hits (default 8, capped at 32)" }
                },
                "required": ["query"]
            })),
            annotations: None,
            meta: None,
        },
        McpTool {
            name: "rag_stats".to_string(),
            description: Some(
                "Report indexed chunk count and index phase (no embedding call).".to_string(),
            ),
            input_schema: Some(json!({
                "type": "object",
                "properties": {}
            })),
            annotations: None,
            meta: None,
        },
        McpTool {
            name: "rag_ingest".to_string(),
            description: Some(
                "Index workspaces into the shared SQLite index asynchronously. Returns a job_id; poll rag_ingest_status for progress."
                    .to_string(),
            ),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "workspaces": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Absolute workspace paths to index"
                    }
                },
                "required": ["workspaces"]
            })),
            annotations: None,
            meta: None,
        },
        McpTool {
            name: "rag_ingest_status".to_string(),
            description: Some(
                "Poll the status of an ingest job started by rag_ingest.".to_string(),
            ),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Job id returned by rag_ingest" }
                },
                "required": ["job_id"]
            })),
            annotations: None,
            meta: None,
        },
    ]
}

pub fn build_server(state: Arc<AppState>) -> McpServer {
    let spec = McpServerSpec {
        server_name: "claw-rag".to_string(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        tools: rag_tools(),
        tool_handler: Box::new(move |name, args| handle_tool(state.clone(), name, args.clone())),
    };
    McpServer::new(spec)
}

pub fn handle_tool(
    state: Arc<AppState>,
    name: &str,
    args: JsonValue,
) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
    Box::pin(async move {
        match name {
            "rag_query" => rag_query(&state, &args).await,
            "rag_stats" => rag_stats(&state, &args).await,
            "rag_ingest" => rag_ingest(&state, &args),
            "rag_ingest_status" => rag_ingest_status(&state, &args),
            other => Err(format!("unknown tool: {other}")),
        }
    })
}

fn format_query_result(r: &QueryResponse) -> String {
    let mut out = format!("phase: {}\n", r.phase);
    if r.hits.is_empty() {
        out.push_str("(no hits)\n");
        return out;
    }
    for (i, h) in r.hits.iter().enumerate() {
        let mut header = format!("{}. ", i + 1);
        if let Some(s) = h.score {
            header.push_str(&format!("score={s:.4} "));
        }
        header.push_str(&format!("path={}\n", h.path));
        out.push_str(&header);
        for line in h.snippet.lines().take(32) {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
        if h.snippet.lines().count() > 32 {
            out.push_str("    …\n");
        }
        out.push('\n');
    }
    out
}

fn format_job_status(job_id: &str, status: Option<&JobStatus>) -> String {
    let Some(status) = status else {
        return format!("status: unknown\njob_id: {job_id}");
    };
    match status {
        JobStatus::Running {
            files_done,
            files_total,
            chunks_total,
        } => format!(
            "status: running\nfiles_done: {files_done}/{files_total}\nchunks_total: {chunks_total}"
        ),
        JobStatus::Done {
            files_indexed,
            chunks_total,
            embeddings_written,
        } => format!(
            "status: done\nfiles_indexed: {files_indexed}\nchunks_total: {chunks_total}\nembeddings_written: {embeddings_written}"
        ),
        JobStatus::Failed(e) => format!("status: failed\nerror: {e}"),
    }
}

async fn rag_query(state: &AppState, args: &JsonValue) -> Result<String, String> {
    let q = args
        .get("query")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "rag_query: missing or empty query".to_string())?;
    let top_k = args
        .get("top_k")
        .and_then(JsonValue::as_u64)
        .map(|n| n as u32)
        .unwrap_or(8)
        .clamp(1, TOP_K_MAX);
    let req = QueryRequest {
        query: q.to_string(),
        top_k,
    };
    let resp = query_index(&state.db_path, &state.client, &state.cfg, &req)
        .await
        .map_err(|e| format!("rag_query: {e}"))?;
    Ok(format_query_result(&resp))
}

async fn rag_stats(state: &AppState, _args: &JsonValue) -> Result<String, String> {
    let db = state.db_path.clone();
    if !db.is_file() {
        return Ok("chunks: 0\nphase: 1-sqlite-no-db".to_string());
    }
    let n = tokio::task::spawn_blocking(move || {
        let conn = open_db(&db).map_err(|e| e.to_string())?;
        chunk_count(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("rag_stats: join: {e}"))?
    .map_err(|e| format!("rag_stats: {e}"))?;
    let phase = if n == 0 { "1-sqlite-empty" } else { "1-sqlite" };
    Ok(format!("chunks: {n}\nphase: {phase}"))
}

fn rag_ingest(state: &Arc<AppState>, args: &JsonValue) -> Result<String, String> {
    // Implemented in Task 6.
    let _ = (state, args);
    Err("rag_ingest: not yet implemented".to_string())
}

fn rag_ingest_status(state: &AppState, args: &JsonValue) -> Result<String, String> {
    let jid = args
        .get("job_id")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "rag_ingest_status: missing job_id".to_string())?;
    let jobs = state
        .jobs
        .lock()
        .map_err(|_| "rag_ingest_status: registry poisoned".to_string())?;
    let status = jobs.get(jid).map(|j| &j.status);
    Ok(format_job_status(jid, status))
}
```

Note: `rag_ingest` is a stub here; Task 6 replaces it. `rag_ingest_status` already reads the (currently always-empty) registry, which is fine for these tests.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib tools::tests"`
Expected: PASS (5 tests). The `rag_ingest` stub returns an error — no test calls it yet.

- [ ] **Step 5: Commit**

```bash
git add src/tools.rs
git commit -m "feat: rag_query, rag_stats tool handlers and formatting"
```

---

### Task 6: tools module — async ingest job

**Files:**
- Modify: `D:\tempo\claw-rag-mcp\src\tools.rs`

**Interfaces:**
- Consumes: `AppState`, `JobStatus`, `JobState`, `handle_tool` dispatch from Task 5; claw-rag-service `run_ingest_with_progress`, `IngestProgress` from Task 1.
- Produces: working `rag_ingest` (spawns background task, returns `job_id: N`) and working `rag_ingest_status` (progress).

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `tools.rs`:

```rust
#[tokio::test]
async fn ingest_job_lifecycle_with_mock_embeddings() {
    std::env::set_var("CLAW_RAG_MOCK_PROVIDERS", "1");
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("note.md"), "hello RAG service mock content").unwrap();

    let state = Arc::new(AppState {
        db_path: dir.path().join("idx.sqlite"),
        client: reqwest::Client::new(),
        cfg: EmbedConfig::mock_from_env().expect("mock embed config"),
        jobs: Arc::new(Mutex::new(HashMap::new())),
        ingest_lock: Arc::new(AsyncMutex::new(())),
        next_job_id: AtomicU64::new(0),
    });

    let out = handle_tool(
        state.clone(),
        "rag_ingest",
        json!({"workspaces": [ws.to_string_lossy().to_string()]}),
    )
    .await
    .expect("rag_ingest returns job_id");
    assert!(out.starts_with("job_id: "), "unexpected: {out}");
    let job_id = out.trim().strip_prefix("job_id: ").unwrap().to_string();

    let mut done = false;
    for _ in 0..100 {
        let s = handle_tool(
            state.clone(),
            "rag_ingest_status",
            json!({"job_id": job_id}),
        )
        .await
        .expect("status call");
        if s.contains("status: done") {
            done = true;
            assert!(s.contains("files_indexed: 1"), "stats: {s}");
            break;
        }
        if s.contains("status: failed") {
            panic!("ingest failed: {s}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(done, "ingest never finished");

    let q = handle_tool(state.clone(), "rag_query", json!({"query": "RAG service"}))
        .await
        .expect("query");
    assert!(q.contains("phase: 1-sqlite"), "query: {q}");
    assert!(q.contains("note.md"), "query: {q}");

    let st = handle_tool(state.clone(), "rag_stats", json!({}))
        .await
        .expect("stats");
    assert!(st.contains("chunks: "), "stats: {st}");

    let unk = handle_tool(
        state.clone(),
        "rag_ingest_status",
        json!({"job_id": "999"}),
    )
    .await
    .expect("unknown job");
    assert!(unk.contains("status: unknown"));

    std::env::remove_var("CLAW_RAG_MOCK_PROVIDERS");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release -p claw-rag-mcp --lib tools::tests::ingest_job_lifecycle_with_mock_embeddings"`
Expected: FAIL — `rag_ingest` returns "not yet implemented".

- [ ] **Step 3: Implement rag_ingest**

Replace the `rag_ingest` stub in `tools.rs` with:

```rust
fn rag_ingest(state: &Arc<AppState>, args: &JsonValue) -> Result<String, String> {
    let ws = args
        .get("workspaces")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "rag_ingest: missing workspaces array".to_string())?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for p in ws {
        let s = p
            .as_str()
            .ok_or_else(|| "rag_ingest: workspaces entries must be strings".to_string())?;
        paths.push(PathBuf::from(s));
    }
    if paths.is_empty() {
        return Err("rag_ingest: workspaces is empty".to_string());
    }

    let job_id = state
        .next_job_id
        .fetch_add(1, Ordering::SeqCst)
        .to_string();
    {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| "rag_ingest: registry poisoned".to_string())?;
        jobs.insert(
            job_id.clone(),
            JobState {
                status: JobStatus::Running {
                    files_done: 0,
                    files_total: 0,
                    chunks_total: 0,
                },
            },
        );
    }

    let db = state.db_path.clone();
    let cfg = state.cfg.clone();
    let client = state.client.clone();
    let jobs = state.jobs.clone();
    let lock = state.ingest_lock.clone();
    let jid = job_id.clone();

    tokio::spawn(async move {
        let _guard = lock.lock().await;
        let result = run_ingest_with_progress(&paths, &db, &cfg, &client, |p| {
            if let Ok(mut jobs) = jobs.lock() {
                if let Some(j) = jobs.get_mut(&jid) {
                    j.status = JobStatus::Running {
                        files_done: p.files_done,
                        files_total: p.files_total,
                        chunks_total: p.chunks_total,
                    };
                }
            }
        })
        .await;
        let status = match result {
            Ok(s) => JobStatus::Done {
                files_indexed: s.files_indexed,
                chunks_total: s.chunks_total,
                embeddings_written: s.embeddings_written,
            },
            Err(e) => JobStatus::Failed(e),
        };
        if let Ok(mut jobs) = jobs.lock() {
            if let Some(j) = jobs.get_mut(&jid) {
                j.status = status;
            }
        }
    });

    Ok(format!("job_id: {job_id}"))
}
```

Add the missing import at the top of `tools.rs` (change the claw-rag-service use line):

```rust
use claw_rag_service::{
    chunk_count, open_db, query_index, run_ingest_with_progress, EmbedConfig, QueryRequest,
    QueryResponse, RagHit,
};
```

- [ ] **Step 4: Wire up main.rs and run all tests**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release"`
Expected: PASS — all unit tests plus `ingest_job_lifecycle_with_mock_embeddings`. This is the first full crate build including `main.rs` (`build_server`/`AppState::from_env`/`McpServer::run` now all exist).

- [ ] **Step 5: Commit**

```bash
git add src/tools.rs src/main.rs
git commit -m "feat: async ingest jobs with progress tracking"
```

---

### Task 7: full-flow integration test over the wire

**Files:**
- Create: `D:\tempo\claw-rag-mcp\tests\full_flow.rs`

**Interfaces:**
- Consumes: `build_server`, `AppState`, `framing` (via server run over duplex), claw-rag-service mock embeddings.
- Produces: end-to-end proof that an external MCP host can `initialize` → `tools/list` → `rag_ingest` → poll → `rag_query`/`rag_stats` over the real wire protocol.

- [ ] **Step 1: Write the failing test**

Create `D:\tempo\claw-rag-mcp\tests\full_flow.rs`:

```rust
use std::sync::Arc;

use claw_rag_mcp::tools::{build_server, AppState};
use serde_json::{json, Value as JsonValue};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream};

async fn request(client: &mut DuplexStream, payload: &JsonValue) -> JsonValue {
    let body = serde_json::to_vec(payload).expect("serialize");
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    client.write_all(header.as_bytes()).await.expect("write header");
    client.write_all(&body).await.expect("write body");

    let mut line = String::new();
    let mut reader = BufReader::new(&mut *client);
    reader.read_line(&mut line).await.expect("read len");
    let cl: usize = line.trim().split(':').nth(1).unwrap().trim().parse().expect("len");
    line.clear();
    reader.read_line(&mut line).await.expect("read blank");
    let mut payload = vec![0_u8; cl];
    reader.read_exact(&mut payload).await.expect("read body");
    serde_json::from_slice(&payload).expect("json response")
}

#[tokio::test]
async fn end_to_end_mcp_session() {
    std::env::set_var("CLAW_RAG_MOCK_PROVIDERS", "1");
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().join("ws");
    std::fs::create_dir_all(&ws).expect("mkdir ws");
    std::fs::write(ws.join("note.md"), "hello RAG service mock content").expect("write note");

    let state = Arc::new(AppState {
        db_path: dir.path().join("idx.sqlite"),
        client: reqwest::Client::new(),
        cfg: claw_rag_service::EmbedConfig::mock_from_env().expect("mock embed config"),
        jobs: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        ingest_lock: Arc::new(tokio::sync::Mutex::new(())),
        next_job_id: Default::default(),
    });

    let server = build_server(state);
    let (client, srv) = tokio::io::duplex(1 << 20);
    let server_task = tokio::spawn(server.run(srv.clone(), srv));

    let mut c = client;
    let init = request(&mut c, &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "claw-rag");
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");

    let listed = request(&mut c, &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(names, vec!["rag_query", "rag_stats", "rag_ingest", "rag_ingest_status"]);

    let ingest = request(
        &mut c,
        &json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rag_ingest","arguments":{"workspaces":[ws.to_string_lossy().to_string()]}}}),
    )
    .await;
    assert_eq!(ingest["result"]["isError"], false);
    let job_id = ingest["result"]["content"][0]["text"]
        .as_str()
        .expect("job text")
        .trim()
        .strip_prefix("job_id: ")
        .expect("job prefix")
        .to_string();

    let mut done = false;
    for i in 0..100 {
        let status = request(
            &mut c,
            &json!({"jsonrpc":"2.0","id":4+i,"method":"tools/call","params":{"name":"rag_ingest_status","arguments":{"job_id":job_id}}}),
        )
        .await;
        let text = status["result"]["content"][0]["text"].as_str().expect("status text");
        if text.contains("status: done") {
            done = true;
            break;
        }
        if text.contains("status: failed") {
            panic!("ingest failed: {text}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(done, "ingest never finished over the wire");

    let q = request(
        &mut c,
        &json!({"jsonrpc":"2.0","id":200,"method":"tools/call","params":{"name":"rag_query","arguments":{"query":"RAG service"}}}),
    )
    .await;
    let qtext = q["result"]["content"][0]["text"].as_str().expect("query text");
    assert!(qtext.contains("phase: 1-sqlite"), "query: {qtext}");
    assert!(qtext.contains("note.md"), "query: {qtext}");

    let stats = request(
        &mut c,
        &json!({"jsonrpc":"2.0","id":201,"method":"tools/call","params":{"name":"rag_stats","arguments":{}}}),
    )
    .await;
    assert!(stats["result"]["content"][0]["text"]
        .as_str()
        .expect("stats text")
        .contains("chunks: "));

    drop(c);
    let _ = server_task.await;
    std::env::remove_var("CLAW_RAG_MOCK_PROVIDERS");
}
```

- [ ] **Step 2: Run test to verify it fails (if tools weren't working) / passes**

Run: `cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release --test full_flow"`
Expected: PASS.

- [ ] **Step 3: Verify the release binary builds and print help**

Run:
```bash
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo build --release"
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo clippy --release --all-targets -- -D warnings"
```
Expected: build succeeds; clippy clean with `-D warnings`.

- [ ] **Step 4: Commit**

```bash
git add tests/full_flow.rs
git commit -m "test: end-to-end MCP session over duplex stdio"
```

---

### Task 8: README and final verification

**Files:**
- Create: `D:\tempo\claw-rag-mcp\README.md`

**Interfaces:**
- Consumes: everything above.
- Produces: installation/usage documentation.

- [ ] **Step 1: Write README.md**

Create `D:\tempo\claw-rag-mcp\README.md`:

```markdown
# claw-rag-mcp

Standalone MCP (Model Context Protocol) server exposing RAG capabilities over a
SQLite index shared with `claw-rag-service`.

## Install

```bash
cd D:\tempo\claw-rag-mcp
cargo build --release
```

Copy `target\release\claw-rag-mcp.exe` to a directory on your PATH
(e.g. `C:\Users\<you>\bin`).

## Configure an MCP host

Point any MCP client at the binary via `command`. Example for opencode:

```json
{
  "mcpServers": {
    "claw-rag": {
      "command": "claw-rag-mcp",
      "env": {
        "CLAW_RAG_DB": "D:/data/rag/.claw-rag/index.sqlite",
        "OPENAI_API_KEY": "sk-..."
      }
    }
  }
}
```

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `CLAW_RAG_DB` | `.claw-rag/index.sqlite` | Shared SQLite index path |
| `CLAW_RAG_OPENAI_API_KEY` / `OPENAI_API_KEY` | — | Embedding API key (required) |
| `CLAW_RAG_EMBEDDING_BASE_URL` | `https://api.openai.com/v1` | OpenAI-compatible embeddings endpoint |
| `CLAW_RAG_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model |
| `CLAW_RAG_MOCK_PROVIDERS` | — | `1` = deterministic mock embeddings (testing) |

## Tools

- `rag_query {query, top_k?}` — semantic search over the index. `top_k` default 8, max 32.
- `rag_stats {}` — chunk count and index phase (`1-sqlite-no-db` / `1-sqlite-empty` / `1-sqlite`).
- `rag_ingest {workspaces: [...]}` — asynchronously index workspaces; returns a `job_id`.
- `rag_ingest_status {job_id}` — poll ingest progress (`running` / `done` / `failed` / `unknown`).

## Notes

- Ingest jobs live in process memory; restarting the server loses them. Re-run
  `rag_ingest` after a restart.
- The index is shared with the `claw-rag-service` HTTP server when both use the
  same `CLAW_RAG_DB`. Only one process should ingest at a time (ingest jobs are
  serialized within this server).
```

- [ ] **Step 2: Final verification**

Run:
```bash
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo fmt -- --check"
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo clippy --release --all-targets -- -D warnings"
cmd /c "C:\Users\Incredible\.config\opencode\CompilePreSet.bat && cargo test --release"
```
Expected: fmt clean, clippy clean, all tests PASS.

- [ ] **Step 3: Smoke-test the binary over real pipes**

In PowerShell (uses a temp DB, mock embeddings):
```powershell
$env:CLAW_RAG_MOCK_PROVIDERS = "1"
$db = Join-Path $env:TEMP "claw-rag-mcp-smoke.sqlite"
Remove-Item $db -ErrorAction SilentlyContinue
$env:CLAW_RAG_DB = $db
$body = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
$frame = "Content-Length: $($body.Length)`r`n`r`n$body"
$frame | & .\target\release\claw-rag-mcp.exe
Remove-Item Env:\CLAW_RAG_MOCK_PROVIDERS
Remove-Item Env:\CLAW_RAG_DB
```
Expected: prints a `Content-Length`-framed `initialize` response with `"serverInfo":{"name":"claw-rag"}` and exits when stdin closes.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: installation and usage for claw-rag-mcp"
```
