# run-brewcode.ps1 — launch brewcode with an Ollama Cloud model.
#
# Reads OPENAI_API_KEY / OPENAI_BASE_URL from .env and exports them into the
# session environment, because brewcode only honors OPENAI_BASE_URL from the real
# process environment (not from the .env file directly).
#
# Usage:
#   .\run-brewcode.ps1                       # default model (kimi-k2.6)
#   .\run-brewcode.ps1 qwen3-coder:480b      # any Ollama Cloud model id
#   .\run-brewcode.ps1 kimi-k2-thinking
#
# The "openai/" prefix is added automatically — required so kimi-* models
# route to Ollama Cloud instead of DashScope.

param(
    [string]$Model = "kimi-k2.6"
)

$ErrorActionPreference = "Stop"
$envFile = Join-Path $PSScriptRoot ".env"

if (-not (Test-Path $envFile)) {
    Write-Error ".env not found at $envFile"
    exit 1
}

# Parse simple KEY=VALUE lines from .env (ignores blanks and # comments).
foreach ($line in Get-Content $envFile) {
    $trimmed = $line.Trim()
    if ($trimmed -eq "" -or $trimmed.StartsWith("#")) { continue }
    $eq = $trimmed.IndexOf("=")
    if ($eq -lt 1) { continue }
    $key = $trimmed.Substring(0, $eq).Trim()
    $val = $trimmed.Substring($eq + 1).Trim()
    if ($key -eq "OPENAI_API_KEY" -or $key -eq "OPENAI_BASE_URL") {
        Set-Item -Path "Env:$key" -Value $val
    }
}

if (-not $env:OPENAI_API_KEY -or -not $env:OPENAI_BASE_URL) {
    Write-Error "OPENAI_API_KEY / OPENAI_BASE_URL not found (uncommented) in .env"
    exit 1
}

Write-Host "Endpoint : $env:OPENAI_BASE_URL"
Write-Host "Model    : openai/$Model"
Write-Host ""

& "$PSScriptRoot\rust\target\debug\brewcode.exe" --model "openai/$Model" --dangerously-skip-permissions
