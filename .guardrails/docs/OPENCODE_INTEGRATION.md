# OpenCode Integration

This guide explains how to integrate Agent Guardrails with OpenCode using custom agents and skills.

## Overview

OpenCode uses a multi-agent architecture with:
- **Configuration** - JSONC file (`.opencode/oh-my-opencode.jsonc`)
- **Agents** - Specialized workers for different tasks
- **Skills** - Markdown files with YAML frontmatter
- **MCP Servers** - Remote Model Context Protocol servers for guardrail enforcement

## MCP Server Configuration

### Remote MCP Server Setup

If you have a deployed Guardrail MCP server (see [README.md](../README.md) MCP Server section), configure OpenCode to connect to it:

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",
  
  "mcpServers": {
    "guardrails": {
      "type": "remote",
      "url": "http://your-server-ip:8094/mcp/v1/sse",
      "headers": {
        "Authorization": "Bearer YOUR_MCP_API_KEY"
      }
    }
  }
}
```

**Configuration Details:**

| Field | Description | Example |
|-------|-------------|---------|
| `type` | Must be `"remote"` for external MCP servers | `"remote"` |
| `url` | SSE endpoint URL with external port | `http://0.0.0.0:8094/mcp/v1/sse` |
| `headers.Authorization` | Bearer token with MCP_API_KEY | `Bearer JGtwbxsS2Nvy...` |

**Important Notes:**
- Use the **external port** (e.g., 8094), not the internal container port (8080)
- The `Authorization` header must use `Bearer` format
- Do NOT use `X-API-Key` header - it won't work
- Get the API key from your server's `.env` file (MCP_API_KEY variable)

### Verifying MCP Connection

Test that OpenCode can connect to the MCP server:

```bash
# Check MCP server health
curl http://your-server:8095/health/ready

# Should return: {"status":"ready",...}

# Check version
curl http://your-server:8095/version

# Should return: {"version":"v1.12.0",...}
```

## Setup

### 1. Run Setup Script

```bash
python scripts/setup_agents.py --opencode --full
```

This creates:
```
.opencode/
├── oh-my-opencode.jsonc
├── skills/
│   ├── guardrails-enforcer/
│   │   └── SKILL.md
│   ├── commit-validator/
│   │   └── SKILL.md
│   └── env-separator/
│       └── SKILL.md
└── agents/
    ├── guardrails-auditor.json
    └── doc-indexer.json
```

### 2. Verify Installation

Check the configuration:
```bash
cat .opencode/oh-my-opencode.jsonc
```

Check that skills exist:
```bash
ls -la .opencode/skills/*/
```

### 3. Restart OpenCode

OpenCode loads configuration on startup. Restart to apply changes.

## Configuration Format

OpenCode uses **JSONC** (JSON with Comments):

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",

  // Agent definitions
  "agents": {
    "guardrails-enforcer": {
      "model": "anthropic/claude-sonnet-4",
      "temperature": 0.1,
      "prompt_append": "Additional instructions..."
    }
  },

  // Skill configuration
  "skills": {
    "enable": ["guardrails-enforcer", "commit-validator"]
  }
}
```

## How It Works

### Agents

Agents are specialized workers with:
- `model` - Which LLM to use
- `temperature` - Creativity vs determinism (0.0-1.0)
- `prompt_append` - Additional system instructions
- `permissions` - Access controls

### Skills

Skills are Markdown files with YAML frontmatter:

```markdown
---
name: skill-name
description: "What this skill does"
---

# Skill Title

