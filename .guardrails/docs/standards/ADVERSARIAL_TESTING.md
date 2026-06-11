# Adversarial Testing Protocol (Breaker Agent)

> **Break it before users do.** Systematic protocols for stress-testing AI-generated code.

**Related:** [TESTING_VALIDATION.md](../workflows/TESTING_VALIDATION.md) | [AGENT_REVIEW_PROTOCOL.md](../workflows/AGENT_REVIEW_PROTOCOL.md)

---

## Overview

This document defines the "Breaker Agent" protocol - a systematic approach to adversarial testing that attempts to crash, exploit, or break AI-generated code before deployment. The goal is not to confirm code works; it is to prove it can't be broken.

**Philosophy:** "If you don't actively try to break your code, your users will do it for you."

---

## THE BREAKER AGENT PERSONA

### Agent Configuration

When running adversarial tests, configure the agent with this persona:

```
PERSONA: Senior QA Security Engineer

MISSION: Find every way to crash, exploit, or misbehave this code.

MINDSET:
- Assume the code is broken until proven otherwise
- Think like a malicious user, not a happy-path user
- Every input is a potential attack vector
- Every edge case is a potential crash

GOAL: Produce a comprehensive failure report, not a success confirmation.
```

### Breaker vs Builder Separation

```
BUILDER AGENT:
- Creates features
- Writes "happy path" tests
- Assumes good faith input
- Optimizes for functionality

BREAKER AGENT:
- Destroys features (in testing)
- Writes "sad path" tests
- Assumes malicious input
- Optimizes for finding failures

RULE: The same agent should NOT build and break.
      Use separate sessions or separate agents.
```

---

## ATTACK VECTOR CATEGORIES

### 1. Input Validation Attacks

#### String Attacks

| Attack Type | Test Input | Expected Behavior |
|-------------|------------|-------------------|
| Empty string | `""` | Graceful error, not crash |
| Whitespace only | `"   "` | Rejected or trimmed |
| Very long string | 10,000+ characters | Rejected with limit error |
| Unicode edge cases | `"café"`, `"日本語"`, `"🔥🎉"` | Handled correctly |
| Null bytes | `"hello\x00world"` | Sanitized or rejected |
| Newlines | `"line1\nline2"` | Handled per requirements |
| Control characters | `"\t\r\n\b"` | Sanitized |

#### XSS (Cross-Site Scripting) Attacks

```javascript
// Test these inputs against any user-facing text field:

const xssPayloads = [
  '<script>alert("XSS")</script>',
  '<img src="x" onerror="alert(1)">',
  '<svg onload="alert(1)">',
  'javascript:alert(1)',
  '<iframe src="javascript:alert(1)">',
  '"><script>alert(1)</script>',
  "'-alert(1)-'",
  '<body onload="alert(1)">',
];

// EXPECTED: All should be escaped or rejected
// FAILURE: Any script execution = CRITICAL vulnerability
```

#### SQL Injection Attacks

```javascript
// Test these against any database-connected input:

const sqlPayloads = [
  "'; DROP TABLE users; --",
  "' OR '1'='1",
  "1; SELECT * FROM users",
  "admin'--",
  "' UNION SELECT * FROM passwords --",
  "1 AND 1=1",
  "1' AND '1'='1",
];

// EXPECTED: All should be parameterized/escaped
// FAILURE: Any SQL execution = CRITICAL vulnerability
```

#### Number Attacks

| Attack Type | Test Input | Expected Behavior |
|-------------|------------|-------------------|
| Zero | `0` | Handled (no division errors) |
| Negative | `-1`, `-999999` | Rejected or handled |
| Float precision | `0.1 + 0.2` | Handled correctly |
| Very large | `Number.MAX_SAFE_INTEGER + 1` | Rejected or BigInt |
| NaN | `NaN` | Rejected or handled |
| Infinity | `Infinity`, `-Infinity` | Rejected |
| String as number | `"123abc"` | Rejected |

### 2. Boundary Condition Attacks

#### Array/Collection Attacks

```javascript
const boundaryTests = [
  [],                          // Empty array
  [null],                      // Array with null
  [undefined],                 // Array with undefined
  Array(10000).fill(1),        // Very large array
  [1, [2, [3, [4]]]],         // Deeply nested
  new Array(1000000),          // Sparse array
];
```

#### Object Attacks

```javascript
const objectTests = [
  {},                          // Empty object
  { __proto__: { admin: true } }, // Prototype pollution
  { constructor: { } },        // Constructor pollution
  null,                        // Null object
  Object.create(null),         // No prototype
  { [Symbol()]: 'hidden' },    // Symbol keys
];
```

### 3. State-Based Attacks

#### Race Conditions

