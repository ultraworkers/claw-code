# MCP Auto Checkpoint Documentation

> Integration with MCP servers for automatic checkpointing.

**Related:** [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md) | [ROLLBACK_PROCEDURES.md](./ROLLBACK_PROCEDURES.md)

---

## Overview

This document describes how to integrate MCP (Model Context Protocol) checkpoint servers to automatically create state snapshots before and after critical operations. Checkpoints enable recovery to known-good states if something goes wrong.

---

## Checkpoint Concepts

### What is a Checkpoint?

A checkpoint is a saved state that can be restored later:

```
CHECKPOINT CAPTURES:
- Git state (current commit, branch, staged/unstaged changes)
- File contents at a point in time
- Task progress (which to-dos completed)
- Session context (what was being worked on)
```

### When Checkpoints Occur

```
AUTOMATIC CHECKPOINTS (if configured):
- Before starting a to-do item
- After completing a to-do item
- Before risky operations
- At user-defined intervals

MANUAL CHECKPOINTS:
- When user requests
- Before experimental changes
- At significant milestones
```

---

## Integrating with MCP Servers

### Prerequisites

```
REQUIRED:
- MCP server with checkpoint capability running
- Server accessible via configured endpoint
- Proper authentication configured (if needed)

CHECK CONNECTION:
- MCP server responds to health check
- Checkpoint tools available in tool list
```

### Supported MCP Checkpoint Servers

| Server Type | Checkpoint Method | Recovery Method |
|-------------|-------------------|-----------------|
| Memory MCP | Store/retrieve state | Recall checkpoint |
| Git-based | Git stash/commit | Git restore |
| Custom | Server-specific | Server-specific |

### MCP Checkpoint Tools

If your MCP server provides checkpoint tools, they typically include:

```
COMMON CHECKPOINT TOOLS:

create_checkpoint(name, metadata)
  - Creates new checkpoint
  - Returns checkpoint ID

list_checkpoints()
  - Lists available checkpoints
  - Shows checkpoint metadata

restore_checkpoint(checkpoint_id)
  - Restores to checkpoint state
  - Returns success/failure

delete_checkpoint(checkpoint_id)
  - Removes checkpoint
  - Frees resources
```

---

## Checkpoint Workflow

### Standard Checkpoint Flow

```
START TASK
    ↓
[CHECKPOINT: before-task-start]
    ↓
COMPLETE WORK
    ↓
VALIDATE
    ↓
[CHECKPOINT: after-task-complete]
    ↓
COMMIT
```

### Checkpoint Around To-Dos

```
TO-DO LIST:

[ ] To-do 1: Implement feature
    ├── [CHECKPOINT: before-todo-1]
    ├── Complete work
    ├── Validate
    ├── Commit
    └── [CHECKPOINT: after-todo-1]

[ ] To-do 2: Add tests
    ├── [CHECKPOINT: before-todo-2]
    ├── Complete work
    ├── Validate
    ├── Commit
    └── [CHECKPOINT: after-todo-2]
```

---

## Checkpoint-Aware Sprint Design

### Adding Checkpoints to Sprints

When writing sprint documents, include checkpoint annotations:

```markdown
### STEP 1: Make code change

**Pre-Step Checkpoint:**
[MCP: create_checkpoint("before-step-1")]

**Action:** Edit the file...

**Post-Step Checkpoint:**
[MCP: create_checkpoint("after-step-1")]
```

### Sprint Template with Checkpoints

```markdown
## STEP-BY-STEP EXECUTION

### STEP 1: [Title]

**Checkpoint:** create_checkpoint("pre-step-1")

**Action:** [What to do]

**Checkpoint:** create_checkpoint("post-step-1")

**Decision Point:**
- [ ] Success → Proceed to STEP 2
- [ ] Failure → restore_checkpoint("pre-step-1")
```

---

## Recovery Procedures

### Restoring from Checkpoint

