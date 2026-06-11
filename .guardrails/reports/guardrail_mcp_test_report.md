# Guardrail MCP Server Test Report

**Test Date:** 2026-02-15  
**Test Session:** sess_4a66cdeb4d2c541bfaaf40475b00ec61  
**Total Calls Tested:** 36  
**Success Rate:** 55%

---

## Executive Summary

The Guardrail MCP Server test reveals a **partially functional system** with distinct operational zones:

| Zone | Status | Notes |
|------|--------|-------|
| **Static Validation** | ✅ Fully Operational | Scope checks, commit validation, team structure |
| **Database-Dependent** | ⚠️ Needs Schema | prevention_rules, failure_registry, file_reads tables missing |
| **Python-Dependent** | ❌ Needs Python | team_manager.py requires Python executable |

---

## Raw Test Results

### 1. Session Management

#### guardrail_init_session
**Status:** ✅ SUCCESS

```json
{
  "session_token": "sess_4a66cdeb4d2c541bfaaf40475b00ec61",
  "expires_at": "2026-02-15T19:47:55Z",
  "project_context": "",
  "active_rules_count": 0,
  "capabilities": [
    "bash_validation",
    "git_validation",
    "edit_validation"
  ]
}
```

---

### 2. Validation Calls (Bash, File, Git)

#### guardrail_validate_bash (Safe Command)
**Input:** `ls -la`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "VALIDATION-ERROR",
      "severity": "error",
      "message": "Validation engine error: failed to load rules: failed to get active rules from database: failed to get active rules: ERROR: relation \"prevention_rules\" does not exist (SQLSTATE 42P01)"
    }
  ],
  "meta": {
    "checked_at": "2026-02-15T19:32:57Z",
    "rules_evaluated": 0
  }
}
```

#### guardrail_validate_bash (Dangerous Command)
**Input:** `rm -rf /`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "VALIDATION-ERROR",
      "severity": "error",
      "message": "Validation engine error: failed to load rules: failed to get active rules from database: failed to get active rules: ERROR: relation \"prevention_rules\" does not exist (SQLSTATE 42P01)"
    }
  ],
  "meta": {
    "checked_at": "2026-02-15T19:32:58Z",
    "rules_evaluated": 0
  }
}
```

#### guardrail_validate_file_edit
**Input:** Edit `/tmp/test.txt`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "VALIDATION-ERROR",
      "severity": "error",
      "message": "Validation engine error: failed to load rules: failed to get active rules from database: failed to get active rules: ERROR: relation \"prevention_rules\" does not exist (SQLSTATE 42P01)"
    }
  ],
  "meta": {
    "checked_at": "2026-02-15T19:32:58Z",
    "rules_evaluated": 0
  }
}
```

#### guardrail_validate_git_operation (commit)
**Input:** `commit`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "VALIDATION-ERROR",
      "severity": "error",
      "message": "Validation engine error: failed to load rules: failed to get active rules from database: failed to get active rules: ERROR: relation \"prevention_rules\" does not exist (SQLSTATE 42P01)"
    }
  ],
  "meta": {
    "checked_at": "2026-02-15T19:32:59Z",
    "rules_evaluated": 0
  }
}
```

#### guardrail_validate_git_operation (push --force)
**Input:** `push --force`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "VALIDATION-ERROR",
      "severity": "error",
      "message": "Validation engine error: failed to load rules: failed to get active rules from database: failed to get active rules: ERROR: relation \"prevention_rules\" does not exist (SQLSTATE 42P01)"
    }
  ],
  "meta": {
    "checked_at": "2026-02-15T19:32:59Z",
    "rules_evaluated": 0
  }
}
```

---

### 3. Context & Scope Validation

#### guardrail_get_context
**Status:** ✅ SUCCESS

**Response:**
```
# Default Guardrails

