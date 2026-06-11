# System Architecture

> Architecture diagrams and component documentation for Agent Guardrails Template

**Version:** 1.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Component Diagram](#component-diagram)
3. [Data Flow](#data-flow)
4. [Team Structure](#team-structure)
5. [Deployment Architecture](#deployment-architecture)
6. [Integration Points](#integration-points)

---

## High-Level Architecture

```mermaid
flowchart TB
    subgraph Client["Client Layer"]
        CLI["CLI / Scripts"]
        IDE["IDE Integration"]
        CI["CI/CD Pipeline"]
    end

    subgraph MCP["MCP Layer"]
        Server["MCP Server<br/>Port 8094"]
        Router["Request Router"]
        Validator["Input Validator"]
    end

    subgraph Tools["Tool Layer"]
        TeamTools["Team Tools"]
        GuardrailTools["Guardrail Tools"]
        AgentTools["Agent Tools"]
    end

    subgraph Backend["Backend Layer (Go)"]
        TeamMgr["Team Manager<br/>mcp-server/internal/team/"]
        RuleEngine["Rule Engine<br/>mcp-server/internal/rules/"]
        AuditLog["Audit Logger<br/>mcp-server/internal/audit/"]
    end

    subgraph Storage["Storage Layer"]
        TeamConfig[".teams/*.json"]
        Guardrails[".guardrails/"]
        Logs[".mcp/logs/"]
    end

    CLI -->|HTTP/JSON-RPC| Server
    IDE -->|HTTP/JSON-RPC| Server
    CI -->|HTTP/JSON-RPC| Server

    Server --> Router
    Router --> Validator
    Validator --> TeamTools
    Validator --> GuardrailTools
    Validator --> AgentTools

    TeamTools --> TeamMgr
    GuardrailTools --> RuleEngine
    AgentTools --> TeamMgr

    TeamMgr --> TeamConfig
    TeamMgr --> AuditLog
    RuleEngine --> Guardrails
    AuditLog --> Logs
```

### Architecture Overview

The Agent Guardrails Template follows a layered architecture with clear separation of concerns:

> **Go Implementation:** All backend services are implemented in Go (v2.6.0+). See `mcp-server/internal/`.

| Layer | Responsibility | Components |
|-------|---------------|------------|
| Client | Interface with users | CLI, IDE, CI/CD |
| MCP | Protocol handling | Server, routing, validation |
| Tools | Business logic | Team, guardrail, agent operations |
| Backend | Core services (Go) | Team manager, rules engine, audit logger |
| Storage | Persistence | PostgreSQL, Redis, JSON configs |

---

## Component Diagram

### MCP Server Components

```mermaid
flowchart LR
    subgraph MCP_Server["MCP Server"]
        direction TB

        subgraph Transport["Transport Layer"]
            HTTP["HTTP Listener<br/>Port 8094"]
            JSONRPC["JSON-RPC Handler"]
        end

        subgraph Core["Core Services"]
            Session["Session Manager"]
            ToolReg["Tool Registry"]
            Middleware["Middleware Stack"]
        end

        subgraph Tools["Available Tools"]
            T1["guardrail_team_init"]
            T2["guardrail_team_list"]
            T3["guardrail_team_assign"]
            T4["guardrail_team_unassign"]
            T5["guardrail_team_status"]
            T6["guardrail_phase_gate_check"]
            T7["guardrail_agent_team_map"]
            T8["guardrail_team_size_validate"]
            T9["guardrail_validate_bash"]
            T10["guardrail_validate_git_operation"]
            T11["guardrail_validate_file_edit"]
        end
    end

    HTTP --> JSONRPC
    JSONRPC --> Session
    Session --> Middleware
    Middleware --> ToolReg
    ToolReg --> T1
    ToolReg --> T2
    ToolReg --> T3
    ToolReg --> T4
    ToolReg --> T5
    ToolReg --> T6
    ToolReg --> T7
    ToolReg --> T8
    ToolReg --> T9
    ToolReg --> T10
    ToolReg --> T11
```

### Team Manager Components

```mermaid
flowchart TB
    subgraph Team_Manager["Team Manager"]
        direction TB

        API["API Layer"]

        subgraph Business_Logic["Business Logic"]
            Init["Initialize Teams"]
            Assign["Assign/Unassign"]
            Validate["Validate Teams"]
            Status["Check Status"]
            Gates["Phase Gates"]
        end

        subgraph Data_Access["Data Access"]
            ConfigLoader["Config Loader"]
            ConfigWriter["Config Writer"]
            Validator["Data Validator"]
        end

        subgraph Models["Team Models"]
            Team["Team (1-12)"]
            Role["Role"]
            Member["Member"]
            Phase["Phase (1-5)"]
        end
    end

    API --> Business_Logic
    Init --> ConfigWriter
    Assign --> ConfigWriter
    Validate --> ConfigLoader
    Status --> ConfigLoader
    Gates --> ConfigLoader

    ConfigLoader --> Models
    ConfigWriter --> Models
    Validator --> Models
```

---

## Data Flow

### Tool Execution Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Server as MCP Server
    participant Validator as Input Validator
    participant Tool as Tool Handler
    participant Backend as Backend Service
    participant Storage as Storage

    Client->>Server: HTTP POST /mcp/v1/message
    Server->>Server: Parse JSON-RPC request
    Server->>Validator: Validate input parameters

    alt Validation Failed
        Validator-->>Server: ValidationError
        Server-->>Client: 400 Bad Request
    else Validation Passed
        Validator->>Tool: Route to tool handler
        Tool->>Backend: Execute business logic
        Backend->>Storage: Read/Write data
        Storage-->>Backend: Data response
        Backend-->>Tool: Operation result
        Tool-->>Server: Tool response
        Server-->>Client: 200 OK + result
    end
```

### Team Assignment Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Server as MCP Server
    participant TeamTool as Team Tool
    participant TeamMgr as Team Manager
    participant File as .teams/{project}.json

    Client->>Server: guardrail_team_assign
    Server->>TeamTool: Route request
    TeamTool->>TeamTool: Validate parameters

    alt Invalid Parameters
        TeamTool-->>Server: Error: TEAM-002/TEAM-003
    else Valid Parameters
        TeamTool->>TeamMgr: assign_role(project, team, role, person)
        TeamMgr->>File: Read current config
        File-->>TeamMgr: Team configuration

        alt Team Full
            TeamMgr-->>TeamTool: Error: TEAM-005
        else Role Occupied
            TeamMgr-->>TeamTool: Error: TEAM-004
        else Success
            TeamMgr->>File: Write updated config
            TeamMgr->>TeamMgr: Log audit event
            TeamMgr-->>TeamTool: Assignment confirmed
        end
    end

    TeamTool-->>Server: Response
    Server-->>Client: JSON-RPC result
```

### Phase Gate Check Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Server as MCP Server
    participant GateTool as Phase Gate Tool
    participant TeamMgr as Team Manager
    participant Rules as .guardrails/team-layout-rules.json
    participant Config as .teams/{project}.json

    Client->>Server: guardrail_phase_gate_check
    Server->>GateTool: Route request
    GateTool->>Rules: Load gate requirements
    Rules-->>GateTool: Gate rules

    GateTool->>TeamMgr: Get phase status
    TeamMgr->>Config: Read team config
    Config-->>TeamMgr: All team data

    TeamMgr-->>GateTool: Phase status summary

    GateTool->>GateTool: Compare requirements vs actual

    alt Gate Requirements Met
        GateTool-->>Server: Gate approved, can proceed
    else Gate Requirements Not Met
        GateTool-->>Server: Missing deliverables list
    end

    Server-->>Client: Gate check result
```

---

## Team Structure

### 12-Team Organization

```mermaid
flowchart TB
    subgraph Phase1["Phase 1: Strategy & Planning"]
        T1["Team 1<br/>Business & Product<br/>Strategy"]
        T2["Team 2<br/>Enterprise<br/>Architecture"]
        T3["Team 3<br/>GRC"]
    end

    subgraph Phase2["Phase 2: Platform & Foundation"]
        T4["Team 4<br/>Infrastructure &<br/>Cloud Ops"]
        T5["Team 5<br/>Platform<br/>Engineering"]
        T6["Team 6<br/>Data Governance &<br/>Analytics"]
    end

    subgraph Phase3["Phase 3: The Build Squads"]
        T7["Team 7<br/>Core Feature<br/>Squad"]
        T8["Team 8<br/>Middleware &<br/>Integration"]
    end

    subgraph Phase4["Phase 4: Validation & Hardening"]
        T9["Team 9<br/>Cybersecurity<br/>AppSec"]
        T10["Team 10<br/>Quality<br/>Engineering"]
    end

    subgraph Phase5["Phase 5: Delivery & Sustainment"]
        T11["Team 11<br/>Site Reliability<br/>Engineering"]
        T12["Team 12<br/>IT Operations &<br/>Support"]
    end

    Phase1 -->|Gate 1| Phase2
    Phase2 -->|Gate 2| Phase3
    Phase3 -->|Gate 3| Phase4
    Phase4 -->|Gate 4| Phase5
```

### Phase Gate Flow

```mermaid
flowchart LR
    subgraph Gates["Phase Gates"]
        direction LR

        G1["Gate 1<br/>Architecture<br/>Review Board"]
        G2["Gate 2<br/>Environment<br/>Readiness"]
        G3["Gate 3<br/>Feature Complete"]
        G4["Gate 4<br/>Security + QA<br/>Sign-off"]
    end

    P1["Phase 1<br/>Strategy"] --> G1 --> P2["Phase 2<br/>Platform"]
    P2 --> G2 --> P3["Phase 3<br/>Build"]
    P3 --> G3 --> P4["Phase 4<br/>Validate"]
    P4 --> G4 --> P5["Phase 5<br/>Deliver"]
```

---

## Deployment Architecture

### Single Node Deployment

```mermaid
flowchart TB
    subgraph Server["Single Server"]
        subgraph Docker["Docker Container"]
            MCP["MCP Server<br/>Port 8094"]
            Scripts["Team Manager Scripts"]
        end

        subgraph Volume1["Config Volume"]
            Teams[".teams/"]
            Guardrails[".guardrails/"]
        end

        subgraph Volume2["Log Volume"]
            Logs[".mcp/logs/"]
        end
    end

    Client["Client"]

    Client -->|HTTP| MCP
    MCP --> Scripts
    Scripts --> Teams
    Scripts --> Guardrails
    MCP --> Logs
```

### Production Deployment

```mermaid
flowchart TB
    subgraph Clients["Client Layer"]
        CLI["CLI Tools"]
        IDEs["IDE Extensions"]
        CICD["CI/CD Runners"]
    end

    subgraph LB["Load Balancer"]
        Nginx["Nginx / ALB"]
    end

    subgraph AppServers["Application Servers"]
        S1["MCP Server 1"]
        S2["MCP Server 2"]
        S3["MCP Server 3"]
    end

    subgraph SharedStorage["Shared Storage"]
        NFS["NFS / EFS"]
        Teams["Team Configs"]
        Rules["Guardrail Rules"]
    end

    subgraph Database["Database"]
        Postgres[(PostgreSQL)]
        Audit["Audit Logs"]
    end

    subgraph Monitoring["Monitoring"]
        Prometheus["Prometheus"]
        Grafana["Grafana"]
    end

    CLI -->|HTTPS| Nginx
    IDEs -->|HTTPS| Nginx
    CICD -->|HTTPS| Nginx

    Nginx --> S1
    Nginx --> S2
    Nginx --> S3

    S1 --> NFS
    S2 --> NFS
    S3 --> NFS

    S1 --> Postgres
    S2 --> Postgres
    S3 --> Postgres

    S1 --> Prometheus
    S2 --> Prometheus
    S3 --> Prometheus

    NFS --> Teams
    NFS --> Rules
    Postgres --> Audit
```

---

## Integration Points

### External System Integrations

```mermaid
flowchart LR
    subgraph AGT["Agent Guardrails Template"]
        MCP["MCP Server"]
        Scripts["Management Scripts"]
    end

    subgraph External["External Systems"]
        Git["Git Provider<br/>GitHub/GitLab"]
        CI["CI/CD<br/>Jenkins/Actions"]
        Auth["Auth Provider<br/>OAuth/SAML"]
        Monitor["Monitoring<br/>Datadog/NewRelic"]
    end

    subgraph AI["AI Assistants"]
        Claude["Claude Code"]
        OpenCode["OpenCode"]
        Cursor["Cursor"]
    end

    MCP <-->|Git Operations| Git
    MCP <-->|Pipeline Triggers| CI
    MCP <-->|Authentication| Auth
    Scripts <-->|Metrics Export| Monitor

    Claude <-->|MCP Protocol| MCP
    OpenCode <-->|MCP Protocol| MCP
    Cursor <-->|MCP Protocol| MCP
```

### API Integration Patterns

```mermaid
flowchart TB
    subgraph Integration["Integration Patterns"]
        direction TB

        P1["Synchronous<br/>Request/Response"]
        P2["Asynchronous<br/>Webhook"]
        P3["Batch<br/>File-based"]
    end

    subgraph UseCases["Use Cases"]
        UC1["Team Assignments"]
        UC2["Phase Gate Checks"]
        UC3["Validation"]
        UC4["Audit Export"]
        UC5["Bulk Updates"]
    end

    P1 --> UC1
    P1 --> UC2
    P1 --> UC3
    P2 --> UC4
    P3 --> UC5
```

---

## Security Architecture

### Authentication Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Server as MCP Server
    participant Auth as Auth Service
    participant Tool as Tool Handler

    Client->>Server: Request + API Key
    Server->>Auth: Validate API Key

    alt Invalid Key
        Auth-->>Server: 401 Unauthorized
        Server-->>Client: AUTH-002 Error
    else Valid Key
        Auth-->>Server: User + Permissions
        Server->>Server: Check permissions

        alt Insufficient Permissions
            Server-->>Client: AUTH-003 Error
        else Authorized
            Server->>Tool: Execute request
            Tool-->>Server: Result
            Server-->>Client: Success response
        end
    end
```

### Data Protection

```mermaid
flowchart TB
    subgraph Security["Security Layers"]
        L1["Input Validation<br/>Sanitization"]
        L2["Authentication<br/>Authorization"]
        L3["Audit Logging<br/>Non-repudiation"]
        L4["Encryption<br/>At Rest & Transit"]
    end

    subgraph Threats["Threat Mitigation"]
        T1["Injection Attacks"]
        T2["Unauthorized Access"]
        T3["Data Tampering"]
        T4["Data Breach"]
    end

    L1 --> T1
    L2 --> T2
    L3 --> T3
    L4 --> T4
```

---

## Configuration Architecture

> **Go Implementation:** All backend logic is implemented in Go. See `mcp-server/internal/` for package structure.
> **Migration:** `team_manager.py` has been migrated to Go (v2.6.0). See [PYTHON_MIGRATION.md](PYTHON_MIGRATION.md).

### File Organization

```
/mnt/ollama/git/agent-guardrails-template/
├── mcp-server/
│   ├── internal/                    # Go implementation
│   │   ├── team/                    # Team management logic
│   │   ├── rules/                   # Rule engine
│   │   ├── audit/                   # Audit logging
│   │   ├── database/                # Database operations
│   │   ├── cache/                   # Redis caching
│   │   ├── mcp/                     # MCP protocol
│   │   └── web/                     # HTTP handlers
│   └── cmd/server/                  # Main entry point
├── .teams/
│   ├── {project-name}.json          # Team configurations
│   └── backups/
│       └── *.json.bak               # Automatic backups
├── .guardrails/
│   ├── rules.json                   # Validation rules
│   ├── team-layout-rules.json       # Team structure rules
│   └── schemas/
│       └── team-config.schema.json  # JSON Schema
├── .mcp/
│   ├── mcp.log                      # Server logs
│   ├── audit.log                    # Security audit logs
│   └── config.json                  # Server configuration
└── scripts/
    └── setup_agents.py              # Agent setup (Python - legacy)
```

---

**Last Updated:** 2026-02-15
**Version:** 2.6.0
**Implementation:** Go (mcp-server/internal/)
