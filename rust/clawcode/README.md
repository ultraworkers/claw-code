# Claw Code

A terminal-native AI coding assistant built in Rust. Connects to Anthropic's Messages API and OpenAI-compatible providers (LM Studio, Ollama, vLLM, OpenRouter). Features a full REPL, MCP integration, WASM-based plugin system, agent delegation, and a permission-gated tool ecosystem.

![Terminal](terminal.png)

## Project Origin

This project was developed from a reset of the Claudecode project by UltraWorkers AI. Extensive work was done to make the project functional, with large-scale, wide-ranging modifications — only a small portion of the original code remains. This project holds significant value.

### Crate-Level Changes vs Original

**Removed crates (3):**

| Crate | Description |
|---|---|
| `claw-analog/` | Original main binary — replaced by `claw-cli` |
| `claw-rag-service/` | RAG retrieval service (Qdrant + embeddings) — fully removed |
| `rusty-claude-cli/` | Old CLI layer — merged into `claw-cli` |

**Added crates (4):**

| Crate | Description |
|---|---|
| `agents/` | Agent delegation engine (spawn, discovery, persist, runtime) |
| `claw-cli/` | New main CLI binary (icons, build.rs, config_wizard, picker, render) |
| `migrate-patch-names/` | One-shot patch-name migration utility |
| `plugin-types/` | Plugin shared types (config, lifecycle, MCP) |

**Shared crate changes:**

| Crate | Changes |
|---|---|
| `api/` | Added `convert.rs`, `incremental_body.rs`; `providers/` fully rewritten (anthropic, openai_compat); `error.rs` restructured |
| `commands/` | `lib.rs` slimmed; extracted `handler.rs`, `registry.rs`, `path_extract.rs`, `plugin_agents.rs` |
| `plugins/` | Removed bundled example hooks; added `frontmatter.rs`, `claude_settings.rs`; `lib.rs` expanded |
| `runtime/` | **Most heavily changed** — removed 8 files (approval_tokens, g004_conformance, mcp_tool_bridge, report_schema, trident, worker_boot, etc.); added 18 new files (thinking/ module, tool_registry/ module, boundary, context, image_*, text_only_models, bash_job_object_ffi, etc.); `config.rs` significantly trimmed |
| `tools/` | `lib.rs` massively refactored; added `excel_extract.rs`, `word_extract.rs`, `subagent_overlay.rs`; removed legacy docs and tests |

**Summary:** 13 original crates → 14 crates. Net deletion of ~15,000+ lines from removed crates, ~3,000+ lines in new crates. `runtime/` and `tools/` underwent architectural-level restructuring.

## Features

- **Dual Provider** — Anthropic Claude + any OpenAI-compatible endpoint (local or cloud)
- **REPL & One-Shot** — Interactive session or single `claw "prompt"` invocation
- **MCP** — Full Model Context Protocol over stdio, SSE, remote, and OAuth
- **Plugins** — WASM-based extensions with versioned marketplace
- **Agents** — `@agent` delegation for sub-task parallelism
- **Skills** — Composable workflows via `/skill` slash commands
- **Tools** — Bash, file R/W/E, grep, glob, PDF/Excel/Word extraction, web
- **Permissions** — ReadOnly / WorkspaceWrite / DangerFullAccess tiers
- **Session Persistence** — Save / resume / export to JSONL

## Quick Start

### Prerequisites

- Rust 2021 edition
- MSVC + Clang-CL 22.x (see `CompilePreSet.bat`)
- NASM, Perl (optional, for OpenSSL)

### Tool Dependencies

