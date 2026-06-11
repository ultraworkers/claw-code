local validation = require("guardrail.validation")
local config = {}

local M = {}
local namespace = vim.api.nvim_create_namespace("guardrail")

function M.setup(cfg)
  config = cfg

  if config.validate_on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = vim.api.nvim_create_augroup("Guardrail", { clear = true }),
      callback = function(args)
        M.validate_buffer(args.buf)
      end,
    })
  end
end

function M.validate_buffer(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  local bufname = vim.api.nvim_buf_get_name(bufnr)

  if bufname == "" then
    return
  end

  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  local content = table.concat(lines, "\n")
  local language = vim.bo[bufnr].filetype

  validation.validate_file(bufname, content, language, function(result)
    vim.schedule(function()
      M.clear_diagnostics(bufnr)

      if result.error then
        vim.notify("Guardrail validation error: " .. result.error, vim.log.levels.ERROR)
        return
      end

      local diagnostics = {}
      for _, violation in ipairs(result.violations or {}) do
        if M.should_report(violation.severity) then
          table.insert(diagnostics, M.to_diagnostic(violation))
        end
      end

      vim.diagnostic.set(namespace, bufnr, diagnostics)

      local count = #diagnostics
      if count > 0 then
        vim.notify(string.format("Guardrail: Found %d violation(s)", count), vim.log.levels.WARN)
      end
    end)
  end)
end

function M.validate_selection()
  local bufnr = vim.api.nvim_get_current_buf()
  local mode = vim.fn.mode()

  if mode ~= "v" and mode ~= "V" then
    vim.notify("No visual selection", vim.log.levels.WARN)
    return
  end

  vim.cmd('normal! "vy')
  local code = vim.fn.getreg("v")
  local language = vim.bo[bufnr].filetype

  validation.validate_selection(code, language, function(result)
    vim.schedule(function()
      if result.error then
        vim.notify("Guardrail validation error: " .. result.error, vim.log.levels.ERROR)
        return
      end

      local violations = result.violations or {}
      if #violations == 0 then
        vim.notify("Selection is valid", vim.log.levels.INFO)
      else
        local messages = {}
        for _, v in ipairs(violations) do
          table.insert(messages, string.format("- %s", v.message))
        end
        vim.notify(
          string.format("Found %d violation(s):\n%s", #violations, table.concat(messages, "\n")),
          vim.log.levels.WARN
        )
      end
    end)
  end)
end

function M.clear_diagnostics(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  vim.diagnostic.reset(namespace, bufnr)
end

function M.to_diagnostic(violation)
  local severity_map = {
    error = vim.diagnostic.severity.ERROR,
    warning = vim.diagnostic.severity.WARN,
    info = vim.diagnostic.severity.INFO,
  }

  return {
    lnum = violation.line - 1,
    col = violation.column - 1,
    message = violation.message,
    severity = severity_map[violation.severity] or vim.diagnostic.severity.WARN,
    source = "Guardrail",
    code = violation.rule_id,
  }
end

function M.should_report(severity)
  local levels = { info = 1, warning = 2, error = 3 }
  return (levels[severity] or 0) >= (levels[config.severity_threshold] or 1)
end

return M
