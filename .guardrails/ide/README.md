# Guardrail IDE Extensions

> Native IDE integrations for the Guardrail MCP Server

## Overview

This directory contains IDE extensions that provide real-time guardrail validation directly within your development environment.

## Status

**Branch:** `main` (merged)
**Phase:** Released
**Release:** v2.0.0

## Supported IDEs

| IDE | Status | Priority | Language |
|-----|--------|----------|----------|
| VS Code | ✅ Released | P0 | TypeScript |
| JetBrains | ✅ Released | P1 | Kotlin |
| Neovim | ✅ Released | P2 | Lua |
| Vim | ✅ Released | P3 | VimScript |

Legend:
- ✅ Released
- 🚧 Complete (Ready for Testing)
- 📋 Planned
- ⏸️ On Hold

## Quick Start

### VS Code

```bash
# Clone and setup
git checkout ide
cd ide/vscode-extension
npm install
npm run compile

# Press F5 in VS Code to test
```

### JetBrains

```bash
# Clone and setup
git checkout ide
cd ide/jetbrains-plugin
./gradlew buildPlugin

# Install from build/distributions/
```

### Neovim

```lua
-- Using lazy.nvim
{
  'TheArchitectit/guardrail.nvim',
  dependencies = { 'nvim-lua/plenary.nvim' },
  config = function()
    require('guardrail').setup({
      server_url = 'http://localhost:8095',
      api_key = 'your-api-key',
    })
  end
}
```

### Vim

```bash
# Clone
git clone https://github.com/TheArchitectit/guardrail.vim.git ~/.vim/pack/plugins/start/guardrail.vim

# Or using vim-plug
" Add to .vimrc:
Plug 'TheArchitectit/guardrail.vim'
```

## Security Notice

> **⚠️ Important:** Never commit API keys to version control. Use environment variables:
> - VS Code: Uses SecretStorage API (secure by default)
> - JetBrains: Uses JPasswordField with encryption at rest
> - Neovim/Vim: Use `let g:guardrail_api_key = $GUARDRAIL_API_KEY`
>
> For production deployments, always use HTTPS to connect to the MCP server.

## Directory Structure

```
ide/
├── IDE_EXTENSIONS_PLAN.md     # Master plan document
├── TEAM_STRUCTURE.md          # Team organization
├── TESTING_GUIDE.md           # Testing documentation
├── README.md                  # This file
├── vscode-extension/          # VS Code extension (P0)
│   ├── package.json
│   ├── tsconfig.json
│   └── src/
│       ├── extension.ts
│       ├── types.ts
│       ├── commands.ts
│       ├── providers/
│       │   ├── diagnostics.ts
│       │   └── statusBar.ts
│       └── utils/
│           └── client.ts
├── jetbrains-plugin/          # IntelliJ/PyCharm plugin (P1)
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   └── src/main/
│       ├── kotlin/com/guardrail/plugin/
│       │   ├── GuardrailService.kt
│       │   ├── GuardrailInspection.kt
│       │   ├── GuardrailConfigurable.kt
│       │   ├── GuardrailStatusBarWidget.kt
│       │   └── actions/
│       │       ├── ValidateFileAction.kt
│       │       ├── ValidateSelectionAction.kt
│       │       └── TestConnectionAction.kt
│       └── resources/META-INF/
│           └── plugin.xml
└── neovim-plugin/             # Neovim Lua plugin (P2)
    └── lua/guardrail/
        ├── init.lua
        ├── validation.lua
        ├── diagnostics.lua
        ├── commands.lua
        └── statusline.lua
└── vim-plugin/                # Vim plugin (P3)
    ├── plugin/
    │   └── guardrail.vim
    ├── autoload/
    │   └── guardrail.vim
    └── doc/
        └── guardrail.txt
```

## Features

All IDE extensions provide:

