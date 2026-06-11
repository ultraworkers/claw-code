# Troubleshooting Guide

> Common issues, solutions, and recovery procedures for Agent Guardrails Template

**Version:** 1.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Common Errors](#common-errors)
3. [Debug Mode](#debug-mode)
4. [Log Analysis](#log-analysis)
5. [Performance Issues](#performance-issues)
6. [Recovery Procedures](#recovery-procedures)
7. [Getting Help](#getting-help)

---

## Quick Diagnostics

Run this checklist to quickly identify common issues:

```bash
# 1. Verify installation
curl -s http://localhost:8094/mcp/v1/health | jq .

# 2. Check configuration
ls -la .teams/
ls -la .guardrails/

# 3. Validate tools
python scripts/team_manager.py --validate

# 4. Test MCP connection
curl -s -X POST http://localhost:8094/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/list"}' | jq .
```

**Expected Results:**
- Health endpoint returns `{"status": "ok"}`
- `.teams/` and `.guardrails/` directories exist
- Team manager script exits with code 0
- Tools list returns available guardrail tools

---

## Common Errors

### MCP Server Connection Failed

**Symptoms:**
```
Error: Connection refused (localhost:8094)
Error: Could not connect to MCP server
```

**Causes:**
- MCP server not running
- Wrong port configuration
- Firewall blocking connection

**Solutions:**

1. **Start the MCP server:**
   ```bash
   python mcp_server.py
   # or
   ./start-mcp-server.sh
   ```

2. **Verify port configuration:**
   ```bash
   # Check if server is listening
   netstat -tlnp | grep 8094
   # or
   lsof -i :8094
   ```

3. **Check firewall rules:**
   ```bash
   # For Linux
   sudo ufw allow 8094/tcp
   # For macOS
   sudo pfctl -e
   ```

---

### Team Initialization Failed

**Symptoms:**
```
Error: TEAM-001: Team not found
Error: Failed to initialize project
```

**Causes:**
- Project name contains invalid characters
- `.teams/` directory does not exist
- Permission issues

**Solutions:**

1. **Verify project name:**
   ```bash
   # Valid: my-project, project_123, team-alpha
   # Invalid: my project, project;rm -rf /
   ```

2. **Create required directories:**
   ```bash
   mkdir -p .teams/
   chmod 755 .teams/
   ```

3. **Check permissions:**
   ```bash
   ls -la .teams/
   # Should be writable by current user
   ```

---

### Team Size Violation (TEAM-007)

**Symptoms:**
```
Error: TEAM-005: Team size violation
Team 7 has 8 members (maximum is 6)
```

**Causes:**
- Too many members assigned to a team
- Batch assignment exceeded limits

**Solutions:**

1. **Check current team sizes:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_team_size_validate",
         "arguments":{"project_name":"my-project"}
       }
     }'
   ```

2. **Remove excess members:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_team_unassign",
         "arguments":{
           "project_name":"my-project",
           "team_id":7,
           "role_name":"Extra Role"
         }
       }
     }'
   ```

3. **Rebalance across teams:**
   - Move members to teams with fewer than 4 members
   - Split large teams into multiple smaller teams

---

### Phase Gate Check Failed

**Symptoms:**
```
Error: Phase gate requirements not met
Missing deliverables: Architecture Decision Records
```

**Causes:**
- Required deliverables not complete
- Required teams not assigned
- Approval not obtained

**Solutions:**

1. **List gate requirements:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_phase_gate_check",
         "arguments":{
           "project_name":"my-project",
           "from_phase":1,
           "to_phase":2
         }
       }
     }'
   ```

2. **Complete missing deliverables:**
   - See [Phase Gates](#phase-gate-requirements) section
   - Submit required documents
   - Obtain approvals

3. **Verify team assignments:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_team_status",
         "arguments":{
           "project_name":"my-project",
           "phase":"Phase 1"
         }
       }
     }'
   ```

---

### Role Already Assigned

**Symptoms:**
```
Error: TEAM-004: Person already assigned
Role 'Technical Lead' already has 'Alice Johnson' assigned
```

**Causes:**
- Attempted to assign to an occupied role
- Duplicate assignment in batch script

**Solutions:**

1. **Unassign current person:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_team_unassign",
         "arguments":{
           "project_name":"my-project",
           "team_id":7,
           "role_name":"Technical Lead"
         }
       }
     }'
   ```

2. **Assign new person:**
   ```bash
   curl -X POST http://localhost:8094/mcp/v1/message \
     -d '{
       "jsonrpc":"2.0",
       "method":"tools/call",
       "params":{
         "name":"guardrail_team_assign",
         "arguments":{
           "project_name":"my-project",
           "team_id":7,
           "role_name":"Technical Lead",
           "person":"New Lead Name"
         }
       }
     }'
   ```

---

### API Key Authentication Failed

**Symptoms:**
```
Error: AUTH-001: Authentication required
Error: AUTH-002: Invalid API key
```

**Causes:**
- Missing Authorization header
- Expired or revoked API key
- Incorrect API key format

**Solutions:**

1. **Verify header format:**
   ```bash
   curl -H "Authorization: Bearer YOUR_API_KEY" ...
   ```

2. **Generate new API key:**
   ```bash
   python scripts/generate_api_key.py
   ```

3. **Check key permissions:**
   ```bash
   python scripts/verify_api_key.py YOUR_API_KEY
   ```

---

## Debug Mode

Enable debug mode to get detailed logging for troubleshooting.

### Enable Debug Logging

**Option 1: Environment Variable**
```bash
export MCP_DEBUG=1
export MCP_LOG_LEVEL=debug
python mcp_server.py
```

**Option 2: Configuration File**
```json
// .mcp/config.json
{
  "logging": {
    "level": "debug",
    "file": ".mcp/mcp.log",
    "console": true
  }
}
```

**Option 3: Command Line Flag**
```bash
python mcp_server.py --debug --log-file .mcp/debug.log
```

### Debug Output Examples

**Normal Operation:**
```
[DEBUG] Received request: tools/call
[DEBUG] Tool: guardrail_team_list
[DEBUG] Parameters: {"project_name": "my-project"}
[DEBUG] Execution time: 45ms
[INFO] Response sent successfully
```

**Error Condition:**
```
[DEBUG] Received request: tools/call
[DEBUG] Tool: guardrail_team_assign
[DEBUG] Parameters: {"team_id": 99, ...}
[ERROR] Validation failed: Invalid team_id
[ERROR] Error code: TEAM-002
[DEBUG] Stack trace:
  File "scripts/team_manager.py", line 45, in validate_team
    raise InvalidTeamError(f"Team {team_id} not found")
