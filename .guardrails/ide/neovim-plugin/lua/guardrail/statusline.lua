local validation = require("guardrail.validation")

local M = {}
local config = {}
local is_connected = false

function M.setup(cfg)
  config = cfg

  vim.defer_fn(function()
    validation.test_connection(function(connected)
      is_connected = connected
    end)
  end, 1000)
end

function M.component()
  if not config.enabled then
    return "Guardrail: Off"
  end

  if is_connected then
    return "Guardrail: Connected"
  else
    return "Guardrail: Disconnected"
  end
end

function M.set_connected(connected)
  is_connected = connected
end

return M