- ✅ Real-time validation (on save and on type)
- ✅ Inline diagnostics with severity levels
- ✅ Status bar connection indicator
- ✅ Command palette integration
- ✅ Quick fixes for common violations
- ✅ Configuration UI
- ✅ Output channel for logs

## Architecture

```
IDE Extensions
├── VS Code (TypeScript)
│   ├── HTTP Client → MCP Server
│   ├── Diagnostics Provider (inline)
│   ├── Status Bar Widget
│   └── Commands (5 commands)
├── JetBrains (Kotlin)
│   ├── HTTP Client → MCP Server
│   ├── Inspection Tool (annotator)
│   ├── Status Bar Widget
│   └── Actions (3 actions)
├── Neovim (Lua)
│   ├── HTTP Client → MCP Server
│   ├── Diagnostic API
│   ├── Status Line Component
│   └── Commands (5 commands)
├── Vim (VimScript)
│   ├── curl → MCP Server
│   ├── Signs (inline indicators)
│   ├── Location List
│   └── Commands (5 commands)
└── MCP Server (Port 8095)
    ├── /ide/validate/file
    ├── /ide/validate/selection
    └── /ide/health
```

## Development

### Prerequisites

- Node.js 16+ (VS Code)
- JDK 17+ (JetBrains)
- Neovim 0.9+ + plenary.nvim (Neovim)

### Testing

See [TESTING_GUIDE.md](./TESTING_GUIDE.md) for comprehensive testing documentation.

### Commands

| IDE | Validate File | Validate Selection | Test Connection |
|-----|---------------|-------------------|-----------------|
| VS Code | ✅ `Ctrl+Shift+G` | ✅ Command Palette | ✅ Command Palette |
| JetBrains | ✅ `Ctrl+Shift+G` | ✅ Code Menu | ✅ Tools Menu |
| Neovim | ✅ `:GuardrailValidate` | ✅ `:GuardrailValidateSelection` | ✅ `:GuardrailTestConnection` |
| Vim | ✅ `:GuardrailValidate` | ✅ `:GuardrailValidateSelection` | ✅ `:GuardrailTestConnection` |
## Configuration

### Standard Config Format

```jsonc
// ~/.guardrail/config.jsonc
{
  "server_url": "http://localhost:8095",
  "api_key": "your-api-key",
  "project_slug": "my-project",
  "enabled": true,
  "validate_on_save": true,
  "severity_threshold": "warning"
}
```

### VS Code Settings

```json
{
  "guardrail.enabled": true,
  "guardrail.serverUrl": "http://localhost:8095",
  "guardrail.apiKey": "your-api-key",
  "guardrail.projectSlug": "my-project",
  "guardrail.validateOnSave": true,
  "guardrail.severityThreshold": "warning"
}
```

### Neovim Lua

```lua
require('guardrail').setup({
  server_url = 'http://localhost:8095',
  api_key = 'your-api-key',
  project_slug = 'my-project',
  enabled = true,
  validate_on_save = true,
  severity_threshold = 'warning',
})
```

### Vim Configuration

```vim
let g:guardrail_server_url = 'http://localhost:8095'
let g:guardrail_api_key = 'your-api-key'
let g:guardrail_project_slug = 'my-project'
let g:guardrail_enabled = 1
let g:guardrail_validate_on_save = 1
let g:guardrail_severity_threshold = 'warning'

set statusline+=%{guardrail#Statusline()}
```

## Contributing

See [TEAM_STRUCTURE.md](./TEAM_STRUCTURE.md) for team organization and [IDE_EXTENSIONS_PLAN.md](./IDE_EXTENSIONS_PLAN.md) for roadmap.

## Resources

- **Plan:** [IDE_EXTENSIONS_PLAN.md](./IDE_EXTENSIONS_PLAN.md)
- **Team:** [TEAM_STRUCTURE.md](./TEAM_STRUCTURE.md)
- **Testing:** [TESTING_GUIDE.md](./TESTING_GUIDE.md)
- **MCP Server:** `/mcp-server/`

## License

BSD-3-Clause