[INFO] Error response sent
```

---

## Log Analysis

### Log File Locations

| Component | Log File | Description |
|-----------|----------|-------------|
| MCP Server | `.mcp/mcp.log` | Main server logs |
| Team Manager | `.mcp/team_manager.log` | Team operations |
| Validation | `.mcp/validation.log` | Guardrail checks |
| Audit | `.mcp/audit.log` | Security events |

### Log Format

```
[TIMESTAMP] [LEVEL] [COMPONENT] Message
```

**Example:**
```
2026-02-15 14:32:15 [INFO] [MCP] Server started on port 8094
2026-02-15 14:32:18 [DEBUG] [TEAM] Validating team assignment
2026-02-15 14:32:18 [ERROR] [VALID] TEAM-002: Invalid team ID
```

### Common Log Patterns

**Startup Issues:**
```bash
# Check for port binding errors
grep "Address already in use" .mcp/mcp.log

# Check for permission denied
grep "Permission denied" .mcp/mcp.log

# Check for missing files
grep "No such file" .mcp/mcp.log
```

**Authentication Issues:**
```bash
# Find failed authentication attempts
grep "AUTH-" .mcp/audit.log

# List unauthorized access attempts
grep "403\|Unauthorized" .mcp/audit.log
```

**Performance Issues:**
```bash
# Find slow requests
grep "slow\|timeout" .mcp/mcp.log

