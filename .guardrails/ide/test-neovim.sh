#!/bin/bash
# Test gate for Neovim plugin
# Validates Lua syntax and plugin structure

set -e

cd "$(dirname "$0")/neovim-plugin"

echo "=== Neovim Plugin Test Gate ==="
echo

# Check required files
echo "[1/4] Checking required files..."
required_files=(
    "lua/guardrail/init.lua"
    "lua/guardrail/validation.lua"
    "lua/guardrail/diagnostics.lua"
    "lua/guardrail/commands.lua"
    "lua/guardrail/statusline.lua"
)
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Required file missing: $file"
        exit 1
    fi
done
echo "✓ All required files present"

# Check README
echo "[2/4] Checking documentation..."
if [ ! -f "README.md" ]; then
    echo "WARNING: README.md not found"
fi
echo "✓ Documentation check complete"

# Lua syntax validation (if luac available)
echo "[3/4] Checking Lua syntax..."
if command -v luac &> /dev/null; then
    for file in lua/guardrail/*.lua; do
        if ! luac -p "$file" 2>&1; then
            echo "ERROR: Lua syntax error in $file"
            exit 1
        fi
    done
    echo "✓ Lua syntax valid"
else
    echo "⚠ luac not available, skipping syntax check"
fi

# Check for common issues
echo "[4/4] Checking for common issues..."

# Check for required patterns in init.lua
if ! grep -q "function M.setup" "lua/guardrail/init.lua"; then
    echo "ERROR: init.lua missing M.setup function"
    exit 1
fi

# Check for plenary dependency mentioned
if ! grep -q "plenary" "README.md"; then
    echo "WARNING: README should mention plenary.nvim dependency"
fi

# Check for insecure patterns (should use proper escaping)
if grep -r "os.execute" "lua/guardrail/" 2>/dev/null; then
    echo "WARNING: Found os.execute - ensure proper input validation"
fi

# Check for vim.api usage
if ! grep -q "vim.api" "lua/guardrail/init.lua"; then
    echo "WARNING: init.lua should use vim.api for Neovim compatibility"
fi

echo "✓ Common issues check complete"

echo
echo "=== All Neovim Plugin Tests Passed ==="
