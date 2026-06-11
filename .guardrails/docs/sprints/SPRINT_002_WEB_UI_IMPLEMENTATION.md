# Sprint: Web UI Implementation - Management Interface

**Sprint Date:** 2026-02-08 (Saturday)
**Archive After:** 2026-02-15 (Saturday) [+7 days]
**Sprint Focus:** Implement complete Web UI SPA for guardrail management with 26 API endpoints
**Priority:** P1 (High)
**Estimated Effort:** 6-8 hours
**Status:** COMPLETED
**Completed Date:** 2026-02-08
**Actual Effort:** 6 hours
**Coverage:** 25/26 API endpoints (96%), All 6 pages (100%)

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Execution Safety Checks

| Check | Requirement | Verify |
|-------|-------------|--------|
| **READ FIRST** | NEVER edit a file without reading it first | [ ] |
| **SCOPE LOCK** | Only modify files explicitly in scope | [ ] |
| **NO FEATURE CREEP** | Do NOT add features or "improve" unrelated code | [ ] |
| **PRODUCTION FIRST** | Production code created BEFORE test code | [ ] |
| **TEST/PROD SEPARATION** | Test infrastructure is separate from production | [ ] |
| **ASK IF UNCERTAIN** | If test/production boundary unclear, ask user | [ ] |
| **BACKUP AWARENESS** | Know the rollback command before editing | [ ] |
| **TEST BEFORE COMMIT** | All tests must pass before committing | [ ]

### Guardrails Reference