# High response times
awk '/Execution time/ && $NF > 1000 {print}' .mcp/mcp.log
```

### Log Rotation

Configure automatic log rotation to prevent disk space issues:

```bash
# Add to crontab
crontab -e

# Rotate logs daily at midnight
0 0 * * * /usr/sbin/logrotate /etc/logrotate.d/mcp
```

**logrotate configuration:**
```
# /etc/logrotate.d/mcp
/mnt/ollama/git/agent-guardrails-template/.mcp/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 user user
}
```

---

## Performance Issues

### Slow Team Operations

**Symptoms:**
- Team list takes > 5 seconds
- Batch assignments timeout
- Phase gate checks are slow

**Diagnosis:**
```bash
# Check response times
time curl -s -X POST http://localhost:8094/mcp/v1/message \
  -d '...team_list...'

# Monitor server resources
htop
iostat -x 1
```

**Solutions:**

1. **Enable caching:**
   ```json
   {
     "cache": {
       "enabled": true,
       "ttl": 30
     }
   }
   ```

2. **Optimize batch operations:**
   ```bash
   # Use parallel processing
   python scripts/batch_execute.py --parallel 8
   ```

3. **Increase timeouts:**
   ```bash
   curl --max-time 30 ...
   ```

---

### High Memory Usage

**Symptoms:**
- MCP server using > 500MB RAM
- System swapping
- Out of memory errors

**Diagnosis:**
```bash
# Check memory usage
ps aux | grep mcp_server
free -h

# Monitor over time
watch -n 1 'ps -o pid,rss,cmd -p $(pgrep -f mcp_server)'
```

**Solutions:**

1. **Limit cache size:**
   ```json
   {
     "cache": {
       "max_size": 100,
       "ttl": 30
     }
   }
   ```

2. **Restart server periodically:**
   ```bash
   # Add to cron
   0 */6 * * * systemctl restart mcp-server
   ```

3. **Profile memory usage:**
   ```bash
   python -m memory_profiler mcp_server.py
   ```

---

### Rate Limiting

**Symptoms:**
```
Error: RATE-001: Rate limit exceeded
Retry after 60 seconds
```

**Solutions:**

1. **Implement backoff:**
   ```python
   import time
   import random

   def with_backoff(func, max_retries=5):
       for i in range(max_retries):
           try:
               return func()
           except RateLimitError:
               wait = (2 ** i) + random.random()
               time.sleep(wait)
       raise MaxRetriesExceeded()
   ```

2. **Use batch endpoints:**
   ```bash
   # Instead of multiple single calls
   python scripts/batch_execute.py --file operations.json
   ```

3. **Request limit increase:**
   Contact support to increase rate limits for your use case.

---

## Recovery Procedures

### Restore from Backup

**Prerequisites:**
- Backup files in `.teams/backups/`
- Valid project configuration

**Steps:**
```bash
# 1. Stop MCP server
pkill -f mcp_server

# 2. Backup current state
cp -r .teams/ .teams/emergency-backup-$(date +%Y%m%d)

# 3. Restore from backup
cp .teams/backups/project-backup-20260214.json .teams/my-project.json

# 4. Restart server
python mcp_server.py

# 5. Verify restoration
curl -X POST http://localhost:8094/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_team_list","arguments":{"project_name":"my-project"}}}'
```

---

### Reset Project State

**Warning:** This will remove all team assignments and reset to initial state.

```bash
# 1. Archive current state
mv .teams/my-project.json .teams/my-project-$(date +%Y%m%d).json.bak

