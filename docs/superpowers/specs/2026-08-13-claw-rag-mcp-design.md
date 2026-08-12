# claw-rag-mcp — 独立 RAG MCP Server 设计

日期：2026-08-13
状态：已批准（设计评审通过）

## 背景与目标

将 `claw-rag-service` 的 RAG 能力（语义检索 / 索引 / 统计）封装为一个**完全独立**的 MCP server 可执行文件，放入系统 PATH，供任意 MCP host（opencode、Claude Desktop、Cursor 等）通过标准 `command` 方式启动，走 stdio 传输协议。

**关键约束**：
- **与 claw-analog / 原 HTTP 服务的 `retrieve_context` 实现无任何耦合**。工具名、实现、行为均为全新设计，不沿用原版的工具名（`retrieve_context`）与调用约定。
- 独立可分发 exe（`cargo build --release` 后单文件），放 PATH 即用。
- 传输层自研极简 stdio（参考 `runtime::mcp_server.rs` 的 framing/dispatch 模式），**零外部协议依赖**，符合仓库 `forbid(unsafe_code)` lint。
- 依赖 `claw-rag-service` 的 lib（`query_index` / `run_ingest` / `chunk_count` / `EmbedConfig`）复用索引、分块、embedding 逻辑；与现有 HTTP 服务**共享同一份 SQLite 索引**（`CLAW_RAG_DB`，默认 `.claw-rag/index.sqlite`）。

## 架构总览

```
新 crate: crates/claw-rag-mcp
  └─ bin: claw-rag-mcp
       │  - 自研极简 stdio MCP server（LSP Content-Length framing + JSON-RPC dispatch）
       │  - 协议子集: initialize / tools/list / tools/call
       │  - 依赖: tokio, serde_json, claw-rag-service (lib)
       └─ serverInfo: name=claw-rag, version=workspace version
```

- 二进制名：`claw-rag-mcp`
- 协议版本：`2025-03-26`
- 能力声明：`{"tools": {}}`
- 配置环境变量（复用 claw-rag-service 现有约定）：
  - `CLAW_RAG_DB`：SQLite 索引路径（默认 `.claw-rag/index.sqlite`）
  - `CLAW_RAG_OPENAI_API_KEY` / `OPENAI_API_KEY`：embedding API key
  - `CLAW_RAG_EMBEDDING_BASE_URL`：默认 `https://api.openai.com/v1`
  - `CLAW_RAG_EMBEDDING_MODEL`：默认 `text-embedding-3-small`
  - `CLAW_RAG_MOCK_PROVIDERS=1`：确定性 mock embedding（测试/试用）

## 暴露的工具（全新命名，前缀 `rag_`）

| 工具 | 入参 | 返回 | 权限 |
|---|---|---|---|
| `rag_query` | `query`(必填), `top_k`(默认8, ≤32) | 格式化 hits（path/snippet/score）+ `phase` | 只读 |
| `rag_stats` | `{}` | `chunks` 数 + `phase` | 只读 |
| `rag_ingest` | `workspaces`: 路径数组 | 立即返回 `job_id`（后台任务） | 写索引 |
| `rag_ingest_status` | `job_id` | `running`(进度) / `done`(统计) / `failed`(错误) / `unknown` | 只读 |

`phase` 取值（沿用索引状态语义，但作为输出字段而非协议）：
- `1-sqlite-no-db`：索引文件不存在
- `1-sqlite-empty`：索引存在但无 chunk
- `1-sqlite`：有数据

## 异步 ingest job 机制

- 进程内存 `JobRegistry`（`Mutex<HashMap<job_id, JobState>>`）。stdio server 为长驻进程，job 跨 `tools/call` 有效。
- **限制**：host 重启进程后 job 丢失（不持久化）。在文档中注明；符合"查询 + 维护性索引"定位。
- **进度上报**：给 `claw-rag-service` 增加 `run_ingest_with_progress(workspaces, db_path, cfg, client, progress: impl FnMut(IngestProgress))`；现有 `run_ingest` 委托它并传 no-op。`IngestProgress { files_done, files_total, chunks_total }`。现有调用方零改动。
- **SQLite 单写者约束**：ingest job 用全局 `Mutex` 串行执行，避免 SQLITE_BUSY。
- job_id：进程内递增整数转字符串（如 `"1"`、`"2"`）。

## 错误处理

- JSON-RPC 规范错误码：
  - `-32700` parse error
  - `-32600` invalid request
  - `-32601` method not found
  - `-32602` invalid params
- 工具执行错误 → `isError: true` + text 消息（与 claw 现有约定一致），例如：
  - `no index (run rag_ingest first)`
  - `embedding dimension mismatch ...`（索引维度与查询维度不一致提示）
  - 工具参数缺失/非法

## 测试

- **单元**：dispatch 层各分支（initialize / tools/list / tools/call 正常与错误 / 未知方法 / 非法参数）。
- **集成**：`CLAW_RAG_MOCK_PROVIDERS=1` + tempdir，全流程：
  1. `rag_ingest` 起 job → 立即返回 job_id
  2. 轮询 `rag_ingest_status` 至 `done`
  3. `rag_query` 命中相关文件
  4. `rag_stats` 反映 chunks 数
- **framing**：进程内 pipe 模拟 stdin/stdout 往返验证 LSP 帧格式与 dispatch。

## 非目标（YAGNI）

- 不做 HTTP/SSE 传输。
- 不持久化 job 状态。
- 不实现 MCP resources / prompts / 认证。
- 不改动 claw-analog 的 `retrieve_context` 实现。
- 不新增对 `runtime` crate 的依赖（避免引入 plugins/telemetry）。

## 文档

- 在 `rust/README` 或 crate 内 `README.md` 记录安装与配置方式：
  - `cargo build --release -p claw-rag-mcp` → 将 `target/release/claw-rag-mcp.exe` 放入 PATH
  - opencode / Claude Desktop 配置 `command: "claw-rag-mcp"`
  - 环境变量说明
