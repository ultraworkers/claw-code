#!/bin/bash
# Team CLI Installation Script

set -e

REPO_URL="https://github.com/thearchitectit/agent-guardrails-template"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="team"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64)
        ARCH="amd64"
        ;;
    arm64|aarch64)
        ARCH="arm64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

case "$OS" in
    linux)
        PLATFORM="linux"
        ;;
    darwin)
        PLATFORM="darwin"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Installing team CLI for $PLATFORM-$ARCH..."

# Check if running from source
if [ -f "cmd/team-cli/team" ]; then
    echo "Installing from local build..."
    cp "cmd/team-cli/team" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
elif [ -f "team" ]; then
    echo "Installing from current directory..."
    cp "team" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
else
    echo "Binary not found. Please build from source with:"
    echo "  cd cmd/team-cli && make build"
    exit 1
fi

echo "team CLI installed to $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "Run 'team --help' to get started"
