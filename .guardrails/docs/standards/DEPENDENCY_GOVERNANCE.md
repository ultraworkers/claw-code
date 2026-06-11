# Dependency Governance

> **Control the supply chain.** Only approved packages may be used.

**Related:** [PROJECT_CONTEXT_TEMPLATE.md](./PROJECT_CONTEXT_TEMPLATE.md) | [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## Overview

This document establishes the dependency governance framework - an allow-list approach to package management that prevents AI agents from installing arbitrary, potentially insecure, or incompatible dependencies.

**Core Principle:** If it's not on the approved list, it doesn't get installed.

---

## WHY DEPENDENCY GOVERNANCE

### The Risks of Uncontrolled Dependencies

```
RISK 1: Security Vulnerabilities
- Unknown packages may contain malware
- Abandoned packages may have unpatched CVEs
- Typosquatting attacks (e.g., 'lodash' vs 'lodahs')

RISK 2: License Compliance
- GPL licenses may require open-sourcing your code
- Some licenses prohibit commercial use
- License conflicts can create legal liability

RISK 3: Technical Debt
- Incompatible versions cause runtime errors
- Abandoned packages become maintenance burden
- Duplicate functionality bloats bundle size

RISK 4: AI Hallucination
- AI may invent package names that don't exist
- AI may suggest outdated or deprecated packages
- AI may not know about security issues
```

---

## ALLOW-LIST STRUCTURE

### Package Categories

```
CATEGORY STRUCTURE:

approved_packages.json
├── core/           # Framework essentials (React, Next.js)
├── ui/             # UI components and styling
├── forms/          # Form handling and validation
├── data/           # Data fetching and state
├── utils/          # Utility libraries
├── testing/        # Test frameworks and utilities
├── dev/            # Development-only tools
└── security/       # Security-related packages
```

### Allow-List Template

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "name": "approved-packages",
  "version": "1.0.0",
  "lastUpdated": "2026-01-21",
  "approvedBy": "TheArchitectit",
  
  "categories": {
    "core": {
      "description": "Framework essentials - do not add without architect approval",
      "packages": {
        "react": {
          "allowedVersions": "^18.0.0",
          "license": "MIT",
          "reason": "Core UI framework"
        },
        "next": {
          "allowedVersions": "^14.0.0",
          "license": "MIT",
          "reason": "React framework with SSR"
        },
        "typescript": {
          "allowedVersions": "^5.0.0",
          "license": "Apache-2.0",
          "reason": "Type safety"
        }
      }
    },
    
    "ui": {
      "description": "UI components and styling",
      "packages": {
        "@radix-ui/react-dialog": {
          "allowedVersions": "^1.0.0",
          "license": "MIT",
          "reason": "Accessible dialog component"
        },
        "@radix-ui/react-dropdown-menu": {
          "allowedVersions": "^2.0.0",
          "license": "MIT",
          "reason": "Accessible dropdown"
        },
        "tailwindcss": {
          "allowedVersions": "^3.0.0",
          "license": "MIT",
          "reason": "Utility-first CSS"
        },
        "tailwind-merge": {
          "allowedVersions": "^2.0.0",
          "license": "MIT",
          "reason": "Merge Tailwind classes"
        },
        "clsx": {
          "allowedVersions": "^2.0.0",
          "license": "MIT",
          "reason": "Conditional class names"
        },
        "lucide-react": {
          "allowedVersions": "^0.300.0",
          "license": "ISC",
          "reason": "Icon library"
        }
      }
    },
    
    "forms": {
      "description": "Form handling and validation",
      "packages": {
        "react-hook-form": {
          "allowedVersions": "^7.0.0",
          "license": "MIT",
          "reason": "Performant form handling"
        },
        "zod": {
          "allowedVersions": "^3.0.0",
          "license": "MIT",
          "reason": "Schema validation"
        },
        "@hookform/resolvers": {
          "allowedVersions": "^3.0.0",
          "license": "MIT",
          "reason": "Zod resolver for react-hook-form"
        }
      }
    },
    
    "data": {
      "description": "Data fetching and state management",
      "packages": {
        "@tanstack/react-query": {
          "allowedVersions": "^5.0.0",
          "license": "MIT",
          "reason": "Server state management"
        },
        "zustand": {
          "allowedVersions": "^4.0.0",
          "license": "MIT",
          "reason": "Client state management"
        },
        "axios": {
          "allowedVersions": "^1.0.0",
          "license": "MIT",
          "reason": "HTTP client"
        }
      }
    },
    
    "utils": {
      "description": "General utility libraries",
      "packages": {
        "date-fns": {
          "allowedVersions": "^3.0.0",
          "license": "MIT",
          "reason": "Date manipulation"
        },
        "lodash-es": {
          "allowedVersions": "^4.0.0",
          "license": "MIT",
          "reason": "Utility functions (ES modules)"
        },
        "nanoid": {
          "allowedVersions": "^5.0.0",
          "license": "MIT",
          "reason": "ID generation"
        }
      }
    },
    
    "testing": {
      "description": "Test frameworks and utilities",
      "devOnly": true,
      "packages": {
        "vitest": {
          "allowedVersions": "^1.0.0",
          "license": "MIT",
          "reason": "Test runner"
        },
        "@testing-library/react": {
          "allowedVersions": "^14.0.0",
          "license": "MIT",
          "reason": "React testing utilities"
        },
        "@playwright/test": {
          "allowedVersions": "^1.40.0",
          "license": "Apache-2.0",
          "reason": "E2E testing"
        },
        "msw": {
          "allowedVersions": "^2.0.0",
          "license": "MIT",
          "reason": "API mocking"
        }
      }
    },
    
    "dev": {
      "description": "Development-only tools",
      "devOnly": true,
      "packages": {
        "eslint": {
          "allowedVersions": "^8.0.0",
          "license": "MIT",
          "reason": "Linting"
        },
        "prettier": {
          "allowedVersions": "^3.0.0",
          "license": "MIT",
          "reason": "Code formatting"
        },
        "@types/node": {
          "allowedVersions": "^20.0.0",
          "license": "MIT",
          "reason": "Node.js types"
        }
      }
    }
  },
  
  "forbidden": {
    "description": "Packages explicitly prohibited",
    "packages": {
      "moment": "Use date-fns instead (smaller, tree-shakeable)",
      "lodash": "Use lodash-es instead (ES modules)",
      "request": "Deprecated, use axios or fetch",
      "node-fetch": "Use native fetch (Node 18+)",
      "express": "Use Next.js API routes or Hono",
      "jquery": "Not needed with React"
    }
  },
  
  "licenses": {
    "allowed": ["MIT", "Apache-2.0", "ISC", "BSD-2-Clause", "BSD-3-Clause"],
    "forbidden": ["GPL-2.0", "GPL-3.0", "AGPL-3.0", "LGPL-3.0"],
    "requiresReview": ["MPL-2.0", "CDDL-1.0"]
  }
}
```

---

## AGENT DIRECTIVE

### When Agent Wants to Add a Package

```
DEPENDENCY REQUEST PROTOCOL:

