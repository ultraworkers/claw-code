# TUI Documentation for Claw Code

This directory contains all research, analysis, and architecture documentation for the TUI mode (`claw tui`).

## Architecture Plan

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** — The definitive architecture plan, synthesized from 5 expert MoA (Mixture of Agents) reports. This is the implementation blueprint.

## Original Plans

- **[TUI.md](./TUI.md)** — Initial research & build plan
- **[TUI-52-WEEK-PLAN.md](./TUI-52-WEEK-PLAN.md)** — 52-week roadmap
- **[TUI-52-WEEK-SPRINT.md](./TUI-52-WEEK-SPRINT.md)** — Sprint-level breakdown

## MoA Research Reports

Five independent expert analyses were conducted before writing the architecture plan:

| Report | Focus | Key Takeaway |
|--------|-------|--------------|
| [moa-codex-rs.md](./moa-codex-rs.md) | OpenAI's codex-rs/tui — closest prior art | `tokio::select!` event loop, demand-driven frames, active/committed cell split, adaptive 2-gear chunking |
| [moa-aichat.md](./moa-aichat.md) | sigoden/aichat streaming markdown rendering | "Erase-and-redraw" with 50ms batch, LineType state machine, textwrap for CJK |
| [moa-ecosystem.md](./moa-ecosystem.md) | Broader Rust TUI ecosystem survey | **Render to stderr** to solve output bleeding; tui-markdown, ansi-to-tui, virtual scroll, testing patterns |
| [moa-failure-analysis.md](./moa-failure-analysis.md) | Root-cause analysis of 3 failed TUI attempts | All failures share one root: runtime prints to stdout; TUI grafted on top |
| [moa-runtime-integration.md](./moa-runtime-integration.md) | claw-code runtime integration points | `TurnProgressReporter` only fires post-tool; `ApiClient::stream()` returns collected Vec — no live streaming channel |

## Reading Order

1. **ARCHITECTURE.md** — The plan (start here)
2. **moa-failure-analysis.md** — Why the previous 3 attempts failed
3. **moa-codex-rs.md** — The closest working reference implementation
4. **moa-ecosystem.md** — Broader patterns and crate recommendations
5. **moa-aichat.md** — Streaming markdown rendering deep-dive
6. **moa-runtime-integration.md** — How to hook into claw-code's runtime

## Key Decisions

- **Render to stderr** — stdout freed for child processes, eliminates output bleeding
- **Event-driven architecture** — runtime emits `TuiTurnEvent` via channel, TUI consumes
- **Forward-offset scroll** — 0=top, max=bottom (kills the inverted-scroll bugs)
- **Active cell + committed cells** — streaming cell flushed to markdown cell on completion
- **Modify `ApiClient::stream()`** — return async stream for live token rendering
- **`--tui-stdio` mode** — JSON-over-stdio for programmatic driving (separate PR)
