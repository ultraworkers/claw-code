---
name: deep-systems-debugger
description: Use when debugging multi-layer or distributed systems where the root cause may reside in a different architectural layer than the symptom, or when standard debugging has not identified the root cause after initial investigation
---

# Deep Systems Debugger

## Overview

In multi-layer systems (CI/CD, distributed services, complex pipelines), the root cause almost never lives in the same layer as the symptom. Random patching wastes time. This skill provides a structured four-phase protocol for tracing failures across architectural boundaries with surgical precision.

**Core principle:** Map every layer and trace every boundary before forming any hypothesis. Be the detective, not the gambler.

## The Iron Law

```
NO FIXES WITHOUT COMPLETED ROOT-CAUSE INVESTIGATION
```

If you have not finished Phase 1, you are forbidden from proposing code changes, configuration tweaks, or operational patches.

## When to Use

- Error manifests in a different layer than where the cause likely lives
- System has 3+ architectural layers (CI/CD pipeline, API gateway → service → DB, distributed services)
- Error message is a transport-level symptom (HTTP error, timeout, decode failure, connection refused)
- Standard investigation has been attempted but root cause remains unclear
- Intermittent or environment-specific failures
- The failure involves configuration, build, or deployment scripts
- Multiple failed fix attempts have already been made

**Do NOT use for:** Simple single-layer bugs (use `systematic-debugging` instead)

## Prerequisites

This skill builds on `systematic-debugging`. If you haven't completed Phase 1-2 of that skill, start there first.

## Quick Reference

| Phase | Focus | Key Technique | Output |
|-------|-------|--------------|--------|
| **1. Root-Cause Mapping** | Observe only | Recursive diff, error routing, boundary instrumentation | Evidence log, divergence point |
| **2. Pattern Analysis** | Analyze before theorizing | Backward tracing, working reference comparison | Single clear hypothesis |
| **3. Scientific Validation** | Minimal experiment | One variable change | Confirmed or rejected hypothesis |
| **4. Permanent Fix** | Lock in root cause | Failing test, isolated fix, regression suite | Fixed bug + test |

## Phase 1: Root-Cause Mapping & Evidence Gathering

*Do not propose fixes. Only observe and trace.*

### 0. Perform Full Recursive Diff of All Layers

Before reading any code, diff the **entire** broken codebase against a known-good reference (previous version, sibling branch, stable release). Sort diff output by architectural layer, outermost to innermost:

```
[CI/Dockerfile] → [Build scripts] → [HTTP client config] → [API wiring] → [Middleware/policy] → [Feature dispatch] → [Business logic]
```

Examine **every** difference, especially in configuration files, builder chains, dependency versions, environment variable handling, and client setup code. Do not filter by suspected feature area.

### 1. Route by Error Type, Then Map from Outermost Layer

Let the **error message text** determine the starting layer:

| Error Keyword | Starting Layer |
|--------------|----------------|
| `http error`, `decode`, `timeout`, `connection refused` | HTTP client config / transport layer |
| `permission denied`, `auth`, `policy` | Middleware / enforcer / policy layer |
| `parse`, `serialize`, `invalid format` | Serialization / API boundary |
| `null pointer`, `index out of bounds`, `unreachable` | Business logic layer |

Trace outward from that layer: identify every architectural layer from outermost trigger down to deepest call. List all middleware, adapters, policy enforcers, aliases, and caching layers.

### 2. Identify All Data Boundaries

For each function, module, or service in the chain, explicitly define:

- **Input**: What enters (type, format, size, origin)
- **Output**: What exits (type, format, serialization, destination)
- **Side Effects**: State mutations, cache writes, external I/O, logging, metric emissions

### 3. Instrument with Diagnostic Tracing

At **EVERY** critical boundary, insert tracing logic (structured logs, print statements, metric counters, span attributes). Record:

- Entry/exit timestamps
- Key input metadata (ID, length, checksum, source)
- Key output metadata (status code, size, target location)
- Environment/context values (auth tokens, feature flags, config overrides)

**Post-trace sanity check:** Before analyzing, scan which layers produced output vs. produced no output. If the outermost transport layer shows the first error, do NOT dig deeper — the failure is already localized.

For large payloads, log size, hash, or truncated preview — never flood logs with raw data.

### 4. Gather Empirical Evidence

