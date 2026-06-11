#!/bin/bash
# Test gate for Vim plugin
# Validates VimScript syntax and plugin structure

set -e

cd "$(dirname "$0")/vim-plugin"

echo "=== Vim Plugin Test Gate ==="
echo

# Check required files
echo "[1/5] Checking required files..."
required_files=(
    "plugin/guardrail.vim"
    "autoload/guardrail.vim"
)
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Required file missing: $file"
        exit 1
    fi
done
echo "✓ All required files present"

# Check README
echo "[2/5] Checking documentation..."
if [ ! -f "README.md" ] && [ ! -f "INSTALL.md" ]; then
    echo "WARNING: No README.md or INSTALL.md found"
fi
echo "✓ Documentation check complete"

# VimScript syntax validation
echo "[3/5] Checking VimScript syntax..."

# Check for proper function definitions
if ! grep -q "^function!" "autoload/guardrail.vim"; then
    echo "ERROR: autoload/guardrail.vim missing function definitions"
    exit 1
fi

# Check for proper script guard in plugin/guardrail.vim
if ! grep -q "if exists('g:loaded_guardrail')" "plugin/guardrail.vim"; then
    echo "WARNING: plugin/guardrail.vim should have load guard"
fi

echo "✓ VimScript structure valid"

# Security checks
echo "[4/5] Running security checks..."

# Check for shellescape usage (security fix applied)
if ! grep -q "shellescape" "autoload/guardrail.vim"; then
    echo "ERROR: autoload/guardrail.vim missing shellescape (shell injection risk)"
    exit 1
fi
echo "✓ shellescape properly used"

# Check for curl usage
if ! grep -q "curl" "autoload/guardrail.vim"; then
    echo "WARNING: No curl found - may be using different HTTP client"
fi

# Check for configurable variables
echo "[5/5] Checking configuration variables..."
config_vars=(
    "g:guardrail_server_url"
    "g:guardrail_api_key"
    "g:guardrail_project_slug"
    "g:guardrail_enabled"
)
for var in "${config_vars[@]}"; do
    if ! grep -q "$var" "autoload/guardrail.vim"; then
        echo "WARNING: Configuration variable $var not found"
    fi
done
echo "✓ Configuration variables check complete"

echo
echo "=== All Vim Plugin Tests Passed ==="
