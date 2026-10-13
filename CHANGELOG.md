# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-10-13

### Added

- SDK session lifecycle management with session reconciliation and recovery

### Fixed

- Session lifecycle outcome serialization and preservation in recovery workflows

## [0.1.5] - 2026-10-13

### Added

- CLI turn interruption: press Esc to interrupt running turns and cancel in-flight model requests
- Custom OpenAI provider namespace with separate environment variables for multi-tenant scenarios
- `/tools` and `/info` slash commands for command discovery and runtime information
- Team enhancements: subagentModel accessor chain and parallel tool execution support
- Configuration options for concurrent tool calls and parallel subagent execution (`maxParallelToolCalls`, `maxConcurrentAgents`)
- Per-team subagent concurrency control via AgentGate
- SDK operation registry with schema validation for extensible operation handling
- Bundled skills registry and skill chaining support for skill composition
- Per-session settings override with schema validation
- WebSocket client transport completion for SDK wire protocol support

### Fixed

- Turn interruption signal handling for graceful cancellation of in-flight requests
- Markdown streaming block spacing in CLI output
- Spinner restoration for proper is_terminal() check in run_turn
- Model token limit catch-all to return None for unknown models instead of panicking
- Parallel tool result ordering by call index with fan-out capping
- Workspace unsafe_code=forbid restoration across all crates
- TaskClaim TOCTOU (time-of-check-time-of-use) vulnerability
- TUI dependency removal and claw-rag-service compilation on non-Linux platforms
- Clippy lint violations in trident, compact, and claw-rag-service crates
- PermissionMode comparison to use explicit .covers() method instead of direct operators
- OPENAI_BASE_URL environment isolation in provider hint tests

### Changed

- Enhanced permission mode checking for consistency and clarity
- Improved team and concurrent agent configuration schema
- Refactored slash command processing for /tools and /info implementations

### Known Limitations

- ACP / Zed integration is not yet implemented (status stub available via `claw acp`)
- Python porting workspace is for reference only, not used in production

## [0.1.4] - 2026-10-13

### Added

- GJC-compatible line range selectors for read_file tool (`:50-200`, `:50+150` syntax)
- GJC-compatible `:conflicts` selector for git merge conflict detection
- SDK wire transport with WebSocket support
- Nix flake development environment
- Persistent provider configuration loading
- Pagination support for large read_file results
- Comprehensive GJC compatibility documentation and analysis

### Fixed

- Windows path containment checks for security boundary enforcement
- Git bash detection on Windows with proper probe caching
- Unresolvable path denial to prevent escape attempts
- Auto-compaction context window measurement
- Tool result preservation on failed turns
- Model stall recovery without tool calls
- Streaming markdown formatting with preserved block separators
- Local model validation bypass generalization beyond Ollama
- Release artifact checksums on macOS and Linux
- Duplicate assistant text rendering
- Workspace slash command wire-up

### Changed

- Enhanced path scope validation for multiple Windows path spellings
- Improved error handling and configuration defaults
- Upgraded GitHub Actions to immutable commit SHAs for CI security

### Known Limitations

- ACP / Zed integration is not yet implemented (status stub available via `claw acp`)
- Python porting workspace is for reference only, not used in production

## [0.1.3] - 2026-10-13

This is the first public binary release of Claw Code.

### Added

- Initial binary release for Linux x64, macOS arm64, and Windows x64
- Full CLI implementation with session persistence and MCP support
- Multi-provider support: Anthropic Claude, OpenAI, X.AI Grok, and OpenAI-compatible providers (Ollama, llama.cpp, vLLM)
- 55 built-in tools: Bash, file I/O, editing, search, web access, agent delegation, and more
- 120+ slash commands for configuration, help, and system operations
- Permission system with support for workspace-write, read-only, and prompt-mode enforcement
- Plugin system with manifest-driven plugin installation and management
- Comprehensive session management with configuration loading and persistence
- Dogfood build with build-time git provenance tracking

### Fixed

- Fixed duplicate assistant text in message rendering
- Fixed Windows workspace path compatibility (support for multiple Windows path spellings)
- Fixed session_id path traversal vulnerability in session store
- Fixed untrusted project hooks in read-only mode
- Fixed permission privilege ladder to use explicit rank ordering
- Fixed Windows extended-path boundary handling
- Fixed output ownership reporting on auto-compact retry paths

### Changed

- Reorganized permission system for improved clarity and consistency
- Enhanced configuration system to support persistent provider configuration
- Improved error handling and reporting throughout the runtime

### Known Limitations

- ACP / Zed integration is not yet implemented (status stub available via `claw acp`)
- Python porting workspace is for reference only, not used in production

## Pre-release

This project has been in development as an agent-managed exhibit. Previous work is tracked in git history.