```javascript
// Test concurrent operations:

async function raceConditionTest() {
  // Submit same form 100 times simultaneously
  const promises = Array(100).fill().map(() => 
    submitPayment({ amount: 100 })
  );
  
  const results = await Promise.all(promises);
  
  // EXPECTED: Only 1 payment processed
  // FAILURE: Multiple payments = race condition bug
}
```

#### Session Attacks

```javascript
const sessionTests = [
  { token: '' },               // Empty token
  { token: 'expired_token' },  // Expired token
  { token: 'malformed' },      // Invalid format
  { token: null },             // Null token
  { /* no token */ },          // Missing token
  { token: 'other_user_token' }, // Token from different user
];
```

### 4. Resource Exhaustion Attacks

#### Memory Exhaustion

```javascript
// Test inputs that could exhaust memory:

const memoryTests = [
  'a'.repeat(1000000000),      // 1GB string
  JSON.parse('{"a":'.repeat(100000) + '1' + '}'.repeat(100000)), // Deep JSON
  new Array(100000000),        // Huge array allocation
];

// EXPECTED: Rejection before allocation
// FAILURE: Out of memory crash
```

#### CPU Exhaustion (ReDoS)

```javascript
// Regular expressions vulnerable to catastrophic backtracking:

const redosPayloads = [
  'a'.repeat(30) + '!',        // For regex like /^(a+)+$/
  'aaaaaaaaaaaaaaaaaaaaaaaaaaaa!', // Exponential backtracking
];

// EXPECTED: Timeout or rejection
// FAILURE: CPU hang
```

---

## FUZZ TESTING PROTOCOL

### Automated Fuzzing Setup

```javascript
// fuzzer.js - Generic input fuzzer

const FUZZ_ITERATIONS = 1000;

function generateFuzzInput() {
  const generators = [
    () => '',                                    // Empty
    () => null,                                  // Null
    () => undefined,                             // Undefined
    () => Math.random().toString(36),            // Random string
    () => Math.floor(Math.random() * 1000000),   // Random number
    () => Array(Math.floor(Math.random() * 100)).fill(Math.random()), // Random array
    () => ({ [Math.random()]: Math.random() }), // Random object
    () => '<script>alert(1)</script>',          // XSS
    () => "'; DROP TABLE users; --",            // SQL injection
    () => 'a'.repeat(10000),                    // Long string
    () => -Math.floor(Math.random() * 1000000), // Negative number
    () => NaN,                                   // NaN
    () => Infinity,                              // Infinity
  ];
  
  return generators[Math.floor(Math.random() * generators.length)]();
}

async function fuzzEndpoint(endpoint, method = 'POST') {
  const results = { passed: 0, failed: 0, errors: [] };
  
  for (let i = 0; i < FUZZ_ITERATIONS; i++) {
    const input = generateFuzzInput();
    try {
      const response = await fetch(endpoint, {
        method,
        body: JSON.stringify({ data: input }),
        headers: { 'Content-Type': 'application/json' },
      });
      
      // Success = 2xx or 4xx (handled error)
      // Failure = 5xx (unhandled error)
      if (response.status >= 500) {
        results.failed++;
        results.errors.push({ input, status: response.status });
      } else {
        results.passed++;
      }
    } catch (error) {
      results.failed++;
      results.errors.push({ input, error: error.message });
    }
  }
  
  return results;
}
```

### Fuzz Test Directive

```
FUZZ TESTING PROTOCOL:

Target: [endpoint or function]
Iterations: 1000 minimum
Input Types: All categories above

PASS CRITERIA:
- Zero 5xx errors
- Zero unhandled exceptions
- Zero crashes
- All 4xx errors have meaningful messages

FAILURE CRITERIA:
- Any 5xx error → CRITICAL
- Any unhandled exception → CRITICAL
- Any crash → CRITICAL
- Timeout without rejection → HIGH
```

---

## COMPONENT-SPECIFIC ATTACK CHECKLISTS

### Form Component Attacks

```
ATTACK CHECKLIST: Form Component

[ ] Submit empty form
[ ] Submit with all fields empty strings
[ ] Submit with XSS in each text field
[ ] Submit with SQL injection in each text field
[ ] Submit with 10,000 character strings
[ ] Submit with emoji-only strings
[ ] Submit with unicode edge cases
[ ] Submit same form 100 times rapidly
[ ] Submit with JavaScript disabled
[ ] Submit with modified hidden fields
[ ] Submit bypassing client validation
```

### API Endpoint Attacks

