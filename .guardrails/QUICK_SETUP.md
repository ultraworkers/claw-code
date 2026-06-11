# 🚀 Quick Setup Guide

> Get Agent Guardrails running in 5 minutes — for teams building with AI at full velocity

---

## TL;DR - The Absolute Basics

**Step 1:** Clone this template
```bash
git clone https://github.com/TheArchitectit/agent-guardrails-template.git
cd agent-guardrails-template
```

**Step 2:** Run setup script
```bash
python scripts/setup_agents.py --claude --full
```

**Step 3:** Done! 🎉

Your AI agent now has guardrails. Every time it edits code, it will:
- ✅ Read files before editing them
- ✅ Validate bash commands before running them
- ✅ Check git operations for safety
- ✅ Run pre-work checklists
- ✅ Ask for help when uncertain

---

## Detailed Setup (5 Minutes)

### Step 1: Get the Template (30 seconds)

```bash
# Clone the repository
git clone https://github.com/TheArchitectit/agent-guardrails-template.git

# Enter the directory
cd agent-guardrails-template

# Or download as ZIP if you prefer
# https://github.com/TheArchitectit/agent-guardrails-template/archive/refs/heads/main.zip
```

### Step 2: Choose Your AI Tool (1 minute)

This template works with **Claude Code**, **OpenCode**, or both.

**Option A: Claude Code (Anthropic)**
```bash
python scripts/setup_agents.py --claude --full
```

**Option B: OpenCode**
```bash
python scripts/setup_agents.py --opencode --full
```

**Option C: Both (Recommended)**
```bash
python scripts/setup_agents.py --claude --opencode --full
```

### Step 3: Verify Installation (30 seconds)

Check what was created:

```bash
# For Claude Code
ls -la .claude/

# For OpenCode
ls -la .opencode/
```

You should see:
- Configuration files
- Skills directories
- Hooks (for Claude Code)

### Step 4: Restart Your AI Tool (1 minute)

**Claude Code:**
```bash
# Exit and restart
claude
```

**OpenCode:**
```bash
# Restart the application
```

### Step 5: Test It (2 minutes)

Ask your AI to do something simple:

> "Create a test file called hello.txt with content 'Hello World'"

You should see the guardrails in action:
- Agent reads the request
- Agent checks scope
- Agent executes safely

---

## What Just Happened?

The setup script created:

### For Claude Code:
```
.claude/
├── settings.json          # Your Claude configuration
├── skills/                # Safety skills
│   ├── guardrails-enforcer/
│   ├── commit-validator/
│   └── env-separator/
└── hooks/                 # Pre/post execution hooks
    ├── pre-execution
    ├── post-execution
    └── pre-commit
```

### For OpenCode:
```
.opencode/
├── oh-my-opencode.jsonc   # Your OpenCode configuration
├── agents/                # Agent definitions
│   ├── guardrails-auditor.json
│   └── doc-indexer.json
└── skills/                # Safety skills
    ├── guardrails-enforcer/
    ├── commit-validator/
    └── env-separator/
```

---

## Daily Usage

### What You Don't Need To Do

- ❌ Manually configure anything
- ❌ Remember to turn it on
- ❌ Check every AI action

### What Happens Automatically

**When AI reads code:**
- ✅ Logs file access for audit trail

**When AI edits code:**
- ✅ Validates file was read first
- ✅ Checks scope boundaries
- ✅ Scans for secrets

**When AI runs commands:**
- ✅ Blocks dangerous commands (`rm -rf /`, etc.)
- ✅ Validates git operations
- ✅ Checks for forbidden patterns

**When AI commits:**
- ✅ Validates commit message format
- ✅ Ensures tests pass
- ✅ Checks for AI attribution

---

## Windows Setup

### Option 1: WSL2 (Recommended)

WSL2 provides the best compatibility with all guardrails features:

```powershell
# Install WSL2 with Ubuntu
wsl --install -d Ubuntu

# Then follow the Linux instructions inside WSL2
wsl
cd /mnt/c/Users/YourName/agent-guardrails-template
python3 scripts/setup_agents.py --claude --full
```

### Option 2: Native Windows (PowerShell)

```powershell
# Install Python from https://python.org or Microsoft Store
python --version

# Run setup
python scripts/setup_agents.py --claude --full

# Or for Cursor / VS Code
python scripts/setup_agents.py --cursor --full
```