Execute the reproduction path once with instrumentation active. Compare observed outputs against expected outputs at every boundary. Note where the two first diverge — that is your initial suspect region.

## Phase 2: Pattern Analysis & Hypothesis Formation

*Analyze evidence before forming a theory.*

1. **Locate Divergence Point** — Find the **first** boundary where reality differs from expectation.
2. **Perform Backward Tracing** — If error manifests deep in stack, ask repeatedly: *"What component supplied this incorrect value?"* Follow chain upward to the original source of invalid state.
3. **Compare Against Working References** — Identify a similar known-good path. List **every** difference, no matter how trivial.
4. **Formulate a Single Clear Hypothesis** — Write explicitly: *"The root cause is likely [X], because the trace shows [Y] at [Z], and this differs from the working example where [W] happens."*

## Phase 3: Scientific Validation (Minimal Experimentation)

*Test the hypothesis with surgical restraint.*

1. **Design the smallest possible test** — Make **one** isolated change to validate your hypothesis. Change only one variable at a time.
2. **Run the reproduction** — If the change resolves the issue → proceed to Phase 4. If not → **STOP**. Discard that hypothesis. Return to Phase 2 with fresh evidence.
3. **NEVER** apply multiple fixes in one test run — you lose the ability to isolate causality.

## Phase 4: Permanent Implementation & Verification

*Fix the root cause and lock it in.*

1. **Create a failing test case** — Minimal automated test that reliably reproduces the original failure.
2. **Apply the single, root-cause fix** — Modify only what is necessary. No opportunistic refactoring.
3. **Run full verification** — New test passes. Existing regression suite passes. Original symptom is gone.
4. **If the fix fails after 3 attempts** — **STOP**. Escalate to architectural review. Repeated failures suggest a deeper structural flaw (improper layering, incorrect state ownership, broken abstraction).

## Command Patterns (Action Sequence)

When beginning a deep debugging session, follow this sequence:

1. **`DIFFING`** — Recursive diff broken vs working across ALL files, sorted outermost to innermost
2. **`MAPPING`** — Route by error type, search codebase, construct end-to-end call chain table
3. **`INSTRUMENTING`** — Generate tracing/logging at every identified boundary
4. **`ANALYZING`** — Execute reproduction, capture traces, pinpoint first divergence
5. **`HYPOTHESIZING`** — State single clear hypothesis with supporting evidence
6. **`VALIDATING`** — Implement minimal change to test hypothesis; report result
7. **`FIXING`** — Commit permanent isolated fix and accompanying regression test

## Universal Constraints

- **Separate data flow from presentation flow** — UI layers consume final output; they are rarely the source of logical corruption. Focus on the core transactional data pipeline.
- **Track all hidden state** — Explicitly log cache hits/misses, environment variables, config precedence, feature flags, and global singletons.
- **Reproducibility first** — If intermittent, increase observability across multiple runs. Do not guess at race conditions.
- **Environment parity** — Always verify if the bug exists only in specific environments. Compare configs, resource limits, and dependency versions.

## Red Flags (Immediate Halt)

If you catch yourself thinking any of these, STOP and return to Phase 1:

- "Let's just change this one thing and see if the test passes."
- "It's probably a race condition; let's add a sleep."
- "I'll write the test after I confirm it works manually."
- "I'll fix these two related issues together since I'm here."
- "This is trivial; I don't need to trace the whole flow."
- "I've tried two patches already — maybe a third will stick."

## Output Structure

When reporting findings, use this format:

### 1. Execution Chain Overview
`[Layer A] → [Layer B] → [Layer C] → ... → [Layer N]`

### 2. Boundary Trace Table
| Boundary | Input | Expected Output | Actual Output | Status |
|----------|-------|----------------|---------------|--------|
| ... | ... | ... | ... | ✅/❌ |

### 3. Root-Cause Hypothesis
*[Concise statement of the suspected origin, supported by trace evidence.]*

### 4. Validation Experiment
*[Description of the minimal change made and the observed result.]*

### 5. Final Resolution
*[The committed fix, the regression test added, and confirmation of success.]*

## Related Skills

- **`systematic-debugging`** — General-purpose debugging process (use this first for most bugs)
- **`test-driven-development`** — For creating failing test cases in Phase 4
- **`verification-before-completion`** — Verify fix worked before claiming success
