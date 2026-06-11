# Rules Index Map

Master index of all 36 prevention rules in the guardrails system.

---

## Quick Navigation

| Category | Count | Rules | Severity Range |
|----------|-------|-------|--------------|
| [Bash](#bash-rules) | 1 | BASH-001 | critical |
| [General](#general-rules) | 1 | GENERAL-001 | error |
| [Git](#git-rules) | 6 | GIT-001 to GIT-006 | error |
| [Security](#security-rules) | 28 | CODE, API, DB, CONT, CFG | critical, error, warning |

---

## Bash Rules

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **BASH-001** | critical | `(?i)(rm\s+-[a-z]*rf\|rm\s+-[a-z]*f[a-z]*r)\s*/` | Dangerous bash command detected: destructive command that could delete system files |

**Blocks:** `rm -rf /`, `rm --force --recursive /`, `rm -fr /`, fork bombs, and other destructive patterns.

---

## General Rules

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **GENERAL-001** | error | `(?i)(?:\.claude/\|\.claude$\|.*\.md$\|docs/)` | Cannot modify protected files: .claude/, docs/, or .md files |

**Blocks:** Modification of protected directories and file types.

---

## Git Rules

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **GIT-001** | error | `git\s+push\s+.*--force.*main` | Force push to main/master is blocked |
| **GIT-002** | error | `git\s+push\s+.*--force.*master` | Force push to main/master is blocked |
| **GIT-003** | error | `git\s+push\s+.*--delete.*main` | Branch deletion on main/master is blocked |
| **GIT-004** | error | `git\s+push\s+.*--delete.*master` | Branch deletion on main/master is blocked |
| **GIT-005** | error | `git\s+.*--hard\s+` | Hard reset of pushed commits is blocked |
| **GIT-006** | error | `git\s+push\s+.*--force` | Force push to protected branch |

**Blocks:** Force pushes, branch deletions, hard resets, and history rewrites on protected branches.

---

## Security Rules

### Code Security (CODE-*)

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **CODE-001** | critical | `(?i)eval\s*\([^)]*\$` | Dangerous eval() with variable detected |
| **CODE-002** | critical | `(?i)exec\s*\([^)]*\$` | Dangerous exec() with variable detected |
| **CODE-003** | error | `(?i)innerHTML\s*=` | Potential XSS via innerHTML assignment |
| **CODE-004** | error | `(?i)document\.write\s*\(` | Potential XSS via document.write |
| **CODE-005** | error | `(?i)SELECT\s+.*\s+FROM\s+.*\$` | Potential SQL injection detected |

### API Security (API-*)

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **API-001** | critical | `(?i)api[_-]?key\s*[:=]\s*["'][^"']{10,}` | API key exposure detected |
| **API-002** | critical | `(?i)secret[_-]?key\s*[:=]\s*["'][^"']{10,}` | Secret key exposure detected |
| **API-003** | critical | `(?i)api[_-]?secret\s*[:=]\s*["'][^"']{10,}` | API secret exposure detected |
| **API-004** | critical | `(?i)auth[_-]?token\s*[:=]\s*["'][^"']{10,}` | Auth token exposure detected |
| **API-005** | error | `(?i)access[_-]?token\s*[:=]\s*["'][^"']{10,}` | Access token exposure detected |
| **API-006** | error | `(?i)client[_-]?secret\s*[:=]\s*["'][^"']{10,}` | Client secret exposure detected |
| **API-007** | warning | `(?i)token\s*[:=]\s*["'][^"']{20,}` | Potential token exposure |

### Database Security (DB-*)

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **DB-001** | critical | `(?i)mongodb(\+srv)?://[^:]+:[^@]+@` | MongoDB connection string with password |
| **DB-002** | critical | `(?i)postgres(ql)?://[^:]+:[^@]+@` | PostgreSQL connection string with password |
| **DB-003** | error | `(?i)mysql://[^:]+:[^@]+@` | MySQL connection string with password |
| **DB-004** | error | `(?i)redis://:[^@]+@` | Redis connection string with password |
| **DB-005** | error | `(?i)database[_-]?url\s*[:=]\s*["'][^"']{10,}` | Database URL with credentials |

### Container Security (CONT-*)

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **CONT-001** | error | `(?i)USER\s+root` | Container running as root user |
| **CONT-002** | error | `(?i)chmod\s+777` | Overly permissive file permissions (777) |
| **CONT-003** | error | `(?i)--privileged` | Privileged container detected |
| **CONT-004** | warning | `(?i)FROM\s+[^:]*:latest` | Using 'latest' tag in FROM directive |

### Configuration Security (CFG-*)

| ID | Severity | Pattern | Message |
|----|----------|---------|---------|
| **CFG-001** | critical | `(?i)password\s*[:=]\s*["'][^"']{6,}` | Hardcoded password detected |
| **CFG-002** | critical | `(?i)private[_-]?key\s*[:=]` | Private key configuration detected |
| **CFG-003** | error | `(?i)-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----` | Private key block detected |
| **CFG-004** | error | `(?i)bearer\s+[a-zA-Z0-9]{20,}` | Bearer token detected |
| **CFG-005** | warning | `(?i)(secret|key|token|password)\s*[:=]\s*["'][^"']{4,}["']` | Generic credential pattern |

---

## Severity Summary

| Severity | Count | Rules |
|----------|-------|-------|
| **Critical** | 12 | BASH-001, CODE-001-002, API-001-004, DB-001-002, CFG-001-002 |
| **Error** | 17 | GENERAL-001, GIT-001-006, CODE-003-005, API-005-006, DB-003-005, CONT-001-003 |
| **Warning** | 5 | API-007, CONT-004, CFG-005 |
| **Info** | 2 | (Reserved for future use) |

---

## Validation Tool Mapping

| Tool | Validates Categories | Rule Count |
|------|---------------------|------------|
| `guardrail_validate_bash` | bash | 1 |
| `guardrail_validate_git_operation` | git | 6 |
| `guardrail_validate_file_edit` | file_edit, content, edit, security | 29 |

---

## Additional Identified Rules

From documentation review of 80 MD files, **200+ additional rules** were identified across categories:

| Source Document | Rules Identified | Categories |
|-----------------|-----------------|------------|
| AGENT_GUARDRAILS.md | 50+ | bash, git, security |
| Security audit docs | 26 | CODE, API, DB, CONT, CFG |
| Workflow guides | 30+ | git, file operations |
| MCP docs | 40+ | validation, tools |
| Standards docs | 60+ | coding, testing, deployment |

These are candidates for future rule additions based on organizational needs.

---

## Rule Maintenance

### Adding New Rules

1. Use SQL INSERT with unique rule_id
2. Follow naming convention: `CATEGORY-NNN`
3. Test pattern before deployment
4. Set appropriate severity

### Disabling Rules

```sql
UPDATE prevention_rules SET enabled = false WHERE rule_id = 'RULE-ID';
```

### Rule Statistics

```sql
SELECT category, severity, COUNT(*) FROM prevention_rules
WHERE enabled = true GROUP BY category, severity ORDER BY category;
```

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [MCP_TOOLS_REFERENCE.md](MCP_TOOLS_REFERENCE.md) | MCP validation tools reference |
| [RULE_PATTERNS_GUIDE.md](RULE_PATTERNS_GUIDE.md) | Pattern authoring guide |
| [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) | Main guardrails guide |
| [INDEX_MAP.md](INDEX_MAP.md) | Document navigation hub |
