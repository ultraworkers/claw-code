Project Sentinel: The Active Guardrail PlatformVersion: 3.0.0-EnterpriseStatus: Production ReadyArchitecture: Sentinel Core🛡️ What Is Project Sentinel?Project Sentinel is an active governance layer for Autonomous AI Agents. Unlike passive templates that rely on an LLM "remembering" to follow rules, Sentinel uses a compiled Model Context Protocol (MCP) server to physically enforce safety, security, and financial guardrails.It transforms the "Soft Laws" of documentation into the "Hard Physics" of the development environment.Core CapabilitiesVFS Jail: Prevents Agents from accessing restricted paths (.env, ~/.ssh).State Machine: Enforces strict SDLC states (Planning -> Active -> Review).FinOps: Tracks token usage and enforces budget caps ($/sprint).Polyglot: Native toolchains for Go, Python, TS, Rust, Java, and 10+ others.📂 Repository StructureThe repository is organized into specific domains. Agents should query the INDEX_MAP.md to find specific rules..
├── bin/                    # Compiled Sentinel binaries
├── services/
│   └── sentinel/           # The Sentinel Core (Go Source)
│       ├── cmd/            # Entry points
│       ├── internal/       # Kernel, Jailor, Auditor, Ledger
│       └── tools/          # MCP Tool Definitions
├── docs/
│   ├── SENTINEL_ARCH.md    # System Architecture
│   ├── workflows/          # Operational Protocols (Commit, Deploy, Review)
│   ├── standards/          # Coding Standards (API, Logging, Secrets)
│   ├── languages/          # Language-Specific Profiles (Go, Python, etc.)
│   └── setup/              # Installation & Config
├── sprints/                # Active Sprint Data (if not using DB)
└── sentinel.toml           # The Policy Configuration
🚀 Quick Start1. Installation# Download and install the background service
curl -L [https://releases.project-sentinel.io/install.sh](https://releases.project-sentinel.io/install.sh) | sh
sentinel init --root .
2. Connect Your AgentAdd the Sentinel MCP server to your Agent configuration (Claude Desktop, OpenCode, Cursor):{
  "mcpServers": {
    "sentinel": {
      "command": "./bin/sentinel",
      "args": ["serve", "--stdio"]
    }
  }
}
3. Start a SprintAsk your Agent:"Initialize a new sprint for the Auth Service refactor. Create tasks for Login, Logout, and Password Reset."🤖 Agent Instructions: Maintaining This READMEProtocol for AI Agents:Trigger: When you add a new top-level directory or significantly change the architecture.Action: You must update the Repository Structure tree above.Constraint: Do not remove the INDEX_MAP link.Verification: Ensure sentinel --version matches the version badge at the top.License: BSD-3-Clause | Maintainer: TheArchitectit