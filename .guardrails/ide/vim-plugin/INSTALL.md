# Guardrail Vim Plugin - Installation Guide

Real-time guardrail validation for Vim.

## Requirements

- Vim 8.0+ or Neovim
- `curl` or `wget` for HTTP requests
- Running Guardrail MCP Server

## Installation

### Using [vim-plug](https://github.com/junegunn/vim-plug)

Add to `.vimrc`:

```vim
Plug 'TheArchitectit/agent-guardrails-template', { 'rtp': 'ide/vim-plugin' }
```

Then run:

```vim
:PlugInstall
```

### Using [Vundle](https://github.com/VundleVim/Vundle.vim)

Add to `.vimrc`:

```vim
Plugin 'TheArchitectit/agent-guardrails-template', {'rtp': 'ide/vim-plugin/'}
```

Then run:

```vim
:PluginInstall
```

### Using [pathogen.vim](https://github.com/tpope/vim-pathogen)

```bash
cd ~/.vim/bundle
git clone --depth 1 https://github.com/TheArchitectit/agent-guardrails-template.git
cd agent-guardrails-template/ide/vim-plugin
```

### Manual Installation

```bash
mkdir -p ~/.vim/pack/plugins/start
cd ~/.vim/pack/plugins/start
git clone --depth 1 https://github.com/TheArchitectit/agent-guardrails-template.git guardrail
cp -r guardrail/ide/vim-plugin/* guardrail/
rm -rf guardrail/ide guardrail/mcp-server guardrail/docs
```

## Configuration

Add to `.vimrc`:

```vim
" Required settings
let g:guardrail_server_url = 'http://localhost:8095'
let g:guardrail_project_slug = 'your-project'

" Optional settings
let g:guardrail_api_key = 'your-api-key'
let g:guardrail_validate_on_save = 1
let g:guardrail_severity_threshold = 'warning'  " error, warning, info
```

> **Security Warning:** Never commit API keys to version control.
> Use environment variables: `let g:guardrail_api_key = $GUARDRAIL_API_KEY`

## Commands

| Command | Description |
|---------|-------------|
| `:GuardrailValidate` | Validate current file |
| `:GuardrailValidateSelection` | Validate visual selection |
| `:GuardrailTestConnection` | Test MCP server connection |
| `:GuardrailConfigure` | Open configuration |

## Key Mappings

Default mappings (can be disabled with `g:guardrail_no_mappings = 1`):

- `<leader>gv` - Validate file
- `<leader>gs` - Validate selection (visual mode)
- `<leader>gc` - Test connection

Customize:

```vim
nmap <leader>g <Plug>GuardrailValidate
vmap <leader>g <Plug>GuardrailValidateSelection
```

## Status Line

Add to your status line:

```vim
set statusline+=%{guardrail#statusline#GetStatus()}
```

Or with airline/lightline:

```vim
let g:airline_section_x = airline#section#create(['guardrail'])
```

## Quick Start

1. Install the plugin
2. Set required config in `.vimrc`:
   ```vim
   let g:guardrail_server_url = 'http://localhost:8095'
   let g:guardrail_project_slug = 'my-project'
   ```
3. Reload Vim or run `:source ~/.vimrc`
4. Test connection: `:GuardrailTestConnection`
5. Validate file: `:GuardrailValidate`

## Troubleshooting

### "Command not found"
- Plugin not loaded - check installation path
- Run `:scriptnames` to verify plugin loaded

### "Connection failed"
- Check MCP server is running
- Verify `g:guardrail_server_url`
- Check `:messages` for errors

### No diagnostics showing
- Check `g:guardrail_project_slug` is set
- Verify file is within project scope
- Check `g:guardrail_severity_threshold`

## Compatibility

- Vim 8.0+ with `+job` support (for async)
- Vim 7.4+ (sync only)
- Neovim 0.3+
