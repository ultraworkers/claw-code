local curl = require("plenary.curl")

local M = {}
local config = {}

function M.setup(cfg)
  config = cfg
end

function M.test_connection(callback)
  curl.get(config.server_url .. "/health/ready", {
    headers = {
      ["Authorization"] = config.api_key ~= "" and "Bearer " .. config.api_key or nil,
    },
    callback = function(response)
      if not response then
        callback(false)
      elseif response.status == 200 then
        callback(true)
      else
        callback(false)
      end
    end,
  })
end

function M.validate_file(file_path, content, language, callback)
  if not config.enabled then
    callback({ valid = true, violations = {} })
    return
  end

  local body = vim.fn.json_encode({
    file_path = file_path,
    content = content,
    language = language,
    project_slug = config.project_slug ~= "" and config.project_slug or nil,
  })

  curl.post(config.server_url .. "/ide/validate/file", {
    body = body,
    headers = {
      ["Content-Type"] = "application/json",
      ["Authorization"] = config.api_key ~= "" and "Bearer " .. config.api_key or nil,
    },
    callback = function(response)
      if not response then
        callback({ valid = false, violations = {}, error = "Network error: unable to reach server" })
        return
      end
      if response.status ~= 200 then
        callback({ valid = false, violations = {}, error = response.body })
        return
      end

      local ok, result = pcall(vim.fn.json_decode, response.body)
      if not ok then
        callback({ valid = false, violations = {}, error = "Failed to parse response" })
        return
      end

      callback(result)
    end,
  })
end

function M.validate_selection(code, language, callback)
  if not config.enabled then
    callback({ valid = true, violations = {} })
    return
  end

  local body = vim.fn.json_encode({
    code = code,
    language = language,
  })

  curl.post(config.server_url .. "/ide/validate/selection", {
    body = body,
    headers = {
      ["Content-Type"] = "application/json",
      ["Authorization"] = config.api_key ~= "" and "Bearer " .. config.api_key or nil,
    },
    callback = function(response)
      if not response then
        callback({ valid = false, violations = {}, error = "Network error: unable to reach server" })
        return
      end
      if response.status ~= 200 then
        callback({ valid = false, violations = {}, error = response.body })
        return
      end

      local ok, result = pcall(vim.fn.json_decode, response.body)
      if not ok then
        callback({ valid = false, violations = {}, error = "Failed to parse response" })
        return
      end

      callback(result)
    end,
  })
end

return M
