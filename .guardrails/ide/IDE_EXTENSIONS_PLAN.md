# IDE Extensions Plan

> Multi-platform IDE extensions for Guardrail MCP Server integration

**Branch:** `ide`  
**Status:** Planning Phase  
**Target Release:** v1.13.0

---

## Overview

This initiative will create native IDE extensions for popular development environments, enabling real-time guardrail validation directly within the editor.

### Goals

1. **Real-time Validation** - Validate code as you type and on save
2. **Inline Diagnostics** - Show violations with severity levels in the editor
3. **Quick Fixes** - Auto-fix common violations (where safe)
4. **Status Integration** - Connection status to MCP server in status bar
5. **Seamless Workflow** - No context switching required

### Supported IDEs

| IDE | Language | Priority | Team Lead |
|-----|----------|----------|-----------|
| VS Code | TypeScript | **P0** | TBD |
| JetBrains (IntelliJ/PyCharm) | Kotlin | **P1** | TBD |
| Neovim | Lua | **P2** | TBD |
| Vim | VimScript | **P3** | TBD |
| Emacs | Emacs Lisp | **P3** | TBD |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        IDE Extensions                        │
├──────────────┬──────────────┬──────────────┬────────────────┤
│   VS Code    │  JetBrains   │   Neovim     │     Vim        │
│  (TypeScript)│   (Kotlin)   │    (Lua)     │  (VimScript)   │
└──────┬───────┴──────┬───────┴──────┬───────┴────────┬───────┘
       │              │              │                │
       └──────────────┴──────────────┴────────────────┘
                          │
       ┌──────────────────┼──────────────────┐
       │                  │                  │
┌──────▼──────┐  ┌───────▼──────┐  ┌───────▼──────┐
│  HTTP API   │  │  WebSocket   │  │   Shared     │
│   Client    │  │   (Future)   │  │   Utils      │
└─────────────┘  └──────────────┘  └──────────────┘
       │                  │
       └──────────────────┘
              │
       ┌──────▼──────────────────────────────┐
       │     Guardrail MCP Server            │
       │     (Port 8095 - IDE API)           │
       └─────────────────────────────────────┘
```

### Shared Components

All extensions share:
- **API Client** - HTTP client for MCP server communication
- **Protocol** - JSON schema for validation requests/responses
- **Configuration** - Standard config format (JSON/JSONC)
- **Icons/Assets** - Status indicators, severity icons

---

## Team Structure

### Extension Leads

| Role | Responsibility | Skills Needed |
|------|---------------|---------------|
| **VS Code Lead** | VS Code extension development | TypeScript, VS Code API |
| **JetBrains Lead** | IntelliJ/PyCharm plugin | Kotlin, IntelliJ Platform SDK |
| **Neovim Lead** | Neovim Lua plugin | Lua, Neovim API |
| **Platform Lead** | Shared components & coordination | Architecture, testing |

### Shared Resources Team

| Role | Responsibility | Deliverables |
|------|---------------|--------------|
| **Protocol Designer** | API contract & schemas | OpenAPI spec, TypeScript types |
| **Icon Designer** | Status bar icons, severity markers | SVG icons, theme variants |
| **Documentation Lead** | User guides, setup docs | READMEs, tutorials |
| **QA Lead** | Testing strategy, CI/CD | Test suites, automation |

### Advisory Roles

- **MCP Server SME** - Backend API knowledge
- **UX Consultant** - IDE UX best practices
- **Security Reviewer** - API key handling, CORS

---

## Development Phases

### Phase 1: Foundation (Week 1-2)

**Deliverables:**
- [ ] Shared API client library (TypeScript)
- [ ] OpenAPI specification for IDE endpoints
- [ ] Protocol buffer definitions (for future gRPC)
- [ ] Configuration schema standardization
- [ ] Icon set design (light/dark themes)

**Team:** Protocol Designer, Platform Lead

### Phase 2: VS Code Extension (Week 2-4)

**MVP Features:**
- [ ] Configuration UI (server URL, API key)
- [ ] Status bar connection indicator
- [ ] Validate on save
- [ ] Inline diagnostics (squiggles)
- [ ] Commands palette integration

**Extended Features:**
- [ ] Real-time validation (on type)
- [ ] Code actions (quick fixes)
- [ ] Output channel for logs
- [ ] Settings sync support

**Team:** VS Code Lead + Platform Lead

### Phase 3: JetBrains Plugin (Week 4-6)

**MVP Features:**
- [ ] Configuration UI (IDE settings)
- [ ] Status bar widget
- [ ] Validate on save
- [ ] Annotator integration
- [ ] Tool window for violations

**Extended Features:**
- [ ] Inspection profile integration
- [ ] Quick fixes with intentions
- [ ] Notifications & balloons

**Team:** JetBrains Lead + Platform Lead

### Phase 4: Neovim Plugin (Week 6-7)

**MVP Features:**
- [ ] Configuration via Lua
- [ ] Status line integration
- [ ] Validate on save (autocmd)
- [ ] Virtual text for diagnostics
- [ ] Telescope picker for violations

**Team:** Neovim Lead

### Phase 5: Polish & Release (Week 7-8)

- [ ] Documentation complete
- [ ] Test suites passing
- [ ] Marketplace submissions
- [ ] Release notes
- [ ] Demo videos

---

## Technical Specifications

### VS Code Extension

**Package:** `guardrail-vscode`
**Publisher:** TheArchitectit
**Engine:** `^1.60.0`

**Extension Points:**
- `configuration` - Settings schema
- `commands` - Command palette entries
- `languages` - Code actions provider
- `views` - Sidebar panel (future)

**Key APIs:**
- `vscode.languages.createDiagnosticCollection()`
- `vscode.workspace.onDidSaveTextDocument()`
- `vscode.commands.registerCommand()`
- `vscode.window.createStatusBarItem()`

### JetBrains Plugin

**Plugin ID:** `com.guardrail.plugin`
**Since Build:** `223.*`

**Extension Points:**
- `annotator` - Real-time inspection
- `inspectionTool` - Code inspection
- `projectConfigurable` - Settings UI
- `statusBarWidgetFactory` - Status bar

**Key APIs:**
- `ExternalAnnotator`
- `LocalInspectionTool`
- `ProjectConfigurable`

### Neovim Plugin

**Plugin Name:** `guardrail.nvim`
**Dependencies:** `plenary.nvim`

**Features:**
- Lua configuration
- Async validation
- LSP-style diagnostics
- Telescope integration

**Key APIs:**
- `vim.diagnostic`
- `vim.api.nvim_create_autocmd()`
- `vim.lsp.util`

---

## Configuration

### Standard Config Format

All IDEs use the same configuration structure:

```jsonc
// ~/.guardrail/config.jsonc
{
  "version": "1.0",
  "servers": [
    {
      "name": "Production",
      "url": "http://100.96.49.42:8095",
      "apiKey": "${GUARDRAIL_API_KEY}",  // Env var or literal
      "default": true
    },
    {
      "name": "Local",
      "url": "http://localhost:8095",
      "apiKey": "dev-key",
      "default": false
    }
  ],
  "settings": {
    "validateOnSave": true,
    "validateOnType": false,
    "showStatusBar": true,
    "severityThreshold": "warning"
  }
}
```

### Per-Project Config

```jsonc
// .guardrailrc.jsonc (in project root)
{
  "extends": "~/.guardrail/config.jsonc",
  "projectSlug": "my-project",
  "settings": {
    "validateOnType": true  // Override global
  }
}
```

---

## API Integration

### HTTP Client Requirements

**Request Format:**
```typescript
interface ValidationRequest {
  file_path: string;
  content: string;
  language: string;
  project_slug?: string;
  session_token?: string;
}
```

**Response Format:**
```typescript
interface ValidationResponse {
  valid: boolean;
  violations: Violation[];
}

