#!/usr/bin/env bash
set -euo pipefail

# Load configuration
ENV_FILE="${1:-/mnt/data/git/agent-guardrails-template/mcp-server/config/llama-server.env}"
if [[ -f "$ENV_FILE" ]]; then
    set -a
    source "$ENV_FILE"
    set +a
else
    echo "Config file not found: $ENV_FILE" >&2
    exit 1
fi

exec /home/user001/llama.cpp/build/bin/llama-server \
    -m "${LLAMA_MODEL}" \
    --mmproj "${LLAMA_MMPROJ}" \
    -c "${LLAMA_CTX_SIZE}" \
    -ctk "${LLAMA_CACHE_TYPE_K}" \
    -ctv "${LLAMA_CACHE_TYPE_V}" \
    --flash-attn "${LLAMA_FLASH_ATTN}" \
    --port "${LLAMA_PORT}" \
    --host "${LLAMA_HOST}"
