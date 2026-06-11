# Project Context Template (Project Bible)

> **The Single Source of Truth** for AI agent behavior in this repository.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [DEPENDENCY_GOVERNANCE.md](./DEPENDENCY_GOVERNANCE.md)

---

## Overview

This template defines the "Project Bible" - a context container that forces AI agents to adhere to specific architectural standards, coding patterns, and forbidden practices. Every AI session MUST ingest this context before writing code.

**Purpose:** Prevent the "Stochastic Gap" where AI reverts to generic/outdated patterns instead of your project-specific standards.

---

## HOW TO USE THIS TEMPLATE

1. Copy this template to your project root as `PROJECT_CONTEXT.md` or `.cursorrules`
2. Fill in project-specific values in each section
3. Ensure all AI agents read this file BEFORE any code generation
4. Update when stack versions change or new patterns are adopted

---

## TEMPLATE START

```markdown
# Project Context: [PROJECT_NAME]

**Version:** 1.0
**Last Updated:** YYYY-MM-DD
**Stack Version Lock Date:** YYYY-MM-DD

---

## 1. TECH STACK CONSTRAINTS (Hard Limits)

### Primary Stack

| Technology | Version | Paradigm | Notes |
|------------|---------|----------|-------|
| Language | [e.g., TypeScript 5.4] | [Strict Mode] | [No `any` type] |
| Framework | [e.g., Next.js 14.2] | [App Router] | [No Pages Router] |
| Styling | [e.g., Tailwind CSS 3.4] | [Utility-first] | [No inline styles] |
| Database | [e.g., PostgreSQL 16] | [via Prisma ORM] | [No raw SQL] |
| State | [e.g., Zustand 4.5] | [Atomic stores] | [No Redux] |

### Version Lock Directive

"You are an expert Senior Engineer. When generating code:
- Use ONLY the versions specified above
- Do NOT suggest upgrades or alternatives
- If a feature requires a different version, HALT and ask"

### Package Manager

- **Use:** [npm | pnpm | yarn | bun]
- **Lock file:** [package-lock.json | pnpm-lock.yaml | yarn.lock | bun.lockb]
- **Install command:** [npm install | pnpm add | yarn add | bun add]

---

## 2. CODING STYLE GUIDE (The "Vibe")

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Variables | camelCase, descriptive | `isUserLoggedIn`, `fetchUserData` |
| Functions | camelCase, verb-first | `getUserById`, `validateEmail` |
| Components | PascalCase | `UserProfile`, `NavigationBar` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRY_ATTEMPTS`, `API_BASE_URL` |
| Files | kebab-case | `user-profile.tsx`, `api-client.ts` |
| Types/Interfaces | PascalCase, I-prefix optional | `User`, `IUserResponse` |

### Export Patterns

```typescript
// REQUIRED: Named exports for components
export const Button = () => { ... };
export const UserCard = () => { ... };

// FORBIDDEN: Default exports (harder to refactor)
// export default Button;  // NO
```

### Function Style

```typescript
// REQUIRED: Arrow functions for components
export const MyComponent = () => { ... };

// REQUIRED: Function declarations for utilities
export function calculateTax(amount: number): number { ... }

// FORBIDDEN: Mixed styles in same file
```

### Comment Standards

```typescript
// REQUIRED: JSDoc for public functions
/**
 * Fetches user data from the API.
 * @param userId - The unique identifier of the user
 * @returns Promise resolving to User object
 * @throws {ApiError} When user not found
 */
export async function getUser(userId: string): Promise<User> { ... }

// FORBIDDEN: Inline comments explaining obvious code
// const x = 1; // set x to 1  // NO
```

---

## 3. ARCHITECTURAL PATTERNS (Enforced)

### Directory Structure

```
src/
├── app/                    # Next.js App Router pages
├── components/
│   ├── ui/                 # Primitive components (Button, Input)
│   │   └── Button/
│   │       ├── Button.tsx
│   │       ├── Button.test.tsx
│   │       └── index.ts    # Barrel export
│   └── features/           # Feature-specific components
├── lib/                    # Utilities, helpers
├── hooks/                  # Custom React hooks
├── services/               # API clients, external services
├── stores/                 # State management
├── types/                  # TypeScript types/interfaces
└── constants/              # App-wide constants
```

### Barrel Pattern (MANDATORY)

Every component directory MUST have an `index.ts`:

```typescript
// components/ui/Button/index.ts
export { Button } from './Button';
export type { ButtonProps } from './Button';
```

### Dependency Flow (One-Way Street)

```
Layer 1 (UI):        Components    → Can import from Layer 2, 3
Layer 2 (Logic):     Hooks/Services → Can import from Layer 3 only
Layer 3 (Core):      Utils/Types   → Cannot import from Layer 1, 2

VIOLATION: If a utility imports a component → REJECT
```

### File Size Limits

| File Type | Max Lines | Action if Exceeded |
|-----------|-----------|-------------------|
| Components | 150 | Split into sub-components |
| Utilities | 100 | Extract to separate modules |
| Types | 200 | Split by domain |
| Tests | 300 | Split by test category |

---

## 4. FORBIDDEN PATTERNS (No-Go Zone)

### TypeScript Forbidden

```typescript
// FORBIDDEN: any type
const data: any = response;  // NO

// REQUIRED: Proper typing
const data: UserResponse = response;  // YES

// FORBIDDEN: Type assertions without validation
const user = data as User;  // NO

// REQUIRED: Type guards or validation
if (isUser(data)) { const user = data; }  // YES
```