Instructions here...
```

### Categories

OpenCode uses cost-based categories:

| Category | Cost | Use For |
|----------|------|---------|
| `quick` | $ | Fast queries, greps |
| `unspecified-low` | $$ | Standard operations |
| `unspecified-high` | $$$ | Complex analysis |
| `visual-engineering` | $$$ | UI/UX work |

## Included Agents

### guardrails-enforcer

**Purpose:** Real-time enforcement of safety rules

**Configuration:**
- Model: `claude-sonnet-4`
- Temperature: 0.1 (deterministic)
- Permissions: Ask before edit/bash

**Behavior:**
- Validates read-before-edit
- Enforces scope boundaries
- Checks for halt conditions

### guardrails-auditor

**Purpose:** Post-execution compliance review

**Configuration:**
- Model: `claude-sonnet-4`
- Read-only permissions
- No edit or bash access

**Behavior:**
- Reviews completed work
- Reports violations
- Suggests corrections

### doc-indexer

**Purpose:** Keeps documentation maps updated

**Configuration:**
- Model: `claude-haiku-4` (fast, cheap)
- Edit permissions for docs only

**Behavior:**
- Updates INDEX_MAP.md
- Updates HEADER_MAP.md
- Runs on document changes

## Included Skills

### guardrails-enforcer

**Activates:** All operations

**Enforces:**
- Four Laws of Agent Safety
- Pre-operation checklist
- Halt conditions
- Three Strikes Rule

### commit-validator

**Activates:** Before commits

**Validates:**
- AI attribution
- Single focus
- No secrets
- Tests passing

### env-separator

**Activates:** Test code creation

**Enforces:**
- Production code first
- Separate instances
- No data mixing

## Customization

### Adding a Custom Agent

1. Create a JSON file in `.opencode/agents/`:

```json
{
  "name": "my-agent",
  "model": "anthropic/claude-sonnet-4",
  "temperature": 0.1,
  "prompt_append": "Your instructions...",
  "permissions": {
    "edit": "ask",
    "bash": "deny",
    "read": "allow"
  }
}
```

2. Reference it in `oh-my-opencode.jsonc`:

```jsonc
{
  "agents": {
    "my-agent": {
      "model": "anthropic/claude-sonnet-4",
      // ...
    }
  }
}
```

3. Restart OpenCode

### Adding a Custom Skill

1. Create a directory in `.opencode/skills/`:

```bash
mkdir .opencode/skills/my-skill
```

2. Create `SKILL.md`:

```markdown
---
name: my-skill
description: "What this skill does"
---

# My Skill

Instructions here...
```

3. Enable in `oh-my-opencode.jsonc`:

```jsonc
{
  "skills": {
    "enable": ["guardrails-enforcer", "my-skill"]
  }
}
```

4. Restart OpenCode

## Advanced Configuration

### Category Overrides

Change which models categories use:

```jsonc
{
  "categories": {
    "quick": {
      "model": "opencode/gpt-5-nano"
    },
    "visual-engineering": {
      "model": "google/gemini-3-pro"
    }
  }
}
```

### Permission Tuning

Fine-grain agent permissions:

```jsonc
{
  "agents": {
    "explore": {
      "permission": {
        "edit": "deny",
        "bash": "ask",
        "webfetch": "allow"
      }
    }
  }
}
```

Options: `allow`, `ask`, `deny`

### Model Parameters

Advanced model configuration:

```jsonc
{
  "agents": {
    "my-agent": {
      "model": "anthropic/claude-opus-4",
      "temperature": 0.1,
      "top_p": 0.9,
      "maxTokens": 4096,
      "thinking": {
        "budget_tokens": 2000
      }
    }
  }
}
```

## Troubleshooting

### Configuration Not Loading

**Check:**
- JSONC syntax is valid (use a JSONC validator)
- `$schema` URL is correct
- File is at `.opencode/oh-my-opencode.jsonc`

### Skills Not Activating

**Check:**
- Skill is listed in `skills.enable` array
- SKILL.md has valid YAML frontmatter
- Skill directory name matches skill name

### Agent Not Responding

**Check:**
- Agent defined in `agents` section
- Model identifier is valid
- Permissions allow the operation

## Best Practices

1. **Use temperature 0.1 for safety agents** - More deterministic
2. **Enable skills explicitly** - Don't rely on auto-discovery
3. **Set restrictive permissions** - Default to `ask` for destructive ops
4. **Version control** - Commit `.opencode/` to share with team
5. **Document custom agents** - Add purpose and usage notes

## Differences from Claude Code

| Feature | Claude Code | OpenCode |
|---------|-------------|----------|
| Config format | JSON | JSONC |
| Skills location | `.claude/skills/*.json` | `.opencode/skills/*/SKILL.md` |
| Agents | Implicit | Explicit definition |
| Hooks | Shell scripts | Not supported |
| Categories | N/A | Cost-based routing |

## References

- [OpenCode Documentation](https://github.com/code-yeongyu/opencode)
- [Oh My OpenCode Schema](https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json)
- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - General setup guide