```
ATTACK CHECKLIST: API Endpoint

[ ] Call without authentication
[ ] Call with expired token
[ ] Call with malformed token
[ ] Call with other user's token
[ ] Send malformed JSON
[ ] Send wrong Content-Type
[ ] Send empty body
[ ] Send body > 10MB
[ ] Send deeply nested JSON (100 levels)
[ ] Call 1000 times in 1 second (rate limit)
[ ] Call with HTTP method not allowed
```

### File Upload Attacks

```
ATTACK CHECKLIST: File Upload

[ ] Upload empty file
[ ] Upload file > max size
[ ] Upload file with wrong extension
[ ] Upload .exe renamed to .jpg
[ ] Upload file with null bytes in name
[ ] Upload file with path traversal (../../../etc/passwd)
[ ] Upload SVG with embedded JavaScript
[ ] Upload zip bomb (compressed 1MB → 1GB)
[ ] Upload same file 100 times rapidly
[ ] Upload with modified Content-Type header
```

### Authentication Attacks

```
ATTACK CHECKLIST: Authentication

[ ] Login with empty credentials
[ ] Login with SQL injection
[ ] Login 100 times with wrong password (lockout)
[ ] Login with valid user, no password
[ ] Login with case-modified username
[ ] Password reset with non-existent email
[ ] Password reset token replay
[ ] Session fixation attempt
[ ] CSRF token bypass attempt
[ ] Remember-me token theft simulation
```

---

## BREAKER AGENT PROMPT TEMPLATE

Use this prompt to configure a "Breaker Agent" session:

```
BREAKER AGENT SESSION

PERSONA: You are a Senior Security QA Engineer specializing in 
breaking software. Your job is to find bugs, not confirm success.

TARGET: [Component/Endpoint Name]

OBJECTIVE: Systematically attempt to crash, exploit, or cause 
unexpected behavior in the target.

ATTACK CATEGORIES TO USE:
1. Input validation attacks (XSS, SQLi, boundary values)
2. State-based attacks (race conditions, session manipulation)
3. Resource exhaustion attacks (memory, CPU, network)
4. Authentication/authorization bypass attempts

REPORT FORMAT:

For each attack attempted, report:
- Attack type
- Input used
- Expected behavior
- Actual behavior
- Severity (CRITICAL / HIGH / MEDIUM / LOW)
- Reproduction steps

CONSTRAINTS:
- Do NOT attack production systems
- Do NOT use attacks that could cause data loss
- Do NOT attempt actual credential theft
- Stay within test environment boundaries

BEGIN ATTACK SEQUENCE:
```

---

## INTEGRATION WITH CI/CD

### Automated Adversarial Tests

```yaml
# .github/workflows/adversarial-tests.yml
name: Adversarial Testing

on:
  pull_request:
    branches: [main]

jobs:
  security-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Run OWASP ZAP scan
        uses: zaproxy/action-full-scan@v0.8.0
        with:
          target: 'http://localhost:3000'
      
      - name: Run SQLMap
        run: |
          sqlmap -u "http://localhost:3000/api/search?q=test" --batch --level=5
      
      - name: Run Fuzz Tests
        run: npm run test:fuzz
      
      - name: Check results
        run: |
          if [ -f "fuzz-failures.json" ]; then
            echo "Fuzz tests found failures!"
            cat fuzz-failures.json
            exit 1
          fi
```

### Blocking Gate

```
ADVERSARIAL TEST GATE:

BLOCKING (Must Pass):
[ ] Zero XSS vulnerabilities detected
[ ] Zero SQL injection vulnerabilities detected
[ ] Zero authentication bypasses
[ ] Fuzz tests: < 1% failure rate
[ ] Rate limiting verified functional

NON-BLOCKING (Should Pass):
[ ] All edge cases handled gracefully
[ ] Error messages are user-friendly
[ ] Timeouts configured appropriately
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              ADVERSARIAL TESTING QUICK REFERENCE                  |
+------------------------------------------------------------------+
| ATTACK CATEGORIES:                                                |
|   1. Input Validation (XSS, SQLi, boundaries)                     |
|   2. State-Based (race conditions, sessions)                      |
|   3. Resource Exhaustion (memory, CPU)                            |
|   4. Auth/AuthZ Bypass                                            |
+------------------------------------------------------------------+
| KEY PAYLOADS:                                                     |
|   XSS: <script>alert(1)</script>                                  |
|   SQLi: ' OR '1'='1                                               |
|   Long: 'a'.repeat(10000)                                         |
|   Empty: '', null, undefined                                      |
+------------------------------------------------------------------+
| FUZZ TESTING:                                                     |
|   Iterations: 1000 minimum                                        |
|   Pass: Zero 5xx errors                                           |
|   Fail: Any unhandled exception                                   |
+------------------------------------------------------------------+
| RULE: Builder agent ≠ Breaker agent                               |
|       Separate sessions for building and breaking                 |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~400