No project-specific context found for test-guardrail-mcp
```

#### guardrail_validate_scope (Within Scope)
**Input:** Scope `/app/src`, File `/app/src/components/Button.tsx`  
**Status:** ✅ SUCCESS

```json
{
  "valid": true,
  "message": "File /app/src/components/Button.tsx is within authorized scope",
  "file_path": "/app/src/components/Button.tsx",
  "scope": "/app/src"
}
```

#### guardrail_validate_scope (Outside Scope)
**Input:** Scope `/app/src`, File `/etc/passwd`  
**Status:** ✅ SUCCESS (Validation Failed as Expected)

```json
{
  "valid": false,
  "message": "File /etc/passwd is OUTSIDE authorized scope /app/src",
  "file_path": "/etc/passwd",
  "scope": "/app/src",
  "outside_scope": true
}
```

---

### 4. Commit & Push Validation

#### guardrail_validate_commit (Valid)
**Input:** `feat: add new feature`  
**Status:** ✅ SUCCESS

```json
{
  "valid": true,
  "format_compliant": true,
  "message": "feat: add new feature",
  "conventional_type": "feat"
}
```

#### guardrail_validate_commit (Invalid)
**Input:** `bad commit message`  
**Status:** ✅ SUCCESS (Validation Failed as Expected)

```json
{
  "valid": false,
  "format_compliant": false,
  "issues": [
    "Message does not follow conventional commit format: type(scope): description"
  ],
  "message": "bad commit message"
}
```

#### guardrail_check_test_prod_separation
**Input:** Test environment, File `/tmp/test.config.js`  
**Status:** ✅ SUCCESS

```json
{
  "valid": true,
  "file_path": "/tmp/test.config.js",
  "environment": "test"
}
```

#### guardrail_validate_push
**Input:** Branch `main`, has unpushed commits, not force  
**Status:** ✅ SUCCESS

```json
{
  "valid": true,
  "can_push": true,
  "warnings": [
    "Pushing directly to 'main' branch - consider using a pull request"
  ],
  "branch": "main",
  "is_force": false
}
```

#### guardrail_prevent_regression
**Input:** Code snippet and file paths  
**Status:** ⚠️ DATABASE ERROR

```
Failed to check failures: failed to get active failures: ERROR: relation "failure_registry" does not exist (SQLSTATE 42P01)
```

---

### 5. File Read Tracking

#### guardrail_record_file_read
**Input:** File `/tmp/test.txt`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "success": false,
  "error": "Failed to record file read: failed to create file read record: ERROR: relation \"file_reads\" does not exist (SQLSTATE 42P01)"
}
```

#### guardrail_verify_file_read
**Input:** File `/tmp/test.txt`  
**Status:** ✅ SUCCESS

```json
{
  "valid": true,
  "was_read": false,
  "message": "File has not been read",
  "session_id": "sess_4a66cdeb4d2c541bfaaf40475b00ec61",
  "file_path": "/tmp/test.txt"
}
```

---

### 6. Attempt Tracking & Three Strikes

#### guardrail_record_attempt
**Input:** Error category `syntax`, Task `test-task-1`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "error": "Failed to record attempt: failed to get attempt count: failed to count attempts: ERROR: relation \"task_attempts\" does not exist (SQLSTATE 42P01)"
}
```

#### guardrail_validate_three_strikes
**Input:** Task `test-task-1`  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "error": "Failed to check status: failed to count attempts: ERROR: relation \"task_attempts\" does not exist (SQLSTATE 42P01)"
}
```

#### guardrail_reset_attempts
**Input:** Task `test-task-1`  
**Status:** ❌ HTTP 500 INTERNAL SERVER ERROR

```json
{
  "message": "Internal Server Error"
}
```

---

### 7. Uncertainty & Halt Conditions

