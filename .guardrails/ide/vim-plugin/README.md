# Guardrail for Vim

Real-time guardrail validation for Vim, integrating with the Guardrail MCP Server.

## Features

- Validate files on save
- Validate visual selections
- Inline error indicators (signs)
- Location list integration
- Statusline support

## Installation

### Using vim-plug

```vim
Plug 'TheArchitectit/guardrail.vim'
```

### Using Vundle

```vim
Plugin 'TheArchitectit/guardrail.vim'
```

### Using Pathogen

```bash
git clone https://github.com/TheArchitectit/guardrail.vim.git ~/.vim/bundle/guardrail.vim
```

## Configuration

```vim
" Guardrail Configuration
let g:guardrail_server_url = 'http://localhost:8095'
let g:guardrail_api_key = ''
let g:guardrail_project_slug = ''
let g:guardrail_enabled = 1
let g:guardrail_validate_on_save = 1
let g:guardrail_severity_threshold = 'warning'
```

## Commands

| Command | Description |
|---------|-------------|
| `:GuardrailValidate` | Validate current buffer |
| `:GuardrailValidateSelection` | Validate visual selection |
| `:GuardrailClear` | Clear diagnostics |
| `:GuardrailTestConnection` | Test server connection |
| `:GuardrailConfig` | Open configuration |

## Statusline

```vim
set statusline+=%{guardrail#Statusline()}
```

## Requirements

- Vim 8.0+ or Neovim 0.4+
- curl command-line tool
- Guardrail MCP Server

## License

BSD-3-Clause
