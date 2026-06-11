local validation = require("guardrail.validation")
local diagnostics = require("guardrail.diagnostics")

local M = {}

function M.setup()
  vim.api.nvim_create_user_command("GuardrailValidate", function()
    diagnostics.validate_buffer()
  end, { desc = "Validate current buffer with Guardrail" })

  vim.api.nvim_create_user_command("GuardrailValidateSelection", function()
    diagnostics.validate_selection()
  end, { desc = "Validate visual selection with Guardrail" })

  vim.api.nvim_create_user_command("GuardrailClear", function()
    diagnostics.clear_diagnostics()
  end, { desc = "Clear Guardrail diagnostics" })

  vim.api.nvim_create_user_command("GuardrailTestConnection", function()
    validation.test_connection(function(connected)
      vim.schedule(function()
        if connected then
          vim.notify("Guardrail: Connected", vim.log.levels.INFO)
        else
          vim.notify("Guardrail: Disconnected", vim.log.levels.ERROR)
        end
      end)
    end)
  end, { desc = "Test connection to Guardrail server" })

  vim.api.nvim_create_user_command("GuardrailConfig", function()
    vim.cmd("edit " .. vim.fn.stdpath("config") .. "/lua/guardrail_config.lua")
  end, { desc = "Open Guardrail configuration" })
end

return M