1. CHECK ALLOW-LIST
   - Is package in approved_packages.json?
   - If YES → Proceed with allowed version
   - If NO → Request approval

2. REQUEST APPROVAL TEMPLATE
   "I need to add a package not on the approved list:
   
   Package: [name]
   Version: [version]
   License: [license]
   Purpose: [why it's needed]
   Alternatives Considered: [other options]
   Bundle Size Impact: [size]
   
   Should I add this to the approved list?"

3. USER APPROVES
   - Update approved_packages.json
   - Install package
   - Document reason

4. USER DENIES
   - Find alternative from approved list
   - Or implement functionality without package
```

### Forbidden Package Detection

```
IF AGENT SUGGESTS FORBIDDEN PACKAGE:

1. HALT suggestion
2. Explain why it's forbidden
3. Suggest approved alternative

Example:
"I was going to suggest 'moment' for date handling, but it's 
on the forbidden list (too large, not tree-shakeable). 
Using 'date-fns' instead, which is approved."
```

---

## VALIDATION WORKFLOW

### Pre-Install Check

```bash
# Script: check-dependency.sh

#!/bin/bash
PACKAGE=$1
APPROVED_FILE="approved_packages.json"

# Check if package is in allow-list
if jq -e ".categories[].packages[\"$PACKAGE\"]" "$APPROVED_FILE" > /dev/null 2>&1; then
  echo "✓ Package '$PACKAGE' is approved"
  VERSION=$(jq -r ".categories[].packages[\"$PACKAGE\"].allowedVersions" "$APPROVED_FILE" | head -1)
  echo "  Allowed version: $VERSION"
  exit 0
else
  # Check if explicitly forbidden
  if jq -e ".forbidden.packages[\"$PACKAGE\"]" "$APPROVED_FILE" > /dev/null 2>&1; then
    REASON=$(jq -r ".forbidden.packages[\"$PACKAGE\"]" "$APPROVED_FILE")
    echo "✗ Package '$PACKAGE' is FORBIDDEN"
    echo "  Reason: $REASON"
    exit 2
  else
    echo "? Package '$PACKAGE' is not on the approved list"
    echo "  Request approval before installing"
    exit 1
  fi
fi
```

### CI/CD Integration

```yaml
# .github/workflows/dependency-check.yml
name: Dependency Governance

on:
  pull_request:
    paths:
      - 'package.json'
      - 'package-lock.json'

jobs:
  check-dependencies:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Get changed packages
        id: changes
        run: |
          # Compare package.json changes
          git diff origin/main -- package.json | grep "^\+" | grep -E '"[^"]+":' > new_deps.txt || true
          cat new_deps.txt
      
      - name: Validate against allow-list
        run: |
          # Check each new dependency
          while read line; do
            PACKAGE=$(echo "$line" | grep -oP '"\K[^"]+(?=":)')
            ./scripts/check-dependency.sh "$PACKAGE"
          done < new_deps.txt
      
      - name: License check
        run: npx license-checker --onlyAllow "MIT;Apache-2.0;ISC;BSD-2-Clause;BSD-3-Clause"
      
      - name: Security audit
        run: npm audit --audit-level=high
```

---

## MAINTENANCE

### Adding New Packages

```
PROCESS TO ADD NEW PACKAGE:

1. RESEARCH
   - Check npm for downloads, maintenance status
   - Check Snyk/npm audit for vulnerabilities
   - Verify license compatibility
   - Compare with alternatives

2. DOCUMENT
   - Add to approved_packages.json
   - Include version constraint
   - Include license
   - Include reason for approval

3. NOTIFY
   - Update team on new approved package
   - Document in changelog

4. REVIEW
   - Quarterly review of all approved packages
   - Remove unused packages
   - Update version constraints
```

### Removing Packages

```
PROCESS TO REMOVE PACKAGE:

1. IDENTIFY
   - Package no longer needed
   - Better alternative available
   - Security issues discovered

2. MIGRATE
   - Update code using the package
   - Replace with alternative
   - Remove import statements

3. CLEAN
   - Remove from package.json
   - Remove from approved_packages.json
   - Run npm prune

4. VERIFY
   - All tests pass
   - No runtime errors
   - Bundle size reduced
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              DEPENDENCY GOVERNANCE QUICK REFERENCE                |
+------------------------------------------------------------------+
| RULE: Only install packages from approved_packages.json          |
+------------------------------------------------------------------+
| TO ADD NEW PACKAGE:                                               |
|   1. Check if already approved                                   |
|   2. If not, request approval with:                              |
|      - Package name and version                                  |
|      - License                                                   |
|      - Purpose                                                   |
|      - Alternatives considered                                   |
+------------------------------------------------------------------+
| FORBIDDEN PACKAGES (examples):                                    |
|   moment    → Use date-fns                                       |
|   lodash    → Use lodash-es                                      |
|   request   → Use axios or fetch                                 |
|   jquery    → Not needed with React                              |
+------------------------------------------------------------------+
| ALLOWED LICENSES:                                                 |
|   MIT, Apache-2.0, ISC, BSD-2-Clause, BSD-3-Clause               |
+------------------------------------------------------------------+
| FORBIDDEN LICENSES:                                               |
|   GPL-2.0, GPL-3.0, AGPL-3.0, LGPL-3.0                           |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~380
