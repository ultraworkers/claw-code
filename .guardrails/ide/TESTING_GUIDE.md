# Testing Guide - IDE Extensions

> Comprehensive testing documentation for Guardrail IDE extensions

**Branch:** `ide`  
**Last Updated:** 2026-02-12  
**Status:** Ready for Testing

---

## Quick Start for Testers

### Prerequisites

- VS Code 1.60+
- Node.js 16+
- Running Guardrail MCP Server (v1.12.0+)
- Git

### 5-Minute Setup

```bash
# 1. Clone repo
git clone https://github.com/TheArchitectit/agent-guardrails-template.git
cd agent-guardrails-template

# 2. Switch to ide branch
git checkout ide

# 3. Install dependencies
cd ide/vscode-extension
npm install

# 4. Compile
npm run compile

# 5. Open in VS Code
code .
```

### Launch Extension

Press `F5` in VS Code to launch Extension Development Host.

---

## Test Scenarios

### Scenario 1: Basic Connection

**Goal:** Verify extension connects to MCP server

**Steps:**
1. Open VS Code with extension loaded (F5)
2. Open any file (e.g., `test.js`)
3. Check status bar for shield icon
4. Click shield → "Configure Connection"
5. Enter server URL: `http://localhost:8095`
6. Enter API key from your server
7. Click "Test Connection"

**Expected Result:**
- Status bar shows 🛡️ "Guardrail" (green/connected)
- No error messages

**Pass Criteria:** ✅ Status bar indicates connection successful

---

### Scenario 2: Validate on Save

**Goal:** File validation triggers on save

**Steps:**
1. Create new file: `test-validation.js`
2. Add content that violates a rule (e.g., `console.log("test")`)
3. Save file (Ctrl+S)
4. Check Problems panel (Ctrl+Shift+M)

**Expected Result:**
- Diagnostics appear in Problems panel
- Error squiggles under violation
- Message shows rule ID and description

**Pass Criteria:** ✅ Violations detected and displayed

---

### Scenario 3: Validate Selection

**Goal:** Validate code selection

**Steps:**
1. Open any file
2. Select text containing potential violation
3. Right-click → "Validate Selection" (or Command Palette)
4. Check notification/message

**Expected Result:**
- Validation runs on selected text
- Notification shows result (valid or violations found)

**Pass Criteria:** ✅ Selection validation works

---

### Scenario 4: Configuration Persistence

**Goal:** Settings persist across sessions

**Steps:**
1. Configure server URL and API key
2. Close VS Code
3. Reopen VS Code
4. Check status bar

**Expected Result:**
- Settings retained
- Auto-connects to server

**Pass Criteria:** ✅ Configuration persists and auto-connects

---

### Scenario 5: Disable/Enable

**Goal:** Toggle extension on/off

**Steps:**
1. Open Settings (Ctrl+,)
2. Search "guardrail"
3. Uncheck "Enabled"
4. Check status bar
5. Re-enable

**Expected Result:**
- Disabled: Status bar shows ⭕ with tooltip "Guardrail is disabled"
- Enabled: Status bar reconnects

**Pass Criteria:** ✅ Toggle works and state reflected in UI

---

### Scenario 6: Multi-Language Support

**Goal:** Validation works for different languages

**Test Files:**
```javascript
// test.js
console.log("should warn");
```

```python
# test.py
import os
print("should warn")
```

```bash
# test.sh
rm -rf /
```

**Steps:**
1. Create each test file
2. Save each file
3. Check for language-appropriate validation

**Pass Criteria:** ✅ Each language validated correctly

---

### Scenario 7: Error Handling

**Goal:** Graceful handling of errors

**Test Cases:**

**Case A: Server offline**
1. Stop MCP server
2. Open file and save
3. Check status bar

**Expected:** Status bar shows 🔴 disconnected

**Case B: Invalid API key**
1. Set wrong API key in config
2. Test connection

**Expected:** Error message "Invalid API key"

**Case C: Network timeout**
1. Set invalid server URL
2. Test connection

**Expected:** Timeout error handled gracefully

**Pass Criteria:** ✅ All error cases handled without crashes

---

### Scenario 8: Severity Filtering

**Goal:** Respect severity threshold setting

**Steps:**
1. Open Settings
2. Set "Severity Threshold" to "error"
3. Save file with warning-level violation
4. Check if filtered

**Expected Result:**
- Warnings hidden when threshold = error
- Errors still shown

**Pass Criteria:** ✅ Threshold filtering works

---

### Scenario 9: Command Palette

**Goal:** All commands accessible

**Steps:**
1. Open Command Palette (Ctrl+Shift+P)
2. Type "guardrail"
3. Verify all commands listed:
   - Guardrail: Validate File
   - Guardrail: Validate Selection
   - Guardrail: Configure Connection
   - Guardrail: Show Output
   - Guardrail: Test Connection

**Pass Criteria:** ✅ All 5 commands visible and executable

---

### Scenario 10: Output Channel

**Goal:** Debug logging works

