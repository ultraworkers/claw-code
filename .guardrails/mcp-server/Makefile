.PHONY: all build test clean docker-build docker-run migrate-up migrate-down lint vuln sbom security-scan help

# Variables
BINARY_NAME=guardrail-mcp
DOCKER_IMAGE=guardrail-mcp:latest
MIGRATIONS_DIR=internal/database/migrations

# Version information (set via CI/CD or git)
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
BUILD_TIME ?= $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Go build flags
LDFLAGS=-ldflags "-w -s \
	-X main.version=$(VERSION) \
	-X main.buildTime=$(BUILD_TIME) \
	-X main.gitCommit=$(GIT_COMMIT)"

# Default target
.DEFAULT_GOAL := help

help: ## Show this help message
	@echo "Guardrail MCP Server - Available Targets"
	@echo "========================================="
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

all: lint test build ## Run lint, test, and build

# Build the binary
build: ## Build the server binary
	@echo "Building $(BINARY_NAME) version $(VERSION)..."
	go build $(LDFLAGS) -o bin/$(BINARY_NAME) ./cmd/server
	@echo "Build complete: bin/$(BINARY_NAME)"

# Build for multiple platforms
build-all: ## Build for multiple platforms (linux, darwin)
	@echo "Building for multiple platforms..."
	GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-linux-amd64 ./cmd/server
	GOOS=linux GOARCH=arm64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-linux-arm64 ./cmd/server
	GOOS=darwin GOARCH=amd64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-darwin-amd64 ./cmd/server
	GOOS=darwin GOARCH=arm64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-darwin-arm64 ./cmd/server
	@echo "Multi-platform build complete"

# Run tests
test: ## Run all tests
	go test -v -race ./...

# Run tests with coverage
test-coverage: ## Run tests with coverage report
	go test -v -race -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

# Clean build artifacts
clean: ## Remove build artifacts
	rm -rf bin/
	rm -f $(BINARY_NAME)
	rm -f coverage.out coverage.html

# Build Docker image
docker-build: ## Build Docker image with version info
	@echo "Building Docker image $(DOCKER_IMAGE)..."
	podman build \
		--build-arg VERSION=$(VERSION) \
		--build-arg BUILD_TIME=$(BUILD_TIME) \
		--build-arg GIT_COMMIT=$(GIT_COMMIT) \
		-t $(DOCKER_IMAGE) \
		-f deploy/Dockerfile .
	@echo "Docker image built: $(DOCKER_IMAGE)"

# Build Docker image for multiple architectures
docker-build-multi: ## Build multi-arch Docker image
	@echo "Building multi-arch Docker image..."
	podman build \
		--platform linux/amd64,linux/arm64 \
		--build-arg VERSION=$(VERSION) \
		--build-arg BUILD_TIME=$(BUILD_TIME) \
		--build-arg GIT_COMMIT=$(GIT_COMMIT) \
		-t $(DOCKER_IMAGE) \
		-f deploy/Dockerfile .

# Run locally with docker-compose
docker-up: ## Start services with docker-compose
	@echo "Starting services..."
	VERSION=$(VERSION) BUILD_TIME=$(BUILD_TIME) GIT_COMMIT=$(GIT_COMMIT) \
		podman-compose -f deploy/podman-compose.yml up -d

# Stop docker-compose
docker-down: ## Stop docker-compose services
	podman-compose -f deploy/podman-compose.yml down

# View logs
docker-logs: ## View docker-compose logs
	podman-compose -f deploy/podman-compose.yml logs -f

# Database migrations
migrate-up: ## Run database migrations up
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "Error: DATABASE_URL not set"; \
		echo "Usage: make migrate-up DATABASE_URL=postgres://user:pass@localhost/dbname"; \
		exit 1; \
	fi
	migrate -path $(MIGRATIONS_DIR) -database "$(DATABASE_URL)" up

migrate-down: ## Run database migrations down (one step)
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "Error: DATABASE_URL not set"; \
		echo "Usage: make migrate-down DATABASE_URL=postgres://user:pass@localhost/dbname"; \
		exit 1; \
	fi
	migrate -path $(MIGRATIONS_DIR) -database "$(DATABASE_URL)" down 1

migrate-create: ## Create a new migration file (usage: make migrate-create NAME=description)
	@if [ -z "$(NAME)" ]; then \
		echo "Error: NAME not set"; \
		echo "Usage: make migrate-create NAME=add_users_table"; \
		exit 1; \
	fi
	migrate create -ext sql -dir $(MIGRATIONS_DIR) -seq $(NAME)

# Development
dev: ## Run server in development mode
	go run ./cmd/server

# Format code
fmt: ## Format Go code
	go fmt ./...
	gofumpt -w .

# Run linter
lint: ## Run golangci-lint
	golangci-lint run --timeout 5m

# Run linter with fix
lint-fix: ## Run golangci-lint with auto-fix
	golangci-lint run --fix --timeout 5m

# Generate mocks (if needed)
generate: ## Generate Go code (mocks, etc.)
	go generate ./...

# Download dependencies
deps: ## Download and tidy Go modules
	go mod download
	go mod tidy
	go mod verify

# Check for vulnerabilities
vuln: ## Check for known vulnerabilities in dependencies
	@echo "Checking for vulnerabilities..."
	govulncheck ./...

# Security scan with Trivy
trivy-scan: ## Scan Docker image for vulnerabilities
	@echo "Scanning Docker image with Trivy..."
	trivy image --severity HIGH,CRITICAL --exit-code 1 $(DOCKER_IMAGE)

# Generate SBOM
sbom: ## Generate Software Bill of Materials
	@echo "Generating SBOM..."
	trivy fs --format cyclonedx -o sbom.json .

# Full security scan
security-scan: vuln trivy-scan sbom ## Run all security scans

# CI/CD target
ci: deps lint test build security-scan ## Run CI pipeline locally

# Check code quality
quality: fmt lint test ## Run code quality checks

# Run server with health check
health-check: ## Run health check against local server
	@./bin/$(BINARY_NAME) --health-check

# Show version
version: ## Show version information
	@echo "Version:    $(VERSION)"
	@echo "Build Time: $(BUILD_TIME)"
	@echo "Git Commit: $(GIT_COMMIT)"
