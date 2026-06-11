# Vision Pipeline

Visual feedback and iterative review pipeline for 3D game development.

## Overview

Screenshots from a running Godot game are fed to a local llama.cpp Nemotron vision model for analysis. The system iteratively reviews images, documents findings, and falls back to hosted models (Claude/GPT-4) when local inference is insufficient.

## Architecture

```
Godot Game -> Screenshots -> File Watcher -> Review Engine -> Local LLM
                                                    |
                                              Fallback -> Anthropic / OpenAI
                                                    |
                                               SQLite Storage
                                                    |
                                             MCP Tools + HTTP API
```

## Components

| Component | Location | Role |
|-----------|----------|------|
| `llama-server` | `/home/user001/llama.cpp/build/bin/llama-server` | Local inference |
| Nemotron Omni | `/mnt/data/models/nemotron-30b-a3b/` | Vision-capable LLM |
| Go Modules | `mcp-server/internal/vision/` | Pipeline implementation |
| Godot Addon | `godot/addons/vision_capture/` | Screenshot capture |

## Go Modules

- `models.go` — Review, Iteration, Finding structs
- `inference.go` — InferenceClient interface
- `local_llama.go` — Primary: direct llama-server client
- `anthropic.go` — Fallback 1: Anthropic API
- `openai.go` — Fallback 2: OpenAI API
- `composite.go` — Orchestrator: local -> fallback chain
- `review_engine.go` — Iterative review state machine
- `capture.go` — File watcher + trigger
- `storage.go` — SQLite persistence

## MCP Tools

Enabled when `VISION_ENABLED=true`:

- `vision_capture_screenshot` — Trigger immediate capture
- `vision_analyze_screenshot` — Submit image for review
- `vision_iterate_review` — Re-review with context
- `vision_get_report` — Retrieve documented report
- `vision_check_health` — Backend health status
- `vision_guardrail_check` — Full pipeline + 3D validation

## HTTP API

Mounted at `/v1/vision` on the web server port:

| Method | Path | Description |
|--------|------|-------------|
| POST | `/review` | Submit image for review |
| GET | `/review/:id` | Get review report |
| POST | `/review/:id/iterate` | Run another round |
| GET | `/reviews` | List reviews (query: `limit`) |
| GET | `/events` | SSE stream |
| POST | `/capture/trigger` | Trigger capture |
| GET | `/health` | Backend health |

## Configuration

Copy `mcp-server/config/vision.example.yaml` to `vision.yaml` and fill values, or use environment variables:

| Variable | Purpose | Default |
|----------|---------|---------|
| `VISION_ENABLED` | Enable vision tools | `false` |
| `LOCAL_LLAMA_URL` | llama-server endpoint | `http://localhost:8080/v1` |
| `LOCAL_LLAMA_MODEL` | Model name | `nemotron-vision-local` |
| `FALLBACK_PROVIDER` | `anthropic` or `openai` | (none) |
| `FALLBACK_MODEL` | Hosted model name | (none) |
| `FALLBACK_API_KEY` | API key for fallback | (none) |
| `SCREENSHOT_DIR` | Watch directory | `./screenshots` |
| `VISION_DB_PATH` | SQLite DB path | `./vision_reviews.db` |

## Docker

The docker-compose includes vision environment variables and volumes:

```yaml
volumes:
  - ${SCREENSHOT_DIR:-./screenshots}:/app/screenshots
  - vision_db:/app/vision_data
```

The Dockerfile uses `CGO_ENABLED=1` with static linking to support SQLite in the distroless final image.

## Godot Integration

Copy `godot/addons/vision_capture/` into your Godot project's `addons/` directory and enable the plugin in Project Settings. Screenshots are saved to `user://screenshots/` on a configurable interval.

## Starting llama-server

```bash
/home/user001/llama.cpp/build/bin/llama-server \
  -m /mnt/data/models/nemotron-30b-a3b/NVIDIA-Nemotron-3-Nano-Omni-30B-A3B-Reasoning-UD-Q4_K_XL.gguf \
  --mmproj /mnt/data/models/nemotron-30b-a3b/mmproj-F16.gguf \
  -c 120000 --flash-attn \
  --port 8080
```

## Security

- `vision.yaml` is in `.gitignore` — never commit secrets.
- API keys are loaded from environment variables at runtime.
- The example config uses `${VAR}` placeholders only.