- **Git Bash** must be installed at `C:\Program Files\Git`. Download from [git-scm.com](https://git-scm.com) (use "Portable" or "Full installer" — either works).
- **ripgrep** (`rg.exe`) — place in `C:\Program Files\Git\bin`. Repository: [github.com/BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep). Download from [releases](https://github.com/BurntSushi/ripgrep/releases) (Windows zip, extract `rg.exe`).
- **fd** (`fd.exe`) — place in `C:\Program Files\Git\bin`. Repository: [github.com/sharkdp/fd](https://github.com/sharkdp/fd). Download from [releases](https://github.com/sharkdp/fd/releases) (Windows zip, extract `fd.exe`).

> Place `claw.exe` in a directory that is on your system `PATH`. If unsure where to put it, drop it in the Git Bash `bin\` directory alongside `rg.exe` and `fd.exe`.

### Build

```bat
CompilePreSet.bat && cargo build --release
```

### Run

```bat
start.bat
```

Or with a local LLM via LM Studio:

```bat
run_local_openai.bat
```

### Configure

Reference config lives in `claw/` — place the files placed in it to the project root to .claw/ for per-project settings, or at `~/.claw/` for a global user-level config. Copy `.env.example` to `.claw/.env` and set your API key or local endpoint.
### Text-Only Model Configuration

If your LLM does not support image (multimodal) input — common for local/self-hosted models — add its exact name to `LLM_ONLY_MODEL.config`:

- **User-level** (all projects): `~/.claw/LLM_ONLY_MODEL.config`
- **Project-level** (per repo): `.claw/LLM_ONLY_MODEL.config` (walks ancestor dirs)

The model name must match what is sent in the API `model` field. Examples:

```conf
# Exact match
deepseek-v4-flash

# Substring match — matches any ID containing "llama-3"
llama-3

# Prefix match — matches any ID starting with "gpt-"
gpt-:
```

When a model is listed, `Image` and `ImageRef` blocks are replaced with `[Image attached: ...] (not supported by this model)` text placeholders, preventing API errors.

### WebSearch Configuration

Put `web_search_url.json` in `~/.claw/` (global) or `.claw/` (project) to add extra search providers:

```json
{
  "url_1": {
    "enable": true,
    "url": "https://www.bing.com/search?q={search} site:github.com"
  }
}
```

**Built-in default** (no file needed): `url_0` = general Bing search (`q={search}`), always active.
Slots `url_1`–`url_4` are empty and disabled by default.

The config file can add or override `url_1` through `url_4` for site-specific searches.
Built-in `url_0` is always present and provides unrestricted search results alongside
your custom providers. Toggle any entry on/off with `"enable": true` / `"enable": false`.

**`{search}` placeholder:** The keyword and everything after `{search}` in the URL template
is percent-encoded together as a single query value. Use a literal space (not `%20`) between
`{search}` and any suffix — the space is encoded automatically.

Example with query `ardour` and the template above:

```
Template: https://www.bing.com/search?q={search} site:github.com
                                                                  ↓
Suffix extracted:  site:github.com
Keyword + suffix combined:  ardour site:github.com
                                                                  ↓
Percent-encoded query:  ardour%20site%3Agithub.com
                                                                  ↓
Final request:  GET https://www.bing.com/search?q=ardour%20site%3Agithub.com
```

Multiple enabled providers run in parallel; all results are aggregated.

### Claude Code Plugin Compatibility

Claw Code auto-loads plugins from `~/.claude/plugins/` — any Claude Code plugin installed there is available without additional setup.

## Project Structure

```
Claw Code/
├── claw/                         # Config (project-local; or use ~/.claw/ for global)
│   ├── .env
│   ├── .env.example
│   ├── CLAUDE.md
│   ├── LLM_ONLY_MODEL.config
│   ├── settings.json
│   ├── web_search_url.json
│   ├── agents/                   # Sub-agent definitions
│   └── skills/                   # Skill workflow definitions
├── rust/                         # Rust workspace (binary: claw)
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── agents/               # Agent delegation engine
│   │   ├── api/                  # Provider-agnostic API client
│   │   ├── claw-cli/             # Main CLI binary entrypoint
│   │   ├── commands/             # Slash commands, skills, MCP dispatch
│   │   ├── compat-harness/       # Claude Code project manifest compat
│   │   ├── migrate-patch-names/  # One-shot patch-name migration tool
│   │   ├── mock-anthropic-service/ # Test mock
│   │   ├── plugin-types/         # Plugin shared types
│   │   ├── plugins/              # WASM plugin loader & marketplace
│   │   ├── runtime/              # Core engine: config, MCP, permissions
│   │   ├── telemetry/            # Analytics infrastructure
│   │   └── tools/                # Tool implementations
│   └── target/
├── CompilePreSet.bat             # MSVC + Clang-CL environment
├── build_rust_clang_msvc.bat     # Build script
├── build_rust_clang_msvc_test.bat
├── start.bat                     # Launch with VS2022 env
├── startenv.bat                  # Launch with full env setup
├── run_local_openai.bat          # Launch against LM Studio
├── dump_server.py                # Request dump server (debugging)
├── CLAUDE.md
├── terminal.png
└── LICENSE                       # MIT
```

## License

MIT
