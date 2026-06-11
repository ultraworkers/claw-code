#!/bin/bash
# Test gate for VS Code extension
# Validates TypeScript compilation and basic extension structure

set -e

cd "$(dirname "$0")/vscode-extension"

echo "=== VS Code Extension Test Gate ==="
echo

# Check Node.js version
echo "[1/5] Checking Node.js..."
if ! command -v node &> /dev/null; then
    echo "ERROR: Node.js not found"
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt "16" ]; then
    echo "ERROR: Node.js 16+ required (found $(node -v))"
    exit 1
fi
echo "✓ Node.js $(node -v)"

# Check required files
echo "[2/5] Checking required files..."
required_files=("package.json" "tsconfig.json" "src/extension.ts")
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Required file missing: $file"
        exit 1
    fi
done
echo "✓ All required files present"

# Install dependencies
echo "[3/5] Installing dependencies..."
if [ ! -d "node_modules" ]; then
    npm install --silent 2>&1 | grep -v "npm WARN" || true
fi
echo "✓ Dependencies installed"

# TypeScript compilation
echo "[4/5] Compiling TypeScript..."
npm run compile 2>&1 | tee /tmp/vscode-compile.log
if [ $? -ne 0 ]; then
    echo "ERROR: TypeScript compilation failed"
    echo "See /tmp/vscode-compile.log for details"
    exit 1
fi

# Check output exists
if [ ! -d "out" ] || [ ! -f "out/extension.js" ]; then
    echo "ERROR: Compiled output not found"
    exit 1
fi
echo "✓ TypeScript compilation successful"

# Lint check (optional, don't fail if eslint not configured)
echo "[5/5] Running lint check..."
if npm run lint 2>/dev/null; then
    echo "✓ Lint passed"
else
    echo "⚠ Lint warnings (non-blocking)"
fi

echo
echo "=== All VS Code Extension Tests Passed ==="
