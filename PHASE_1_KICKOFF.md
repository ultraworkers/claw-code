# Phase 1 Kickoff — Classifier Sweeps + Doc-Truth + Design Decisions

**Status:** Ready for execution once Phase 0 (`feat/jobdori-168c-emission-routing`) merges.

**Date prepared:** 2026-04-23 11:47 Seoul (cycles #104–#108 complete, all unaudited surfaces probed)

---

## What Got Done (Phase 0)

- ✅ JSON output shape routing (no-silent test, SCHEMAS baseline, parity guard)
- ✅ 7 dogfood filings (#155, #169, #170, #171, #172, #153, checkpoint)
- ✅ 9 probe cycles (plugins, agents, init, bootstrap-plan, system-prompt, export, sandbox, dump-manifests, skills)
- ✅ 82 pinpoints filed, 67 genuinely open
- ✅ 227/227 tests pass, 0 regressions
- ✅ Review guide + priority queue locked
- ✅ Doctrine: 28 principles accumulated

---

## What Phase 1 Will Do (Confirmed via Gaebal-Gajae)

Execute priority-ordered fixes in 6 bundles + independents:

### Priority 1: Error Envelope Contract Drift

**Bundle:** `feat/jobdori-181-error-envelope-contract-drift` (#181 + #183)

**What it fixes:**
- #181: `plugins bogus-subcommand` returns success-shaped envelope (no `type: "error"`, error buried in message)
- #183: `plugins` and `mcp` emit different shapes on unknown subcommand

**Why it's Priority 1:** Foundation layer. Error envelope is the root contract. All downstream fixes assume correct envelope shape.

**Implementation:** Align `plugins` unknown-subcommand handler to `agents` canonical reference. Ensure both emit `type: "error"` + correct `kind`.

**Risk profile:** HIGH (touches error routing, breaks if consumers depend on old shape) → but gated by Phase 0 freeze + comprehensive tests

---

### Priority 2: CLI Contract Hygiene Sweep

**Bundle:** `feat/jobdori-184-cli-contract-hygiene-sweep` (#184 + #185)

**What it fixes:**
- #184: `claw init` silently accepts unknown positional arguments (should reject)
- #185: `claw bootstrap-plan` silently accepts unknown flags (should reject)

**Why it's Priority 2:** Extensions. Guard clauses on existing envelope shape. Uses envelope from Priority 1.

**Implementation:** Add trailing-args rejection to `init` and unknown-flag rejection to `bootstrap-plan`. Pattern: match existing guard in #171 (extra-args classifier).

**Risk profile:** MEDIUM (adds guards, no shape changes)

---

### Priority 3: Classifier Sweep (4 Verbs)

**Bundle:** `feat/jobdori-186-192-classifier-sweep` (#186 + #187 + #189 + #192)

**What it fixes:**
- #186: `system-prompt --<unknown>` classified as `unknown` → should be `cli_parse`
- #187: `export --<unknown>` classified as `unknown` → should be `cli_parse`
- #189: `dump-manifests --<unknown>` classified as `unknown` → should be `cli_parse`
- #192: `skills install --<unknown>` classified as `unknown` → should be `cli_parse`

**Why it's Priority 3:** Cleanup. Classifier additions, same envelope, one unified pattern across 4 verbs.

**Implementation:** Add 4 classifier branches (one per verb) to the unknown-option handler. Same test pattern for all.

**Risk profile:** LOW (classifier-only, no routing changes)

---

### Priority 4: USAGE.md Standalone Surface Audit

**Bundle:** `feat/jobdori-180-usage-standalone-surface` (#180)

**What it fixes:**
- #180: USAGE.md incomplete verb coverage (doc-truthfulness audit-flow)

**Why it's Priority 4:** Doc audit. Prerequisite for #188 (help-text gaps).

**Implementation:** Audit USAGE.md against all verbs (compare against `claw --help` verb list). Add missing verb documentation.

**Risk profile:** LOW (docs-only)

---

### Priority 5: Dump-Manifests Help-Text Fix

**Bundle:** `feat/jobdori-188-dump-manifests-help-prerequisite` (#188)

**What it fixes:**
- #188: `dump-manifests --help` omits prerequisite (env var or flag required)

**Why it's Priority 5:** Doc-truth probe-flow. Comes after audit-flow (#180).

**Implementation:** Update help text to show required alternatives and environment variable.

**Risk profile:** LOW (help-text only)

---

### Priority 6+: Independent Fixes

- #190: Design decision (help-routing for no-args install) — needs architecture review
- #191: `skills install` filesystem classifier gap — can bundle with #177/#178/#179 or standalone
- #182: Plugin classifier alignment (unknown → filesystem/runtime) — depends on #181 resolution
- #177/#178/#179: Install-surface taxonomy (possible 4-verb bundle)
- #173: Config hint field (consumer-parity)
- #174: Resume trailing classifier (closed? verify)
- #175: CI fmt/test decoupling (gaebal-gajae owned)

---

## Concrete Next Steps (Once Phase 0 Merges)

1. **Create branch 1:** `feat/jobdori-181-error-envelope-contract-drift`
   - Files: error router, tests for #181 + #183
   - PR against main
   - Expected: 2 commits, 5 new tests, 0 regressions

2. **Create branch 2:** `feat/jobdori-184-cli-contract-hygiene-sweep`
   - Files: init guard, bootstrap-plan guard
   - PR against main
   - Expected: 2 commits, 3 new tests

3. **Create branch 3:** `feat/jobdori-186-192-classifier-sweep`
   - Files: unknown-option handler (4 verbs)
   - PR against main
   - Expected: 1 commit, 4 new tests

4. **Create branch 4:** `feat/jobdori-180-usage-standalone-surface`
   - Files: USAGE.md additions
   - PR against main
   - Expected: 1 commit, 0 tests

5. **Create branch 5:** `feat/jobdori-188-dump-manifests-help-prerequisite`
   - Files: help text update (string change)
   - PR against main
   - Expected: 1 commit, 0 tests

6. **Triage independents:** #190 requires architecture discussion; others can follow once above merges.

---

## Hypothesis Validation (Codified for Future Probes)

**Multi-flag verbs (install, enable, init, bootstrap-plan, system-prompt, export, dump-manifests):** 3–4 classifier gaps each.

**Single-issue verbs (list, show, sandbox, agents):** 0–1 gaps.

**Future probe strategy:** Prioritize multi-flag verbs; single-issue verbs are mostly clean.

---

## Doctrine Points Relevant to Phase 1 Execution

- **Doctrine #22:** Schema baseline check before enum proposal
- **Doctrine #25:** Contract-surface-first ordering (foundation → extensions → cleanup)
- **Doctrine #27:** Same-pattern pinpoints should bundle into one classifier sweep PR
- **Doctrine #28:** First observation is hypothesis, not filing (verify before classifying)

---

## Known Blockers & Risks

1. **Phase 0 merge gating:** Can't create Phase 1 branches until Phase 0 lands (28 base + 37 new = 65 total pending)
2. **#190 design decision:** help-routing behavior needs architectural consensus (intentional vs inconsistency)
3. **Cross-family dependencies:** #182 depends on #181 (plugin error envelope must be correct first)

---

## Testing Strategy for Phase 1

- **Priority 1–3 bundles:** Existing test framework (`output_format_contract.rs`, classifier tests). Comprehensive coverage per bundle.
- **Priority 4–5 bundles:** Light doc verification (grep USAGE.md, spot-check help text).
- **Independent fixes:** Case-by-case once prioritized.

---

## Success Criteria

- ✅ All Priority 1–5 bundles merge to main
- ✅ 0 regressions (227+ tests pass across all merges)
- ✅ CI green on all PRs
- ✅ Reviewer sign-offs on all bundles

---

**Phase 1 is ready to execute. Awaiting Phase 0 merge approval.**
