# Local OpenAI-compatible providers and skills setup

This guide covers two common offline/local workflows:

1. running Brewcode against an OpenAI-compatible local model server such as Ollama, llama.cpp, or vLLM; and
2. installing local skills from disk so Brewcode can discover them without network access.

## Brewcode is not Claude-only

Brew Code is a Claude-Code-shaped workflow/runtime, not a Claude-only product. It supports Anthropic directly and can target OpenAI-compatible, provider-routed, and local models depending on configuration. Non-Claude providers are supported honestly: they may require stricter tool-call and response-shape compatibility, and some slash/tool workflows can be rougher than first-party Anthropic/OpenAI paths. Provider-specific identity leaks are bugs, not intended product positioning.

If you need the most polished daily-driver experience for a specific non-Claude model today, compare that provider’s native tools. If you need runtime/provider hackability, Brewcode’s OpenAI-compatible route is the intended extension path.

## OpenAI-compatible routing basics

Set `OPENAI_BASE_URL` to the server’s `/v1` endpoint and set `OPENAI_API_KEY` to either the required token or a harmless placeholder for local servers that expect an Authorization header. The model name must match what the server exposes.

```bash
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="local-dev-token"
brewcode --model "openai/qwen3:latest" prompt "Reply exactly HELLO_WORLD_123"
```

### How the `openai/` prefix interacts with each backend

Brewcode identifies the gateway behind your `OPENAI_BASE_URL` and adjusts the model name in the wire request accordingly. You always pass `--model "openai/<name>"` to brewcode's CLI parser; what reaches the server depends on the gateway:

| Gateway (detected from `OPENAI_BASE_URL`) | Example URL | Wire `model` field |
|---|---|---|
| **OpenAI** (default) | `https://api.openai.com/v1` | `<name>` — prefix stripped |
| **OpenRouter** | `https://openrouter.ai/api/v1` | `openai/<name>` — slug preserved |
| **Ollama Cloud** | `https://ollama.com/v1` | `<name>` — prefix stripped |
| **Ollama (local)** | `http://127.0.0.1:11434/v1`, `localhost:11434`, `0.0.0.0:11434` | `<name>` — prefix stripped |
| **xAI** | `https://api.x.ai/v1` | `<name>` — prefix stripped |
| **DashScope** | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `<name>` — prefix stripped |
| **Generic** (anything else: vLLM, llama.cpp, LiteLLM, in-house proxies) | any custom URL | `<name>` — prefix stripped |

OpenRouter is the only backend whose protocol genuinely uses slash-namespaced model slugs (`openai/gpt-4.1-mini`, `anthropic/claude-3.7-sonnet`, etc.). Every other OpenAI-compatible backend wants the bare model name and will 404 if the routing prefix leaks into the request body.

Routing notes:

- Use the `openai/` prefix on the CLI to make prefix routing win over ambient Anthropic credentials, regardless of which OpenAI-compatible backend you are hitting. Example: `--model "openai/gpt-oss:20b"` with `OPENAI_BASE_URL=https://ollama.com/v1` sends `"model": "gpt-oss:20b"` to Ollama Cloud.
- For local servers, the exact model ID reported by the server is what reaches the wire — `qwen3:latest`, `llama3.2`, `Qwen/Qwen2.5-Coder-7B-Instruct`, etc. The `openai/` prefix is stripped, but slash-containing IDs after the prefix are preserved.
- If you have multiple provider keys in your environment, remove unrelated keys while smoke-testing a local route or choose a model prefix that unambiguously selects the intended provider.
- Tool workflows need model/server support for OpenAI-compatible tool calls. Plain prompt smoke tests can pass even when slash/tool workflows still fail because the server returns an incompatible tool-call shape.

## Raw `/v1/chat/completions` smoke test

Before debugging Brewcode, verify the local server speaks the expected wire format:

```bash
curl -sS "$OPENAI_BASE_URL/chat/completions" \
  -H "Authorization: Bearer ${OPENAI_API_KEY:-local-dev-token}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:latest",
    "messages": [{"role": "user", "content": "Reply exactly HELLO_WORLD_123"}],
    "stream": false
  }'
```

Expected result: a JSON response with one assistant message containing `HELLO_WORLD_123`. If this fails, fix the local server, model name, or auth token before changing Brewcode settings.

## Ollama

Start Ollama and pull a model:

```bash
ollama pull qwen3:latest
ollama serve
```

In another shell:

```bash
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="local-dev-token"
brewcode --model "qwen3:latest" prompt "Reply exactly HELLO_WORLD_123"
```

If Ollama is running without auth and your build accepts authless local OpenAI-compatible servers, `unset OPENAI_API_KEY` is also acceptable. Use a placeholder token rather than a real cloud API key for local testing.

## llama.cpp server

Start a llama.cpp OpenAI-compatible server with the model name you want Brewcode to send:

```bash
llama-server -m ./models/qwen2.5-coder.gguf --host 127.0.0.1 --port 8080 --alias qwen2.5-coder
```

Then smoke-test through Brewcode:

```bash
export OPENAI_BASE_URL="http://127.0.0.1:8080/v1"
export OPENAI_API_KEY="local-dev-token"
brewcode --model "qwen2.5-coder" prompt "Reply exactly HELLO_WORLD_123"
```

## vLLM or another OpenAI-compatible server

Start vLLM with an OpenAI-compatible API server:

```bash
vllm serve Qwen/Qwen2.5-Coder-7B-Instruct --host 127.0.0.1 --port 8000
```

Then route Brewcode to it:

```bash
export OPENAI_BASE_URL="http://127.0.0.1:8000/v1"
export OPENAI_API_KEY="local-dev-token"
brewcode --model "Qwen/Qwen2.5-Coder-7B-Instruct" prompt "Reply exactly HELLO_WORLD_123"
```

## Local skills install from disk

Skills are discovered from Brewcode skill roots such as `.brewcode/skills/` in a workspace and `~/.brewcode/skills/` for user-level installs. Legacy `.codex/skills/` roots may also be scanned for compatibility, but new local Brewcode projects should prefer `.brewcode/skills/`.

A skill directory should contain a `SKILL.md` file with frontmatter:

```text
my-skill/
└── SKILL.md
```

```markdown
---
name: my-skill
description: Explain when this skill should be used.
---

# My Skill

Instructions for the agent go here.
```

Install a skill from a local path in the interactive REPL:

```text
/skills install /absolute/path/to/my-skill
/skills list
/skills my-skill
```

Or inspect skills from the direct CLI surface:

```bash
brewcode skills --output-format json
```

Offline install checklist:

- Install the specific skill directory, not only the repository root, unless that repository root itself contains `SKILL.md`.
- Keep the frontmatter `name` aligned with the directory name users will type.
- After installing, run `/skills list` or `brewcode skills --output-format json` to confirm the discovered name and source path.
- If a skill invocation fails with an HTTP/provider error, the skill may have installed correctly but the current model/provider call failed. Run `brewcode doctor`, verify provider credentials, and try a simple prompt smoke test before reinstalling the skill.

## Troubleshooting

| Symptom | Check |
|---|---|
| Brewcode still asks for Anthropic credentials | Use an explicit OpenAI-compatible model route or remove unrelated Anthropic env vars during local smoke tests. |
| `model not found` from local server | Use the exact model ID exposed by Ollama/llama.cpp/vLLM. |
| Plain prompt works but tools fail | Confirm the model/server supports OpenAI-compatible tool calls and response shapes. |
| Skill says installed but `/skills <name>` fails | Check `/skills list` for the discovered name and source; verify provider credentials separately with `brewcode doctor`. |
| A local docs/log file contains secrets | Redact it before using `@path` file context or attaching it to an issue. |