**Steps:**
1. Run several validations
2. Command Palette → "Guardrail: Show Output"
3. Check output panel

**Expected Content:**
- Extension activation message
- Validation requests logged
- Connection test results
- Errors logged

**Pass Criteria:** ✅ Output shows activity log

---

## Test Data

### Sample Violations

Create these files to test rule detection:

**test-secrets.js:**
```javascript
const AWS_KEY = "AKIAIOSFODNN7EXAMPLE";
```

**test-force-push.sh:**
```bash
git push origin main --force
```

**test-console.js:**
```javascript
console.log("debug message");
```

**test-scope.js:**
```javascript
// Try editing a file outside authorized scope
// This should be blocked based on project rules
```

---

## Environment Setup

### MCP Server Configuration

```bash
# Start MCP server locally
cd mcp-server
docker-compose -f deploy/podman-compose.yml up -d

# Verify running
curl http://localhost:8095/health/ready

# Get API key
cat .env | grep IDE_API_KEY
```

### VS Code Extension

```bash
# Install deps
cd ide/vscode-extension
npm install

# Compile
npm run compile

# Watch mode (auto-compile on changes)
npm run watch

# Open in VS Code
code .

# Press F5 to launch Extension Development Host
```

---

## Testing Checklist

### Pre-Flight

- [ ] MCP server running
- [ ] API key obtained
- [ ] Extension compiled
- [ ] VS Code Extension Development Host launches

### Core Functionality

- [ ] Connection establishes
- [ ] Status bar shows connected
- [ ] Validate on save works
- [ ] Validate selection works
- [ ] Diagnostics appear in Problems panel
- [ ] Error squiggles visible in editor
- [ ] Configuration UI works
- [ ] Settings persist

### Edge Cases

- [ ] Server offline handled gracefully
- [ ] Invalid API key handled
- [ ] Network timeout handled
- [ ] Empty file handled
- [ ] Very large file handled
- [ ] Binary file skipped

### Commands

- [ ] Validate File command
- [ ] Validate Selection command
- [ ] Configure Connection command
- [ ] Show Output command
- [ ] Test Connection command

### Configuration

- [ ] Server URL setting
- [ ] API key setting
- [ ] Project slug setting
- [ ] Validate on save toggle
- [ ] Validate on type toggle
- [ ] Severity threshold dropdown
- [ ] Enable/disable toggle

---

## Reporting Bugs

### Bug Report Template

```markdown
## Bug Report

**Environment:**
- VS Code Version: [e.g., 1.85.0]
- Extension Version: [e.g., 1.0.0]
- MCP Server Version: [e.g., v1.12.0]
- OS: [e.g., macOS 14.0]

**Steps to Reproduce:**
1. 
2. 
3. 

**Expected Result:**

**Actual Result:**

**Error Message (if any):**

**Screenshots:**

**Output Channel Log:**
```
[paste relevant log lines]
```

**Additional Context:**
```

### Submit

Create issue at: https://github.com/TheArchitectit/agent-guardrails-template/issues

Use label: `ide-extension` `bug`

---

## Performance Testing

### Load Test

**Goal:** Test with large files

**Steps:**
1. Create file with 10,000 lines
2. Add violations throughout
3. Save file
4. Measure time to validate

**Expected:** < 2 seconds for large files

### Stress Test

**Goal:** Rapid validation requests

**Steps:**
1. Enable "validate on type"
2. Type rapidly for 60 seconds
3. Check for:
   - Memory leaks
   - UI freezing
   - Duplicate validations

**Expected:** No degradation, no duplicates

---

## Regression Testing

Before each release, verify:

1. [ ] Fresh install works
2. [ ] Upgrade from previous version works
3. [ ] Configuration preserved
4. [ ] All commands functional
5. [ ] No console errors
6. [ ] Status bar accurate
7. [ ] Output channel works
8. [ ] Settings UI accessible

---

## Release Checklist

For maintainers before publishing:

- [ ] All P0 tests pass
- [ ] All P1 tests pass
- [ ] No known critical bugs
- [ ] Documentation updated
- [ ] CHANGELOG updated
- [ ] Version bumped
- [ ] Package.json validated
- [ ] README reviewed
- [ ] Screenshots captured

---

## FAQ

**Q: Extension doesn't activate?**
A: Check Output panel → "Guardrail" for errors

**Q: Status bar shows disconnected?**
A: Configure connection (click status bar or use command)

**Q: No diagnostics appearing?**
A: Check Problems panel (Ctrl+Shift+M), verify file type supported

**Q: Extension crashes?**
A: Check Developer Tools (Help → Toggle Developer Tools)

**Q: How to reset configuration?**
A: Command Palette → "Preferences: Open User Settings (JSON)", remove guardrail entries

---

## Resources

- **Issues:** https://github.com/TheArchitectit/agent-guardrails-template/issues
- **Plan:** `ide/IDE_EXTENSIONS_PLAN.md`
- **Source:** `ide/vscode-extension/src/`

---

**Last Updated:** 2026-02-12  
**Next Review:** On major changes
