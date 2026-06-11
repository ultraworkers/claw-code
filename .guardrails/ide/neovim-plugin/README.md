# Guardrail Neovim Plugin

Real-time guardrail validation for Neovim with native LSP-like diagnostics.

## Features

- Native diagnostics via `vim.diagnostic`
- Status line integration
- Async validation
- Commands for manual validation

## Requirements

- Neovim 0.8+ (for `vim.diagnostic` API)
- [plenary.nvim](https://github.com/nvim-lua/plenary.nvim) (for HTTP requests)
- Running Guardrail MCP Server

## Installation

### Using [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "TheArchitectit/agent-guardrails-template",
  name = "guardrail.nvim",
  dependencies = { "nvim-lua/plenary.nvim" },
  config = function()
    require("guardrail").setup({
      server_url = "http://localhost:8095",
      api_key = "your-api-key",
      project_slug = "your-project",
      validate_on_save = true,
    })
  end,
}
```

### Using [packer.nvim](https://github.com/wbthomason/packer.nvim)

```lua
use {
  "TheArchitectit/agent-guardrails-template",
  name = "guardrail.nvim",
  requires = { "nvim-lua/plenary.nvim" },
  config = function()
    require("guardrail").setup({
      server_url = "http://localhost:8095",
      api_key = "your-api-key",
      project_slug = "your-project",
    })
  end,
}
```

### Using [vim-plug](https://github.com/junegunn/vim-plug)

```vim
Plug 'nvim-lua/plenary.nvim'
Plug 'TheArchitectit/agent-guardrails-template', { 'rtp': 'ide/neovim-plugin', 'name': 'guardrail.nvim' }

lua << EOF
require("guardrail").setup({
  server_url = "http://localhost:8095",
  api_key = "your-api-key",
  project_slug = "your-project",
})
EOF
```

## Configuration

Default configuration:

```lua
require("guardrail").setup({
  -- Connection
  server_url = "http://localhost:8095",
  api_key = nil,
  project_slug = nil,

  -- Validation
  validate_on_save = true,
  validate_on_type = false,
  severity_threshold = "warning", -- "error", "warning", "info"

  -- Diagnostics display
  signs = true,
  virtual_text = true,
  underline = true,
  update_in_insert = false,
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:GuardrailValidate` | Validate current buffer |
| `:GuardrailValidateSelection` | Validate visual selection |
| `:GuardrailTestConnection` | Test MCP connection |
| `:GuardrailStatus` | Show connection status |

## Status Line

Add to your status line:

```lua
-- For lualine
local guardrail = require("guardrail.statusline")

-- Returns: "🛡️ OK", "🛡️ ERR", or "🛡️ --"
require('lualine').setup {
  sections = {
    lualine_x = { guardrail.status },
  }
}
```

Or use manually:

```lua
local statusline = require("guardrail.statusline")
local status = statusline.status() -- Returns icon + text
```

## Keymaps Example

```lua
vim.keymap.set("n", "<leader>gv", ":GuardrailValidate<CR>", { desc = "Guardrail validate file" })
vim.keymap.set("v", "<leader>gv", ":GuardrailValidateSelection<CR>", { desc = "Guardrail validate selection" })
vim.keymap.set("n", "<leader>gc", ":GuardrailTestConnection<CR>", { desc = "Guardrail test connection" })
```

## Troubleshooting

### Diagnostics not showing
- Check `:checkhealth guardrail` (if implemented)
- Verify `vim.diagnostic` is available (Neovim 0.8+)
- Check `:messages` for errors

### Connection failed
- Verify MCP server is running
- Check `server_url` configuration
- Test with `:GuardrailTestConnection`

## Development

For local development:

```bash
git clone https://github.com/TheArchitectit/agent-guardrails-template.git
```

In your init.lua:

```lua
vim.opt.rtp:prepend("/path/to/agent-guardrails-template/ide/neovim-plugin")
require("guardrail").setup({ ... })
```

## Security Notes

> **⚠️ Never commit API keys to version control.** Use environment variables:
> ```lua
> api_key = vim.env.GUARDRAIL_API_KEY,
> ```
>
> **HTTPS Recommended:** For production MCP servers, always use HTTPS.