### Option 3: Docker Desktop (Windows)

For the MCP server on Windows:

```powershell
# Install Docker Desktop: https://www.docker.com/products/docker-desktop
# Ensure WSL2 backend is enabled in Docker Desktop settings

# Deploy MCP server
cd mcp-server
# Use the Docker Compose instructions in DEPLOYMENT_GUIDE.md
```

### Windows-Specific Notes

- **Line endings:** Git may convert LF to CRLF. Configure with `git config --global core.autocrlf input`
- **Make:** Not installed by default. Use WSL2 or install via `choco install make`
- **Docker:** Docker Desktop with WSL2 backend is required for containerized MCP server
- **IDE Integration:** Cursor, VS Code, and Windsurf all support Windows natively

---

## Troubleshooting

### "Command not found: python"

Use `python3` instead:
```bash
python3 scripts/setup_agents.py --claude --full
```

On Windows PowerShell:
```powershell
python scripts/setup_agents.py --claude --full
```

### "Permission denied"

Make the script executable:
```bash
chmod +x scripts/setup_agents.py
python scripts/setup_agents.py --claude --full
```

On Windows, run PowerShell as Administrator if needed.

### "Nothing happened"

Check if Python is installed:
```bash
python --version
# or
python3 --version
```

Install Python if needed: https://python.org

### "AI isn't using guardrails"

1. Make sure you restarted the AI tool
2. Check that files were created in `.claude/` or `.opencode/`
3. Look at the AI's system prompt - it should mention guardrails

---

## Next Steps

### Learn More

- **For AI Safety:** Read [AGENT_GUARDRAILS.md](docs/AGENT_GUARDRAILS.md)
- **For Teams:** Read [HOW_TO_APPLY.md](docs/HOW_TO_APPLY.md) to apply to existing repos
- **For Customization:** Edit `.claude/skills/guardrails-enforcer/SKILL.md` or `.opencode/skills/guardrails-enforcer/SKILL.md`

### Apply to Your Own Repository

```bash
# Copy docs folder to your repo
cp -r docs /path/to/your/repo/

# Copy CLAUDE.md and .claudeignore
cp CLAUDE.md /path/to/your/repo/
cp .claudeignore /path/to/your/repo/

# Run setup in your repo
cd /path/to/your/repo
python /path/to/agent-guardrails-template/scripts/setup_agents.py --claude --full
```

### Update Regularly

```bash
# Pull latest template
git pull origin main

# Re-run setup to get updates
python scripts/setup_agents.py --claude --full
```

---

## What You Can Now Do

With guardrails in place, your AI agents are cleared for rapid development:

- **Generate code at full speed** — Agents know the safety boundaries, so they spend tokens building instead of safety-checking
- **Iterate without fear** — Rollback points and verification gates mean experiments are safe
- **Ship accessible by default** — WCAG 3.0+ compliance is baked into every component pattern
- **Catch ethical issues automatically** — Dark pattern detection runs on every UI generation
- **Scale to any platform** — Cross-platform patterns mean one generation works everywhere

You're not adding constraints. You're removing the need for agents to self-constrain on every decision.

---

## Quick Reference

### Key Commands

| Task | Command |
|------|---------|
| Full setup | `python scripts/setup_agents.py --claude --full` |
| Minimal setup | `python scripts/setup_agents.py --claude --minimal` |
| Remove setup | `python scripts/setup_agents.py --uninstall` |
| Check status | `ls -la .claude/` or `ls -la .opencode/` |

### Key Files

| File | Purpose |
|------|---------|
| `.claude/skills/guardrails-enforcer/SKILL.md` | Main safety rules |
| `.claude/hooks/pre-execution` | Pre-action validation |
| `docs/AGENT_GUARDRAILS.md` | Full documentation |
| `docs/HOW_TO_APPLY.md` | Apply to existing repos |

---

## Need Help?

- 📖 **Documentation:** See [INDEX_MAP.md](INDEX_MAP.md) for all docs
- 🐛 **Issues:** https://github.com/TheArchitectit/agent-guardrails-template/issues
- 💬 **Discussions:** GitHub Discussions tab

---

**That's it!** Your AI now has guardrails. Go build something amazing safely! 🚀