#### guardrail_check_uncertainty
**Input:** Task "Testing guardrail MCP calls", Assessment "low"  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "error": "Failed to save uncertainty record: failed to save uncertainty record: ERROR: relation \"uncertainty_tracking\" does not exist (SQLSTATE 42P01)"
}
```

#### guardrail_check_halt_conditions
**Status:** ✅ SUCCESS

```json
{
  "halt": false,
  "reasons": [],
  "severity": "none",
  "action": "Continue",
  "message": "No halt conditions detected"
}
```

#### guardrail_record_halt
**Input:** Type `code_safety`, Severity `medium`  
**Status:** ❌ HTTP 500 INTERNAL SERVER ERROR

```json
{
  "message": "Internal Server Error"
}
```

#### guardrail_acknowledge_halt
**Input:** Halt ID `test-halt-123`, Resolution `resolved`  
**Status:** ⚠️ VALIDATION ERROR

```json
{
  "success": false,
  "error": "Invalid halt_id format"
}
```

---

### 8. Production & Feature Creep

#### guardrail_validate_production_first
**Input:** Test code with production dependency  
**Status:** ⚠️ DATABASE ERROR

```json
{
  "valid": false,
  "message": "Failed to record code: failed to create production code record: ERROR: relation \"production_code_tracking\" does not exist (SQLSTATE 42P01)",
  "production_code_exists": false
}
```

#### guardrail_detect_feature_creep
**Input:** New button component, +1/-0 lines  
**Status:** ✅ SUCCESS

```json
{
  "creep_detected": false,
  "diff_summary": "+1/-0 lines",
  "total_changes": {
    "additions": 1,
    "deletions": 0
  },
  "recommendation": "No feature creep detected - clear to proceed"
}
```

#### guardrail_verify_fixes_intact
**Input:** File `/tmp/test.js`  
**Status:** ⚠️ DATABASE ERROR

```
Failed to check failures: failed to get active failures: ERROR: relation "failure_registry" does not exist (SQLSTATE 42P01)
```

#### guardrail_pre_work_check
**Input:** Affected files `["/tmp/test.js"]`  
**Status:** ⚠️ DATABASE ERROR

```
Failed to check failures: failed to get active failures: ERROR: relation "failure_registry" does not exist (SQLSTATE 42P01)
```

#### guardrail_validate_exact_replacement
**Input:** Original `const x = 0;`, Modified `const x = 1;`  
**Status:** ✅ SUCCESS

```json
{
  "exact_match": true,
  "diff_stats": {
    "additions": 1,
    "deletions": 1
  },
  "recommendation": "Accept changes - exact match confirmed"
}
```

---

### 9. Team Management

#### guardrail_team_init
**Input:** Project `test-project-mcp`  
**Status:** ✅ SUCCESS

**Response:**
```
✅ Initialized project 'test-project-mcp' with 12 teams
```

#### guardrail_team_list
**Input:** Project `test-project-mcp`  
**Status:** ✅ SUCCESS

**Response:**
```
📋 Teams for project 'test-project-mcp':

ID    Name                                Phase                          Status
----------------------------------------------------------------------------------------------------
6     Data Governance & Analytics         Phase 2: Platform & Foundation not_started
2     Enterprise Architecture             Phase 1: Strategy, Governance & Planning not_started
4     Infrastructure & Cloud Ops          Phase 2: Platform & Foundation not_started
12    IT Operations & Support (NOC)       Phase 5: Delivery & Sustainment not_started
1     Business & Product Strategy         Phase 1: Strategy, Governance & Planning not_started
3     GRC (Governance, Risk, & Compliance) Phase 1: Strategy, Governance & Planning not_started
9     Cybersecurity (AppSec)              Phase 4: Validation & Hardening not_started
7     Core Feature Squad                  Phase 3: The Build Squads      not_started
10    Quality Engineering (SDET)          Phase 4: Validation & Hardening not_started
8     Middleware & Integration            Phase 3: The Build Squads      not_started
11    Site Reliability Engineering (SRE)  Phase 5: Delivery & Sustainment not_started
5     Platform Engineering                Phase 2: Platform & Foundation not_started
```

#### guardrail_team_assign
**Input:** Person `Alice`, Role `planner`, Team 1  
**Status:** ⚠️ VALIDATION ERROR

**Response:**
```
Error: invalid role_name: 'planner'. Must be one of the 48 defined roles in TEAM_STRUCTURE.md
```

#### guardrail_agent_team_map
**Input:** Agent type `backend`  
**Status:** ✅ SUCCESS

**Response:**
```
# Agent Team Assignment

**Agent Type:** backend
**Assigned Team:** Team 7
**Phase:** Phase 3
**Roles:** Senior Backend Engineer
```

#### guardrail_team_size_validate
**Input:** Project `test-project-mcp`, Team 1  
**Status:** ✅ SUCCESS

**Response:** *(Empty response - validation passed)*

#### guardrail_team_status
**Input:** Project `test-project-mcp`  
**Status:** ✅ SUCCESS

```json
{
  "active": 0,
  "completed": 0,
  "not_started": 12,
  "progress_pct": 0,
  "project": "test-project-mcp",
  "total_teams": 12
}
```

#### guardrail_phase_gate_check
**Input:** Phase 1 → 2  
**Status:** ✅ SUCCESS

**Response:**
```
# Phase Gate: Architecture Review Board

**Required Teams:**
- Team 1
- Team 2
- Team 3