Full guardrails: [docs/AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## PROBLEM STATEMENT

The MCP server provides a comprehensive REST API (26+ endpoints) for managing guardrails, but there is no web interface for human administrators to:

1. Browse and search documentation
2. Manage prevention rules (CRUD + toggle)
3. View and update the failure registry
4. Manage projects and their contexts
5. Monitor system stats and health
6. Validate code through the IDE Tools API

**Root Cause:** Initial implementation focused on API and MCP protocol only, leaving a gap for human-friendly management.

**Where:** New directory `mcp-server/web/` - completely isolated from existing code

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - Directory: mcp-server/web/ (CREATE)
    Contents: Complete SPA implementation
    - index.html (main entry)
    - css/ (design system)
    - js/ (API client + components + pages)
    - assets/ (icons, fonts)

  - File: mcp-server/cmd/server/main.go
    Lines: Add static file serving route
    Change: Add web UI static file handler

  - File: mcp-server/internal/web/routes.go (CREATE)
    Lines: All
    Change: Web UI route registration

OUT OF SCOPE (DO NOT TOUCH):
  - API endpoint implementations (internal/api/)
  - Database schema or queries
  - MCP protocol handlers
  - Authentication/authorization logic
  - Web UI API key requirements (use existing)
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: Create project structure
          - mcp-server/web/ directory
          - Subdirectories: css/, js/, js/components/, js/pages/
          - - - - - - - - - - - - - - - - - - > Setup project skeleton
       |
       v
  STEP 2: Create API client module
          - js/api.js with all 26 endpoints
          - Error handling, auth headers, response parsing
          - - - - - - - - - - - - - - - - - - > Enable API communication
       |
       v
  STEP 3: Create design system (CSS)
          - variables.css (colors, typography, spacing)
          - components.css (buttons, cards, forms, tables)
          - layout.css (grid, navigation, responsive)
          - - - - - - - - - - - - - - - - - - > Consistent visual design
       |
       v
  STEP 4: Create reusable components
          - Navigation component
          - DataTable component (sortable, paginated)
          - Form components (input, select, textarea)
          - Modal/Dialog component
          - Toast/Notification component
          - - - - - - - - - - - - - - - - - - > Shared UI building blocks
       |
       v
  STEP 5: Implement pages
          - Dashboard (stats overview)
          - Documents (list, view, edit, search)
          - Rules (list, create, edit, toggle, delete)
          - Projects (list, create, edit, delete)
          - Failures (list, view, update status)
          - IDE Tools (validation interface)
          - - - - - - - - - - - - - - - - - - > Complete management UI
       |
       v
  STEP 6: Wire up routing
          - Hash-based SPA routing
          - Navigation state management
          - - - - - - - - - - - - - - - - - - > Page navigation
       |
       v
  STEP 7: Register web routes in server
          - Add static file serving
          - Configure web UI path
          - - - - - - - - - - - - - - - - - - > Integrate with server
       |
       v
  STEP 8: Build and verify
          - Run server, test UI
          - Check all pages load
          - Verify API calls work
          - - - - - - - - - - - - - - - - - - > Validate implementation
       |
       v
  DONE: Commit and report - - - - - - - - > Summary to user
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: Create Project Structure

**Action:** Create the Web UI directory structure

```bash
mkdir -p mcp-server/web/css
mkdir -p mcp-server/web/js/components
mkdir -p mcp-server/web/js/pages
mkdir -p mcp-server/web/assets
```

**Files to Create:**
- `mcp-server/web/index.html` (SPA entry point)
- `mcp-server/web/css/variables.css`
- `mcp-server/web/css/components.css`
- `mcp-server/web/css/layout.css`
- `mcp-server/web/js/api.js`
- `mcp-server/web/js/components/Navigation.js`
- `mcp-server/web/js/components/DataTable.js`
- `mcp-server/web/js/components/Forms.js`
- `mcp-server/web/js/components/Modal.js`
- `mcp-server/web/js/components/Toast.js`
- `mcp-server/web/js/pages/Dashboard.js`
- `mcp-server/web/js/pages/Documents.js`
- `mcp-server/web/js/pages/Rules.js`
- `mcp-server/web/js/pages/Projects.js`
- `mcp-server/web/js/pages/Failures.js`
- `mcp-server/web/js/pages/IDETools.js`
- `mcp-server/web/js/app.js` (main entry)
- `mcp-server/web/js/router.js` (SPA routing)

**Checkpoint:**
- All directories created
- File list matches plan

**Decision Point:**
- [ ] Success → Proceed to STEP 2
- [ ] Failure → HALT and report

---

### STEP 2: Create API Client Module

**Action:** Create `mcp-server/web/js/api.js` with all 26 API endpoints

**Endpoints to Implement:**

**Health:**
- `GET /health/live`
- `GET /health/ready`
- `GET /version`
- `GET /api/stats`

**Documents (6 endpoints):**
- `GET /api/documents` (list)
- `GET /api/documents/:id` (get)
- `PUT /api/documents/:id` (update)
- `GET /api/documents/search` (search)

**Rules (6 endpoints):**
- `GET /api/rules` (list)
- `GET /api/rules/:id` (get)
- `POST /api/rules` (create)
- `PUT /api/rules/:id` (update)
- `DELETE /api/rules/:id` (delete)
- `PATCH /api/rules/:id` (toggle)

**Projects (5 endpoints):**
- `GET /api/projects` (list)
- `GET /api/projects/:id` (get)
- `POST /api/projects` (create)
- `PUT /api/projects/:id` (update)
- `DELETE /api/projects/:id` (delete)

**Failures (3 endpoints):**
- `GET /api/failures` (list)
- `GET /api/failures/:id` (get)
- `PUT /api/failures/:id` (update status)

**IDE Tools (4 endpoints):**
- `GET /ide/health`
- `GET /ide/rules`
- `POST /ide/validate/file`
- `POST /ide/validate/selection`
- `GET /ide/quick-reference`

**Implementation Pattern:**
```javascript
class GuardrailAPI {
  constructor(baseURL = '') {
    this.baseURL = baseURL;
    this.apiKey = localStorage.getItem('api_key') || '';
  }

  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const headers = {
      'Content-Type': 'application/json',
      ...(this.apiKey && { 'Authorization': `Bearer ${this.apiKey}` }),
      ...options.headers
    };

    const response = await fetch(url, { ...options, headers });
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(error.error || `HTTP ${response.status}`);
    }
    return response.status === 204 ? null : await response.json();
  }

  // Documents
  async listDocuments(params = {}) { /* ... */ }
  async getDocument(id) { /* ... */ }
  async updateDocument(id, data) { /* ... */ }
  async searchDocuments(query) { /* ... */ }

  // Rules
  async listRules(params = {}) { /* ... */ }
  async getRule(id) { /* ... */ }
  async createRule(data) { /* ... */ }
  async updateRule(id, data) { /* ... */ }
  async deleteRule(id) { /* ... */ }
  async toggleRule(id, enabled) { /* ... */ }

  // Projects, Failures, IDE Tools...
}

window.api = new GuardrailAPI();
```

**Decision Point:**
- [ ] Success → Proceed to STEP 3
- [ ] Failure → ROLLBACK and report

---

### STEP 3: Create Design System (CSS)

**Action:** Create CSS files for consistent design

**`css/variables.css`:**
```css
:root {
  /* Colors */
  --color-primary: #3b82f6;
  --color-primary-dark: #2563eb;
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-background: #0f172a;
  --color-surface: #1e293b;
  --color-surface-hover: #334155;
  --color-border: #334155;
  --color-text: #f1f5f9;
  --color-text-muted: #94a3b8;

  /* Typography */
  --font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --font-mono: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;

  /* Spacing */
  --spacing-xs: 0.25rem;
  --spacing-sm: 0.5rem;
  --spacing-md: 1rem;
  --spacing-lg: 1.5rem;
  --spacing-xl: 2rem;

  /* Border Radius */
  --radius-sm: 0.25rem;
  --radius-md: 0.5rem;
  --radius-lg: 0.75rem;
}
```

**`css/components.css`:** Buttons, cards, forms, tables, badges

**`css/layout.css`:** Navigation, grid system, responsive breakpoints

**Decision Point:**
- [ ] Success → Proceed to STEP 4
- [ ] Failure → ROLLBACK and report

---

### STEP 4: Create Reusable Components

**Action:** Create shared UI components

**`js/components/Navigation.js`:**
- Sidebar navigation with icons
- Active state highlighting
- Collapsible on mobile

**`js/components/DataTable.js`:**
- Sortable columns
- Pagination
- Row actions (edit, delete)
- Empty state

**`js/components/Forms.js`:**
- Input with validation
- Select dropdown
- Textarea with markdown preview
- Checkbox/Switch

**`js/components/Modal.js`:**
- Create/Edit modals
- Confirmation dialogs
- Form submission handling

**`js/components/Toast.js`:**
- Success/error notifications
- Auto-dismiss
- Stacking support

**Decision Point:**
- [ ] Success → Proceed to STEP 5
- [ ] Failure → ROLLBACK and report

---

### STEP 5: Implement Pages

**Action:** Create page components

**`js/pages/Dashboard.js`:**
- System stats cards (documents, rules, projects, failures)
- Quick actions
- Recent activity (if available)

**`js/pages/Documents.js`:**
- List view with DataTable
- Search bar
- Category filter
- View/Edit modal with markdown preview

**`js/pages/Rules.js`:**
- List with toggle switches
- Create/Edit modal
- Pattern preview
- Enable/disable toggle
- Delete confirmation

**`js/pages/Projects.js`:**
- Project cards or table
- Create/Edit with context editor
- Active rules selection
- Delete with confirmation

**`js/pages/Failures.js`:**
- Status filter (active/resolved/deprecated)
- Severity badges
- Update status action
- View details modal

**`js/pages/IDETools.js`:**
- Code validation textarea
- Language selector
- Validation results display
- Quick reference sidebar

**Decision Point:**
- [ ] Success → Proceed to STEP 6
- [ ] Failure → ROLLBACK and report

---

### STEP 6: Wire Up Routing

**Action:** Create `js/router.js` for SPA navigation

```javascript
class Router {
  constructor() {
    this.routes = {
      '/': 'Dashboard',
      '/documents': 'Documents',
      '/rules': 'Rules',
      '/projects': 'Projects',
      '/failures': 'Failures',
      '/ide-tools': 'IDETools'
    };
    this.currentPage = null;
    this.init();
  }

  init() {
    window.addEventListener('hashchange', () => this.handleRoute());
    this.handleRoute();
  }

  handleRoute() {
    const hash = window.location.hash.slice(1) || '/';
    const pageName = this.routes[hash] || 'Dashboard';
    this.loadPage(pageName);
  }

  loadPage(pageName) {
    const app = document.getElementById('app');
    const PageClass = window[pageName];
    if (PageClass) {
      this.currentPage = new PageClass(app);
    }
  }

  navigate(path) {
    window.location.hash = path;
  }
}

window.router = new Router();
```

**`js/app.js`:**
- Initialize app
- API key input (if not set)
- Render navigation
- Handle global errors

**Decision Point:**
- [ ] Success → Proceed to STEP 7
- [ ] Failure → ROLLBACK and report

---

### STEP 7: Register Web Routes in Server

**Action:** Add static file serving to server

**Read:** `mcp-server/cmd/server/main.go`

**Add to main.go:**
```go
// Add near route registration
e.Static("/web", "web")
e.File("/web/*", "web/index.html")
```

**Or create `internal/web/routes.go`:**
```go
package web

import (
    "github.com/labstack/echo/v4"
)

// RegisterRoutes registers web UI routes
func RegisterRoutes(e *echo.Echo) {
    // Static files
    e.Static("/web", "web")

    // SPA fallback - serve index.html for all /web/* routes
    e.GET("/web/*", func(c echo.Context) error {
        return c.File("web/index.html")
    })
}
```

**Decision Point:**
- [ ] Success → Proceed to STEP 8
- [ ] Failure → ROLLBACK and report

---

### STEP 8: Build and Verify

**Action:** Build server and test Web UI

```bash
# Build the server
cd mcp-server
go build ./cmd/server

# Run tests
go test ./...

# Check web files are in place
ls -la web/
ls -la web/js/
```

**Manual Verification:**
1. Start server: `./server`
2. Open browser: `http://localhost:8080/web`
3. Verify:
   - [ ] Navigation loads
   - [ ] Dashboard shows stats
   - [ ] Documents page loads
   - [ ] Rules page loads
   - [ ] Projects page loads
   - [ ] Failures page loads
   - [ ] IDE Tools page loads

**Expected Output:**
- Build succeeds
- All pages render
- API calls return data
- No console errors

**Decision Point:**
- [ ] Success → Proceed to DONE
- [ ] Failure → Fix issues and re-run

---

### DONE: Commit and Report

**Action:** Provide completion summary

```bash
# Stage all changes
git add mcp-server/web/
git add mcp-server/cmd/server/main.go
git add mcp-server/internal/web/

# Commit
git commit -m "feat(web-ui): implement complete management interface

- Add SPA with 6 management pages:
  - Dashboard with system stats
  - Documents browser (CRUD + search)
  - Rules management (CRUD + toggle)
  - Projects management (CRUD)
  - Failure registry viewer
  - IDE Tools validation interface

- Implement 26 API endpoints in JavaScript client
- Create reusable UI components (Navigation, DataTable, Forms, Modal, Toast)
- Add complete CSS design system
- Register web routes in server

Authored by TheArchitectit"
```

**REPORT FORMAT:**

## Sprint Complete: Web UI Implementation

**Status:** SUCCESS
**Files Created:**
- mcp-server/web/index.html
- mcp-server/web/css/variables.css
- mcp-server/web/css/components.css
- mcp-server/web/css/layout.css
- mcp-server/web/js/api.js
- mcp-server/web/js/app.js
- mcp-server/web/js/router.js
- mcp-server/web/js/components/Navigation.js
- mcp-server/web/js/components/DataTable.js
- mcp-server/web/js/components/Forms.js
- mcp-server/web/js/components/Modal.js
- mcp-server/web/js/components/Toast.js
- mcp-server/web/js/pages/Dashboard.js
- mcp-server/web/js/pages/Documents.js
- mcp-server/web/js/pages/Rules.js
- mcp-server/web/js/pages/Projects.js
- mcp-server/web/js/pages/Failures.js
- mcp-server/web/js/pages/IDETools.js
- mcp-server/internal/web/routes.go (NEW)

**Files Modified:**
- mcp-server/cmd/server/main.go

**Commit Hash:** [hash]

### Changes Made:
- Implemented complete SPA for guardrail management
- Created JavaScript API client for all 26 endpoints
- Built reusable component library
- Designed dark-themed UI matching system aesthetic
- Integrated with existing API authentication

### Verification Results:
- Syntax check: PASSED
- Unit tests: PASSED
- Build verification: PASSED
- Manual UI testing: PASSED

### Next Steps:
- Deploy updated server
- Test with real data
- Gather user feedback

---

## COMPLETION GATE (MANDATORY)

**This section MUST be completed before marking the sprint done.**

### Validation Loop Rules

```
MAX_CYCLES: 3
MAX_TIME: 30 minutes
EXIT_CONDITIONS:
  - All BLOCKING items pass, OR
  - MAX_CYCLES reached (report blockers), OR
  - MAX_TIME exceeded (report status)
```

### Core Validation Checklist

| Check | Command | Pass Condition | Blocking? | Status |
|-------|---------|----------------|-----------|--------|
| **Files Saved** | `git status` | No unexpected untracked files | YES | [ ] |
| **Changes Staged** | `git diff --cached --stat` | Target files staged | YES | [ ] |
| **Syntax Valid** | `go build ./cmd/server` | Exit code 0 | YES | [ ] |
| **Tests Pass** | `go test ./...` | Exit code 0 | YES | [ ] |
| **Production Code** | Manual check | Production code exists | YES | [ ] |
| **Committed** | `git log -1 --oneline` | Shows sprint commit | YES | [ ] |
| **No Secrets** | `git diff --cached` | No API keys, tokens, passwords | YES | [ ] |
| **UI Loads** | Browser test | http://localhost:8080/web works | NO | [ ] |

**Cycle:** ___ / 3
**Time Started:** ___:___
**Current Status:** VALIDATING | PASSED | BLOCKED | TIMEOUT

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|-----------|------|----------------|
| 1 | All 26 API endpoints implemented | Check `js/api.js` | All methods present |
| 2 | Dashboard page renders | Browser | Stats visible |
| 3 | Documents page works | Browser | List + search + edit |
| 4 | Rules management works | Browser | CRUD + toggle |
| 5 | Projects management works | Browser | CRUD operations |
| 6 | Failures page works | Browser | List + status update |
| 7 | IDE Tools page works | Browser | Validation interface |
| 8 | Navigation works | Browser | All links functional |
| 9 | Server builds | `go build` | Exit code 0 |

---

## ROLLBACK PROCEDURE

```bash
# Immediate rollback - discard all changes
git checkout HEAD -- mcp-server/cmd/server/main.go
rm -rf mcp-server/web
rm -rf mcp-server/internal/web

# Verify rollback
git status

# Report to user
echo "Rollback complete. All web UI changes removed."
```

---

## REFERENCE

### API Endpoint Reference

See `mcp-server/API.md` for complete endpoint documentation.

### Component Patterns

```javascript
// Page Component Pattern
class PageName {
  constructor(container) {
    this.container = container;
    this.render();
    this.attachEvents();
  }

  async render() {
    this.container.innerHTML = `
      <div class="page">
        <h1>Page Title</h1>
        <!-- Content -->
      </div>
    `;
  }

  attachEvents() {
    // Event listeners
  }
}
```

---

## QUICK REFERENCE CARD

```
+------------------------------------------------------------------+
|                    SPRINT QUICK REFERENCE                        |
+------------------------------------------------------------------+
| TARGET DIR:   mcp-server/web/                                    |
|               mcp-server/internal/web/ (NEW)                     |
| CHANGE TYPE:  Create complete SPA                                |
+------------------------------------------------------------------+
| SAFETY:                                                          |
|   - Read before edit                                             |
|   - Isolate web code from API                                    |
|   - Production code FIRST                                        |
|   - Test before commit                                           |
+------------------------------------------------------------------+
| HALT IF:                                                         |
|   - API integration fails                                        |
|   - Server won't build                                           |
|   - Uncertain about component design                             |
+------------------------------------------------------------------+
| ROLLBACK: rm -rf mcp-server/web                                  |
|           rm -rf mcp-server/internal/web                         |
|           git checkout HEAD -- mcp-server/cmd/server/main.go     |
+------------------------------------------------------------------+
```

---

**Created:** 2026-02-08
**Authored by:** TheArchitectit
**Archive Date:** 2026-02-15
**Version:** 1.0
