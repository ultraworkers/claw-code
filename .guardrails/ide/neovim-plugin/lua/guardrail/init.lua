local M = {}

M.config = {
  server_url = "http://localhost:8095",
  api_key = "",
  project_slug = "",
  enabled = true,
  validate_on_save = true,
  severity_threshold = "warning",
}

function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  require("guardrail.validation").setup(M.config)
  require("guardrail.commands").setup()
  require("guardrail.statusline").setup()
end

return M