### React Forbidden

```typescript
// FORBIDDEN: useEffect for data fetching
useEffect(() => {
  fetchData();  // NO
}, []);

// REQUIRED: TanStack Query or Server Components
const { data } = useQuery({ queryKey: ['users'], queryFn: fetchUsers });  // YES

// FORBIDDEN: Inline styles
<div style={{ color: 'red' }}>  // NO

// REQUIRED: Tailwind classes
<div className="text-red-500">  // YES

// FORBIDDEN: Class components
class MyComponent extends React.Component { }  // NO
```

### Database Forbidden

```typescript
// FORBIDDEN: Raw SQL in application code
const result = await db.query('SELECT * FROM users WHERE id = $1', [id]);  // NO

// REQUIRED: ORM methods
const user = await prisma.user.findUnique({ where: { id } });  // YES

// FORBIDDEN: Direct database URL in code
const db = new Client('postgresql://user:pass@localhost/db');  // NO

// REQUIRED: Environment variable
const db = new Client(process.env.DATABASE_URL);  // YES
```

### Security Forbidden

```typescript
// FORBIDDEN: Hardcoded secrets
const API_KEY = 'sk-1234567890';  // NO

// REQUIRED: Environment variables
const API_KEY = process.env.STRIPE_API_KEY;  // YES

// FORBIDDEN: eval() or Function()
eval(userInput);  // NEVER

// FORBIDDEN: innerHTML with user data
element.innerHTML = userInput;  // NO
```

---

## 5. CHAIN OF THOUGHT MANDATE

### Protocol: Plan Before Execution

"Before writing ANY code, you MUST:

1. **Restate Requirements**
   - What is being asked?
   - What are the acceptance criteria?

2. **Outline File Structure**
   - Which files will be created/modified?
   - What is the dependency order?

3. **List Dependencies**
   - What imports are needed?
   - Are they in the approved list?

4. **Identify Risks**
   - What could go wrong?
   - What is the rollback plan?

5. **Wait for Approval**
   - Present plan to user
   - Do NOT proceed until confirmed"

---

## 6. VALIDATION REQUIREMENTS

### Before Committing

```
[ ] TypeScript compiles with zero errors (tsc --noEmit)
[ ] ESLint passes with zero warnings (eslint . --max-warnings 0)
[ ] All tests pass (npm test)
[ ] No forbidden patterns used
[ ] File size limits respected
[ ] Barrel exports created for new components
```

### Code Review Checklist

```
[ ] Follows naming conventions
[ ] Uses approved dependencies only
[ ] No any types
[ ] No inline styles
[ ] No useEffect for data fetching
[ ] Proper error handling
[ ] JSDoc comments on public functions
```

---

## 7. APPROVED DEPENDENCIES

See [DEPENDENCY_GOVERNANCE.md](./DEPENDENCY_GOVERNANCE.md) for the full allow-list.

Quick reference:
- UI: `@radix-ui/*`, `tailwind-merge`, `clsx`
- Forms: `react-hook-form`, `zod`
- Data: `@tanstack/react-query`, `axios`
- State: `zustand`
- Utils: `date-fns`, `lodash-es`

---

## QUICK REFERENCE CARD

```
+------------------------------------------------------------------+
|              PROJECT CONTEXT QUICK REFERENCE                      |
+------------------------------------------------------------------+
| STACK: [Language] [Version] | [Framework] [Version]               |
| STYLE: [Paradigm] | Named exports | Arrow functions               |
+------------------------------------------------------------------+
| FORBIDDEN:                                                        |
|   - any type                                                      |
|   - useEffect for data fetching                                   |
|   - Inline styles                                                 |
|   - Raw SQL                                                       |
|   - Hardcoded secrets                                             |
+------------------------------------------------------------------+
| REQUIRED:                                                         |
|   - Barrel pattern for components                                 |
|   - JSDoc for public functions                                    |
|   - Environment variables for secrets                             |
|   - Plan before execution                                         |
+------------------------------------------------------------------+
| MAX FILE SIZES:                                                   |
|   Components: 150 lines | Utils: 100 lines | Types: 200 lines     |
+------------------------------------------------------------------+
```
```

---

## EXAMPLE: Filled Template (Next.js Project)

```markdown
# Project Context: E-Commerce Platform

**Version:** 1.0
**Last Updated:** 2026-01-21
**Stack Version Lock Date:** 2026-01-01

## 1. TECH STACK CONSTRAINTS

| Technology | Version | Paradigm | Notes |
|------------|---------|----------|-------|
| TypeScript | 5.4.2 | Strict Mode | No `any`, no implicit any |
| Next.js | 14.2.0 | App Router | No Pages Router |
| React | 18.3.0 | Functional only | No class components |
| Tailwind CSS | 3.4.0 | Utility-first | No CSS modules |
| Prisma | 5.10.0 | ORM | No raw SQL |
| PostgreSQL | 16 | Via Prisma | No direct connections |

## 4. FORBIDDEN PATTERNS

- `any` type → Use `unknown` with type guards
- `useEffect` for fetching → Use `useQuery` or Server Components
- `style={}` props → Use Tailwind classes
- Default exports → Use named exports
- `console.log` in production → Use structured logger
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~350
