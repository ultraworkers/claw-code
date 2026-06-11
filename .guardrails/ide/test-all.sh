#!/bin/bash
# Master test gate for all IDE extensions
# Runs all individual test gates and reports overall status

set -e

cd "$(dirname "$0")"

echo "=========================================="
echo "  IDE Extensions Master Test Gate"
echo "=========================================="
echo

FAILED=0
PASSED=0

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

run_test() {
    local name=$1
    local script=$2

    echo "Running $name tests..."
    if bash "$script"; then
        echo -e "${GREEN}✓ $name tests PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ $name tests FAILED${NC}"
        FAILED=$((FAILED + 1))
    fi
    echo
}

# Make scripts executable
chmod +x test-*.sh

# Run all tests
run_test "VS Code" "test-vscode.sh"
run_test "JetBrains" "test-jetbrains.sh"
run_test "Neovim" "test-neovim.sh"
run_test "Vim" "test-vim.sh"

echo "=========================================="
echo "  Test Results Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All IDE extension tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. See above for details.${NC}"
    exit 1
fi