```
RECOVERY WORKFLOW:

1. Identify the checkpoint to restore
   - list_checkpoints()
   - Find last known-good state

2. Restore the checkpoint
   - restore_checkpoint(checkpoint_id)
   - Wait for confirmation

3. Verify restoration
   - Check file contents
   - Check git status
   - Verify task state

4. Resume from restored state
   - Re-attempt failed operation
   - Or take different approach
```

### Recovery Decision Matrix

| Situation | Recovery Action |
|-----------|-----------------|
| Step failed, changes uncommitted | Restore pre-step checkpoint |
| Commit made but wrong | Restore pre-commit checkpoint + git reset |
| Multiple steps failed | Restore earliest safe checkpoint |
| Checkpoint not available | Use git rollback instead |

### Combining Checkpoints with Git

```
BELT AND SUSPENDERS APPROACH:

Before risky operation:
1. Create MCP checkpoint
2. Create git commit (if appropriate)

If operation fails:
1. Try MCP checkpoint restore
2. If that fails, use git rollback
3. If both fail, manual recovery
```

---

## Configuration Templates

### MCP Server Configuration

```json
{
  "mcpServers": {
    "checkpoint-server": {
      "command": "checkpoint-mcp",
      "args": ["--storage", "/path/to/checkpoints"],
      "env": {
        "CHECKPOINT_RETENTION": "7d"
      }
    }
  }
}
```

### Checkpoint Naming Convention

```
FORMAT: <scope>-<action>-<identifier>

EXAMPLES:
  sprint-2026-01-14-before-start
  todo-1-before-edit
  todo-1-after-complete
  risky-operation-before
  milestone-feature-complete
```

---

## Best Practices

### Checkpoint Frequency

```
RECOMMENDED FREQUENCY:

- Before each to-do: ALWAYS
- After each to-do: ALWAYS
- During long operations: Every 10-15 minutes
- Before risky changes: ALWAYS
- After recovery: ALWAYS
```

### Checkpoint Naming

```
GOOD NAMES:
  sprint-123-before-db-migration
  feature-auth-after-tests-pass
  hotfix-security-pre-patch

BAD NAMES:
  checkpoint1
  temp
  backup
```

### Checkpoint Retention

```
RETENTION POLICY:

- Recent checkpoints (< 1 day): Keep all
- Older checkpoints (1-7 days): Keep milestone checkpoints
- Very old (> 7 days): Archive or delete

Cleanup command (if supported):
  cleanup_checkpoints(older_than="7d")
```

---

## Troubleshooting

### Checkpoint Creation Fails

```
POSSIBLE CAUSES:
- MCP server not running
- Storage full
- Permission issues
- Network connectivity

RESOLUTION:
1. Check MCP server status
2. Check storage availability
3. Verify permissions
4. Test network connectivity
5. Fall back to git-based checkpoints
```

### Checkpoint Restore Fails

```
POSSIBLE CAUSES:
- Checkpoint corrupted
- Checkpoint expired/deleted
- State conflicts

RESOLUTION:
1. Try different checkpoint
2. Use git rollback instead
3. Manual state recovery
4. Report to user for guidance
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              MCP CHECKPOINT QUICK REFERENCE                       |
+------------------------------------------------------------------+
| CREATE CHECKPOINT:                                                |
|   create_checkpoint("descriptive-name")                           |
|                                                                   |
| LIST CHECKPOINTS:                                                 |
|   list_checkpoints()                                              |
|                                                                   |
| RESTORE CHECKPOINT:                                               |
|   restore_checkpoint(checkpoint_id)                               |
+------------------------------------------------------------------+
| WHEN TO CHECKPOINT:                                               |
|   • Before each to-do                                             |
|   • After each to-do                                              |
|   • Before risky operations                                       |
+------------------------------------------------------------------+
| IF CHECKPOINT UNAVAILABLE:                                        |
|   Use git rollback: git checkout HEAD -- <file>                   |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~300
