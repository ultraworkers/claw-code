# Team CLI Makefile
# Build automation for the team management CLI tool

.PHONY: all build clean test install lint fmt deps

# Build variables
BINARY_NAME := team
BINARY_NAME_ALT := team-cli
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
BUILD_TIME := $(shell date -u '+%Y-%m-%d_%H:%M:%S')
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
LDFLAGS := -ldflags "-X main.version=$(VERSION) -X main.buildTime=$(BUILD_TIME) -X main.gitCommit=$(GIT_COMMIT)"

# Go parameters
GOCMD := go
GOBUILD := $(GOCMD) build
GOCLEAN := $(GOCMD) clean
GOTEST := $(GOCMD) test
GOGET := $(GOCMD) get
GOMOD := $(GOCMD) mod
GOFMT := $(GOCMD) fmt

# Installation directories
PREFIX := /usr/local
BINDIR := $(PREFIX)/bin

# Default target
all: deps build

# Download dependencies
deps:
	$(GOMOD) tidy
	$(GOMOD) download

# Build the binary
build:
	$(GOBUILD) $(LDFLAGS) -o $(BINARY_NAME) .

# Build with optimizations for release
release:
	CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o $(BINARY_NAME) .

# Build for multiple platforms
cross-compile:
	# Linux AMD64
	GOOS=linux GOARCH=amd64 CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o dist/$(BINARY_NAME)-linux-amd64 .
	# Linux ARM64
	GOOS=linux GOARCH=arm64 CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o dist/$(BINARY_NAME)-linux-arm64 .
	# macOS AMD64
	GOOS=darwin GOARCH=amd64 CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o dist/$(BINARY_NAME)-darwin-amd64 .
	# macOS ARM64
	GOOS=darwin GOARCH=arm64 CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o dist/$(BINARY_NAME)-darwin-arm64 .
	# Windows AMD64
	GOOS=windows GOARCH=amd64 CGO_ENABLED=0 $(GOBUILD) $(LDFLAGS) -ldflags "-s -w" -trimpath -o dist/$(BINARY_NAME)-windows-amd64.exe .

# Run tests
test:
	$(GOTEST) -v -race ./...

# Run tests with coverage
test-coverage:
	$(GOTEST) -v -race -coverprofile=coverage.out ./...
	$(GOCMD) tool cover -html=coverage.out -o coverage.html

# Format code
fmt:
	$(GOFMT) .

# Run linter (requires golangci-lint)
lint:
	@which golangci-lint > /dev/null || (echo "golangci-lint not found, installing..." && go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest)
	golangci-lint run ./...

# Install binary to system
install: build
	@mkdir -p $(BINDIR)
	@install -m 755 $(BINARY_NAME) $(BINDIR)/$(BINARY_NAME)
	@ln -sf $(BINDIR)/$(BINARY_NAME) $(BINDIR)/$(BINARY_NAME_ALT) 2>/dev/null || true
	@echo "Installed $(BINARY_NAME) to $(BINDIR)"

# Uninstall binary
uninstall:
	@rm -f $(BINDIR)/$(BINARY_NAME)
	@rm -f $(BINDIR)/$(BINARY_NAME_ALT)
	@echo "Uninstalled $(BINARY_NAME) from $(BINDIR)"

# Clean build artifacts
clean:
	$(GOCLEAN)
	@rm -f $(BINARY_NAME)
	@rm -f $(BINARY_NAME_ALT)
	@rm -rf dist/
	@rm -f coverage.out coverage.html

# Verify build works
verify: build
	./$(BINARY_NAME) --version
	./$(BINARY_NAME) --help

# Development mode with hot reload (requires air)
dev:
	@which air > /dev/null || (echo "air not found, installing..." && go install github.com/cosmtrek/air@latest)
	air -c .air.toml

# Update dependencies
update-deps:
	$(GOMOD) tidy
	$(GOMOD) update all

# Print build info
info:
	@echo "Binary: $(BINARY_NAME)"
	@echo "Version: $(VERSION)"
	@echo "Build Time: $(BUILD_TIME)"
	@echo "Git Commit: $(GIT_COMMIT)"
	@echo "Go Version: $(shell go version)"