**Required Deliverables:**
- [ ] Architecture Decision Records
- [ ] Approved Tech List
- [ ] Compliance Checklist
```

---

### 10. Team Lifecycle

#### guardrail_team_start
**Input:** Team 1  
**Status:** ✅ SUCCESS

**Response:**
```
✅ Started Team 1 (Business & Product Strategy)
```

#### guardrail_team_health
**Input:** Project `test-project-mcp`  
**Status:** ❌ PYTHON ERROR

**Response:**
```
Health check failed: exec: "python": executable file not found in $PATH
Output:
```

#### guardrail_team_unassign
**Input:** Role `Senior Backend Engineer`, Team 7  
**Status:** ⚠️ LOGIC ERROR

**Response:**
```
Error unassigning role: role 'Senior Backend Engineer' in Core Feature Squad is already unassigned
```

#### guardrail_team_delete
**Input:** Team 2  
**Status:** ❌ PYTHON ERROR

**Response:**
```
Error deleting team: exec: "python": executable file not found in $PATH
Output:
```

#### guardrail_project_delete
**Input:** Project `test-project-mcp`  
**Status:** ❌ PYTHON ERROR

**Response:**
```
Error deleting project: exec: "python": executable file not found in $PATH
Output:
```

---

## Summary Statistics

### Overall Results

| Category | Total | Success | Error | Success Rate |
|----------|-------|---------|-------|--------------|
| **Session** | 1 | 1 | 0 | 100% |
| **Validation** | 5 | 0 | 5 | 0% |
| **Context/Scope** | 3 | 3 | 0 | 100% |
| **Commit/Push** | 5 | 4 | 1 | 80% |
| **File Tracking** | 2 | 1 | 1 | 50% |
| **Attempt Tracking** | 3 | 0 | 3 | 0% |
| **Uncertainty/Halt** | 4 | 1 | 3 | 25% |
| **Production/Creep** | 5 | 2 | 3 | 40% |
| **Team Management** | 7 | 6 | 1 | 86% |
| **Team Lifecycle** | 5 | 1 | 4 | 20% |
| **TOTAL** | **40** | **22** | **18** | **55%** |

---

## Error Analysis

### Database Schema Errors (PostgreSQL)

**Affected Tables Missing:**
1. `prevention_rules` - 5 occurrences
2. `failure_registry` - 4 occurrences
3. `file_reads` - 1 occurrence
4. `task_attempts` - 3 occurrences
5. `uncertainty_tracking` - 1 occurrence
6. `production_code_tracking` - 1 occurrence

**SQL Error Pattern:**
```
ERROR: relation "{table_name}" does not exist (SQLSTATE 42P01)
```

**Root Cause:** PostgreSQL database not initialized with required schema.

**Resolution:** Run database migrations:
```bash
# Assuming migration files exist
psql -h $DB_HOST -U $DB_USER -d $DB_NAME < schema.sql
```

---

### Python Environment Errors

**Affected Calls:**
- `guardrail_team_health`
- `guardrail_team_delete`
- `guardrail_project_delete`

**Error Pattern:**
```
exec: "python": executable file not found in $PATH
```

**Root Cause:** team_manager.py requires Python executable not available in container/ENV.

**Resolution:** Install Python or provide path to Python executable:
```bash
# Dockerfile addition
RUN apt-get update && apt-get install -y python3 python3-pip
# OR
ENV PATH="/usr/bin/python3:$PATH"
```

---

### HTTP 500 Internal Server Errors

**Affected Calls:**
- `guardrail_reset_attempts`
- `guardrail_record_halt`

**Root Cause:** Server-side panic or unhandled exception.

**Resolution:** Check server logs for stack traces.

---

### Validation Errors (Expected Behavior)

**Affected Calls:**
- `guardrail_team_assign` - Invalid role name
- `guardrail_acknowledge_halt` - Invalid halt_id format
- `guardrail_team_unassign` - Role already unassigned

**Status:** These are **correct behavior** - validation is working as designed.

---

## Functional Capabilities

### Fully Working (No Dependencies)

| Capability | Calls |
|------------|-------|
| **Scope Validation** | `guardrail_validate_scope` |
| **Commit Validation** | `guardrail_validate_commit` |
| **Push Validation** | `guardrail_validate_push` |
| **Test/Prod Separation Check** | `guardrail_check_test_prod_separation` |
| **Feature Creep Detection** | `guardrail_detect_feature_creep` |
| **Exact Replacement Validation** | `guardrail_validate_exact_replacement` |
| **Halt Condition Check** | `guardrail_check_halt_conditions` |
| **Team Structure Management** | `guardrail_team_init`, `guardrail_team_list`, `guardrail_team_status`, `guardrail_phase_gate_check`, `guardrail_agent_team_map`, `guardrail_team_start` |

### Requires Database Setup

| Capability | Calls | Missing Tables |
|------------|-------|----------------|
| **Command Validation** | `guardrail_validate_bash` | prevention_rules |
| **File Edit Validation** | `guardrail_validate_file_edit` | prevention_rules |
| **Git Operation Validation** | `guardrail_validate_git_operation` | prevention_rules |
| **Regression Prevention** | `guardrail_prevent_regression` | failure_registry |
| **File Read Tracking** | `guardrail_record_file_read` | file_reads |
| **Attempt Tracking** | `guardrail_record_attempt`, `guardrail_validate_three_strikes` | task_attempts |
| **Attempt Reset** | `guardrail_reset_attempts` | task_attempts |
| **Uncertainty Tracking** | `guardrail_check_uncertainty` | uncertainty_tracking |
| **Halt Recording** | `guardrail_record_halt` | uncertainty_tracking |
| **Production First Validation** | `guardrail_validate_production_first` | production_code_tracking |
| **Fix Verification** | `guardrail_verify_fixes_intact` | failure_registry |
| **Pre-Work Check** | `guardrail_pre_work_check` | failure_registry |

### Requires Python Environment

| Capability | Calls |
|------------|-------|
| **Team Health Check** | `guardrail_team_health` |
| **Team Deletion** | `guardrail_team_delete` |
| **Project Deletion** | `guardrail_project_delete` |

---

## Recommendations

### Immediate Actions

1. **Initialize Database Schema**
   ```bash
   # Run migrations to create required tables
   cd mcp-server
   ./scripts/init-db.sh
   # OR
   psql -h localhost -U guardrail_user -d guardrail_mcp -f migrations/001_initial_schema.sql
   ```

2. **Install Python Runtime**
   ```bash
   # Dockerfile update
   RUN apt-get update && apt-get install -y python3
   # Verify
   python3 --version
   ```

3. **Document 48 Valid Roles**
   Add reference documentation for all valid role names used in `guardrail_team_assign`.

### Long-Term Improvements

1. **Graceful Degradation**
   - When database is unavailable, fall back to static rule validation
   - Return warnings instead of errors for non-critical failures

2. **Health Check Endpoint**
   - Add `/health/db` endpoint to verify database connectivity
   - Add `/health/python` endpoint to verify Python availability

3. **Auto-Schema Initialization**
   - On server startup, check if schema exists
   - Auto-run migrations if tables are missing

4. **Better Error Messages**
   - Distinguish between "table missing" and "validation failed"
   - Provide actionable error messages with fix commands

---

## Appendix: Complete Error Log

```
=== DATABASE ERRORS ===