interface Violation {
  rule_id: string;
  line: number;
  column: number;
  severity: 'error' | 'warning' | 'info';
  message: string;
  suggestion?: string;
  fix?: TextEdit;
}
```

**Error Handling:**
- Network errors → Status bar indicator
- Auth errors → Prompt for API key
- Timeout → Retry with backoff

---

## User Experience

### VS Code UX Flow

1. **Installation** - From marketplace
2. **Configuration** - Prompt for server URL + API key
3. **Connection** - Status bar shows green indicator
4. **Validation** - Save file → diagnostics appear
5. **Quick Fix** - Hover → "Apply Guardrail Fix"

### JetBrains UX Flow

1. **Installation** - From JetBrains marketplace
2. **Configuration** - Settings → Guardrail
3. **Connection** - Status bar widget
4. **Validation** - File saves → Problems tool window
5. **Quick Fix** - Alt+Enter → Intentions menu

### Neovim UX Flow

1. **Installation** - Plugin manager (lazy.nvim, packer)
2. **Configuration** - Lua setup() call
3. **Connection** - Statusline component
4. **Validation** - :GuardrailValidate or auto-save
5. **Navigation** - :Telescope guardrail

---

## Testing Strategy

### Unit Tests

- Mock MCP server responses
- Test configuration parsing
- Test diagnostic mapping

### Integration Tests

- Real MCP server (test container)
- File operations
- Network failure handling

### E2E Tests

- VS Code: Playwright tests
- JetBrains: UI tests with fixtures
- Neovim: Headless Neovim with plenary

---

## Release Plan

### Pre-Release

- [ ] Beta testers recruited (5-10 per IDE)
- [ ] Private beta releases
- [ ] Feedback collection

### Marketplace Submissions

| Platform | Store | Timeline |
|----------|-------|----------|
| VS Code | Visual Studio Marketplace | Week 8 |
| JetBrains | JetBrains Marketplace | Week 9 |
| Neovim | Not required (GitHub) | Week 8 |

### Versioning

- Extensions versioned independently: `1.0.0`, `1.0.1`, etc.
- Major version matches MCP server compatibility
- Changelog per extension

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| VS Code Installs | 1000+ | Marketplace stats |
| Active Users | 500+ | Validation requests/day |
| Violations Caught | 1000+/month | MCP server metrics |
| User Rating | 4.5+ stars | Marketplace reviews |
| Response Time | <500ms | P95 validation latency |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| IDE API changes | High | Follow semantic versioning, deprecation notices |
| MCP server downtime | Medium | Graceful degradation, offline mode |
| Performance issues | High | Async validation, debouncing, caching |
| Security (API keys) | High | OS keychain integration, never commit keys |
| Cross-platform bugs | Medium | CI/CD on Windows, macOS, Linux |

---

## Next Steps

1. **Recruit Team Leads** - Post in community channels
2. **Create RFCs** - One per IDE for community input
3. **Setup Repositories** - Separate repos or monorepo?
4. **Design Review** - Architecture review with stakeholders
5. **Kickoff Meeting** - Team introduction, sprint planning

---

## Resources

- **VS Code API:** https://code.visualstudio.com/api
- **JetBrains SDK:** https://plugins.jetbrains.com/docs/intellij/
- **Neovim Lua:** https://neovim.io/doc/user/lua.html
- **MCP Server API:** `/mcp-server/API.md`

---

**Questions?** Contact the Platform Lead or open an issue in the ide branch.
