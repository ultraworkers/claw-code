# Team CLI

A command-line interface for managing standardized team layouts across projects.

## Overview

The Team CLI provides a fast, intuitive interface for the team management system defined in `scripts/team_manager.py`. It supports all core team operations including initialization, role assignments, status tracking, and backups.

## Installation

### From Source

```bash
cd cmd/team-cli
make install
```

Or manually:

```bash
cd cmd/team-cli
go build -o team .
```

### Cross-Platform Build

```bash
make cross-compile
```

This creates binaries for:
- Linux (amd64, arm64)
- macOS (amd64, arm64)
- Windows (amd64)

## Usage

```
team [command] [flags]
```

### Global Flags

- `-p, --project string` - Project name (required for most commands)
- `-o, --output string` - Output format: `text`, `json` (default: `text`)
- `--version` - Show version information

## Commands

### init

Initialize a new project with the standardized 12-team structure.

```bash
team init my-project
```

### list

List all teams for a project.

```bash
team list -p my-project
team list -p my-project --phase "Phase 1"
```

### assign

Assign a person to a role.

```bash
team assign -p my-project -t 7 -r "Technical Lead" --person "Jane Developer"
```

### unassign

Remove a person from a role.

```bash
team unassign -p my-project -t 7 -r "Technical Lead"
```

### start

Mark a team as started/in-progress.

```bash
team start -p my-project -t 7
```

### complete

Mark a team as completed.

```bash
team complete -p my-project -t 7
```

### status

Show project or phase status.

```bash
team status -p my-project
team status -p my-project --phase "Phase 1"
```

### validate

Validate team sizes meet the 4-6 member requirement.

```bash
team validate -p my-project
```

### phase-gate

Check phase gate requirements.

```bash
team phase-gate -p my-project --from 1 --to 2
```

### agent-map

Get team mapping for an agent type.

```bash
team agent-map backend
team agent-map security
```

Supported agent types:
- `planner` - Team 2, Phase 1
- `architect` - Team 2, Phase 1
- `infrastructure` - Team 4, Phase 2
- `platform` - Team 5, Phase 2
- `backend` - Team 7, Phase 3
- `frontend` - Team 7, Phase 3
- `security` - Team 9, Phase 4
- `qa` - Team 10, Phase 4
- `sre` - Team 11, Phase 5
- `ops` - Team 12, Phase 5

### export

Export project data.

```bash
team export -p my-project -f json
team export -p my-project -f csv
```

### import

Import team assignments from a file.

```bash
team import -p my-project -f data.json --format json
team import -p my-project -f data.csv --format csv
```

### backup

List available backups.

```bash
team backup -p my-project
```

### restore

Restore from a backup.

```bash
team restore -p my-project -b .teams/backups/my-project_20250215_120000.json.gz
```

### delete

Delete a team or entire project.

```bash
# Delete a specific team
team delete -p my-project -t 7

# Delete entire project (with confirmation)
team delete -p my-project

# Force delete without confirmation
team delete -p my-project --force
```

## Examples

### Initialize and Setup a Project

```bash
# Create new project
team init web-platform

# Assign team members
team assign -p web-platform -t 2 -r "Solution Architect" --person "Alice Johnson"
team assign -p web-platform -t 2 -r "Domain Architect" --person "Bob Smith"
team assign -p web-platform -t 4 -r "Cloud Architect" --person "Carol White"
team assign -p web-platform -t 7 -r "Technical Lead" --person "David Brown"

# Check status
team status -p web-platform

# Validate team sizes
team validate -p web-platform
```

### Check Phase Gate

```bash
# Validate phase 1 is complete before moving to phase 2
team phase-gate -p web-platform --from 1 --to 2
```

### JSON Output for Scripting

```bash
# Get status in JSON format for automation
team status -p web-platform -o json | jq '.teams[] | select(.phase == "Phase 1")'
```

## Environment Variables

- `TEAM_MANAGER_PATH` - Path to the `team_manager.py` script (optional)
- `TEAM_ENCRYPTION_KEY` - Key for encrypted project data (optional)

## Requirements

- Go 1.23.2 or later
- Python 3.x (for team_manager.py backend)
- team_manager.py must be accessible (usually in `../../scripts/` relative to the binary)

## Development

### Build

```bash
make build
```

### Test

```bash
make test
```

### Format

```bash
make fmt
```

### Lint

```bash
make lint
```

## Architecture

The Team CLI is a Go application that wraps the existing `team_manager.py` Python script. Commands are translated to Python subprocess calls, with output formatted for the terminal using Charm's Lipgloss and Log libraries.

## License

Part of the Agent Guardrails Template project.