[1] prevention_rules table missing
   Calls: guardrail_validate_bash (x2), guardrail_validate_file_edit, guardrail_validate_git_operation (x2)
   Error: ERROR: relation "prevention_rules" does not exist (SQLSTATE 42P01)

[2] failure_registry table missing
   Calls: guardrail_prevent_regression, guardrail_verify_fixes_intact, guardrail_pre_work_check
   Error: ERROR: relation "failure_registry" does not exist (SQLSTATE 42P01)

[3] file_reads table missing
   Calls: guardrail_record_file_read
   Error: ERROR: relation "file_reads" does not exist (SQLSTATE 42P01)

[4] task_attempts table missing
   Calls: guardrail_record_attempt, guardrail_validate_three_strikes
   Error: ERROR: relation "task_attempts" does not exist (SQLSTATE 42P01)

[5] uncertainty_tracking table missing
   Calls: guardrail_check_uncertainty
   Error: ERROR: relation "uncertainty_tracking" does not exist (SQLSTATE 42P01)

[6] production_code_tracking table missing
   Calls: guardrail_validate_production_first
   Error: ERROR: relation "production_code_tracking" does not exist (SQLSTATE 42P01)

=== PYTHON ERRORS ===

[1] Python not found in PATH
   Calls: guardrail_team_health, guardrail_team_delete, guardrail_project_delete
   Error: exec: "python": executable file not found in $PATH

=== HTTP 500 ERRORS ===

[1] Internal Server Error
   Calls: guardrail_reset_attempts, guardrail_record_halt
   Error: HTTP 500: {"message":"Internal Server Error"}

=== VALIDATION ERRORS (Expected) ===

[1] Invalid role name
   Call: guardrail_team_assign
   Error: invalid role_name: 'planner'. Must be one of the 48 defined roles

[2] Invalid halt_id format
   Call: guardrail_acknowledge_halt
   Error: Invalid halt_id format

[3] Role already unassigned
   Call: guardrail_team_unassign
   Error: role 'Senior Backend Engineer' in Core Feature Squad is already unassigned
```

---

**Report Generated:** 2026-02-15  
**Tested By:** Automated MCP Test Suite  
**Server Version:** guardrail_mcp v1.12.0