# 2. Re-initialize project
curl -X POST http://localhost:8094/mcp/v1/message \
  -d '{
    "jsonrpc":"2.0",
    "method":"tools/call",
    "params":{
      "name":"guardrail_team_init",
      "arguments":{"project_name":"my-project"}
    }
  }'

# 3. Re-assign team members from backup reference
```

---

### Repair Corrupted Configuration

**Symptoms:**
```
Error: Invalid JSON in team configuration
Error: PROJ-002: Project configuration missing
```

**Steps:**
```bash
# 1. Validate JSON syntax
python -m json.tool .teams/my-project.json > /dev/null

# 2. If invalid, try to recover
python scripts/repair_config.py .teams/my-project.json

# 3. If recovery fails, restore from backup
cp .teams/backups/my-project.json .teams/my-project.json
```

---

### Emergency Rollback

Use when critical errors occur during batch operations:

```bash
#!/bin/bash
# emergency_rollback.sh

PROJECT_NAME="$1"
BACKUP_FILE=".teams/backups/${PROJECT_NAME}-pre-batch.json"

if [ ! -f "$BACKUP_FILE" ]; then
    echo "No backup found for $PROJECT_NAME"
    exit 1
fi

echo "Rolling back $PROJECT_NAME..."
cp "$BACKUP_FILE" ".teams/${PROJECT_NAME}.json"
echo "Rollback complete."

echo "Verifying..."
curl -s -X POST http://localhost:8094/mcp/v1/message \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_list\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\"}}}"
```

---

## Getting Help

### Self-Service Resources

1. **Documentation:**
   - [TEAM_TOOLS.md](./TEAM_TOOLS.md) - Tool reference
   - [AGENT_GUARDRAILS.md](./AGENT_GUARDRAILS.md) - Safety protocols
   - [TEAM_STRUCTURE.md](./TEAM_STRUCTURE.md) - Team definitions

2. **Error Code Lookup:**
   - See TEAM_TOOLS.md Error Handling section
   - Search logs for error codes

3. **Community:**
   - GitHub Issues: Report bugs and feature requests
   - Discussions: Ask questions, share solutions

### Support Channels

| Issue Type | Channel | Response Time |
|------------|---------|---------------|
| Critical outage | Email: oncall@example.com | 15 minutes |
| Security issue | security@example.com | 4 hours |
| Feature request | GitHub Issues | 2-3 days |
| General question | GitHub Discussions | 1-2 days |

### Required Information

When reporting issues, include:

1. **Error message:** Full text or screenshot
2. **Log snippets:** Relevant sections from `.mcp/mcp.log`
3. **Steps to reproduce:** Minimal example
4. **Environment:**
   ```bash
   python --version
   uname -a
   git log --oneline -1
   ```
5. **Configuration:** (sanitized)
   ```bash
   cat .mcp/config.json | grep -v password
   ```

### Diagnostic Script

Run this script to gather diagnostic information:

```bash
#!/bin/bash
# diagnose.sh - Collect diagnostic information

echo "=== Agent Guardrails Diagnostics ==="
echo "Date: $(date)"
echo ""

echo "=== System Information ==="
uname -a
echo ""

echo "=== Python Version ==="
python --version
echo ""

echo "=== MCP Server Status ==="
pgrep -f mcp_server || echo "MCP server not running"
echo ""

echo "=== Recent Log Entries ==="
tail -50 .mcp/mcp.log 2>/dev/null || echo "No log file found"
echo ""

echo "=== Configuration ==="
ls -la .teams/ 2>/dev/null || echo "No .teams directory"
ls -la .guardrails/ 2>/dev/null || echo "No .guardrails directory"
echo ""

echo "=== Health Check ==="
curl -s http://localhost:8094/mcp/v1/health 2>/dev/null || echo "Health check failed"
echo ""

echo "=== Diagnostics Complete ==="
```

---

**Last Updated:** 2026-02-15
**Version:** 1.0
