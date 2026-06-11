# Build stage
FROM golang:1.23-alpine AS builder

WORKDIR /app

# Install build dependencies (gcc/g++/musl-dev required for CGO + SQLite)
RUN apk add --no-cache git ca-certificates tzdata gcc g++ musl-dev

# Copy go mod files
COPY go.mod go.sum ./
RUN go mod download && go mod verify

# Copy source code
COPY . .

# Build arguments for versioning
ARG VERSION=dev
ARG BUILD_TIME
ARG GIT_COMMIT

# Build the server with ldflags for version info
# CGO_ENABLED=1 required for SQLite; static linking keeps the binary self-contained
RUN CGO_ENABLED=1 GOOS=linux go build \
    -a -installsuffix cgo \
    -tags "sqlite_omit_load_extension" \
    -ldflags "-w -s -linkmode external -extldflags \"-static\" \
        -X main.version=${VERSION} \
        -X main.buildTime=${BUILD_TIME} \
        -X main.gitCommit=${GIT_COMMIT}" \
    -o server ./cmd/server

# Final stage - distroless with ca-certificates for TLS
FROM gcr.io/distroless/static:nonroot

# Copy CA certificates from builder for TLS connections
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo

# Non-root user (distroless nonroot user)
USER 65532:65532

# Copy binary
COPY --from=builder --chown=65532:65532 /app/server /server

# Copy Web UI files
COPY --from=builder --chown=65532:65532 /app/web /app/web

# Copy prevention rules for syncing
COPY --from=builder --chown=65532:65532 /app/.guardrails/prevention-rules /app/docs

# Expose ports
# 8080 - MCP server
# 8081 - Web UI
# 8082 - Health/metrics (optional separation)
EXPOSE 8080 8081

# No HEALTHCHECK in Dockerfile - use container orchestrator health checks
# This avoids the issue of running the binary incorrectly
# For Docker Compose/Kubernetes, use:
#   healthcheck:
#     test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:8081/health/live"]
#     interval: 30s
#     timeout: 5s
#     retries: 3
#     start_period: 10s

ENTRYPOINT ["/server"]
