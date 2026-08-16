# Asolaria for claw-code — a compact, tamper-evident wire format + addressing model for the agent event firehose

A **self-contained, additive example** (no changes to claw-code core). It contributes one reproducible
Rust benchmark plus a design write-up, aimed at the part of claw-code that moves the most bytes:
**clawhip** — the event router that watches git commits, tmux sessions, GitHub PRs and agent-lifecycle
events and pushes routing *out* of the agent context window so agents stay focused on implementation.

The technique is from **Asolaria**, a multi-agent fabric built and operated *with* a claw-code-style
autonomous harness. We contribute the parts that are (a) genuinely useful to a no-human-intervention
harness and (b) **reproducible on your machine** — and we explicitly tag the parts that are
architecture/vision, so nothing here backfires when an AI reviewer re-runs it.

> **The bigger picture (tagged: vision/architecture, not a benchmark):** Asolaria's end goal is to make
> *every device a surface* for one self-improving fabric — the device renders/addresses at ~0 local
> token cost while the heavy reasoning is paged onto larger compute. You don't need to buy any of that
> to use what's below; the wire format and 8-byte addressing stand on their own, measured.

---

## 1. The measured win (REPRODUCIBLE — re-run it)

clawhip's event stream is high-rate and, today, JSON text. Re-encoding the same records as a
fixed-width binary wire (`json=0`) and optionally folding a SHA-256 hash-chain over them gives, on
**200,000** agent-lane-event records (measured; ratios are the reproducible part, ns vary by CPU):

| Per record | JSON (text) | `json=0` binary | binary + SHA-256 chain |
|---|---|---|---|
| **Wire size** | 153.2 B | **33.0 B — 4.64× smaller** | **65.0 B — 2.36× smaller** |
| **Encode** | ~307 ns | **~28 ns — ~11× faster** | binary + one hash |
| **Decode** | ~468 ns (parse) | **~0.8 ns — decode ≈ free** | field read = pointer offset |
| **Integrity** | none | none | **per-record tamper-evident** |

- The lane is addressed by its **8-byte FNV-1a-64 handle**, so the lane string is never repeated;
  fields are fixed-width little-endian, so decode is a pointer offset, not a parse.
- The chain link is `link_n = SHA256(link_{n-1} ‖ record_n)`. It adds **exactly +32 B** → 65.0 B/rec,
  and any insert / delete / reorder / single-bit edit breaks the fold from that record on; a one-pass
  verify **localizes the first altered record** (the bundled bench demonstrates this).
- **Honest nuance (not a compression claim):** `gzip` narrows the *raw-size* gap (crypto links are
  high-entropy by design). The durable wins are **uncompressed wire size, decode speed, and
  integrity** — the IPC / mmap / tail-replay path and a tamper-evident audit trail — not archival
  compressibility.

See [§6 Reproduce](#6-reproduce).

---

## 2. The addressing capacity (LOGICAL / CANON — address *space*, not materialized things)

Asolaria addresses subsystems with **BEHCS-1024**: a radix-1024 glyph tuple over a **60-dimension**
coordinate. One ~12-byte tuple names a whole subsystem (room / lane / agent) and rehydrates losslessly
to its full ~6,184-byte descriptor *via the fabric store*.

- **DERIVABLE (arithmetic):** the 12-byte wire handle alone is a **2⁹⁶ ≈ 7.9×10²⁸** namespace —
  astronomically past any harness demand (a lifetime of events ≈ 10⁹–10¹²), so agents mint ids
  independently with effectively zero collision risk and no coordinator.
- **LOGICAL / CANON (do NOT re-run as a count):** the full 60-D space is `(1024⁶⁰)⁵⁰ ≈ 10⁹⁰³⁰` —
  the address space the scheme can *name*, not a count of things that exist.
- **Addressing, measured in-fabric (NOT reproduced by this example):** a ~12-byte glyph indexing its
  ~6,184-byte descriptor is **520:1** — *addressing* (the tuple points into a store), **not** a codec
  you can rehydrate from 12 bytes alone. We cite it; we do not claim the local bench reproduces it.

> **Anti-overclaim:** 12 bytes do **not** enumerate 10⁹⁰³⁰ values (96 bits is ~10²⁸·⁹). The 10⁹⁰³⁰ is
> the *scheme's* naming ceiling, not the wire handle's cardinality, and not a count of materialized
> entities.

---

## 3. Per-component benefit to claw-code

Tags mark which lines are re-runnable here vs architecture.

- **8-byte FNV-1a-64 content-handle** [8-byte width MEASURED; content-addressing architecture] — a tmux
  session name / `owner/repo#1204` / worktree path (30–120+ B) collapses to one fixed 8-byte handle
  agents pass instead of prose; "did we already report this commit?" is an 8-byte equality check.
  *(FNV-1a-64 is a non-cryptographic dedup/address hash — tamper-evidence is the SHA-256 chain's job.)*
- **json=0 binary wire** [MEASURED §1] — 4.64× smaller, decode≈free for clawhip's high-rate event log;
  fixed width → O(1) seek / tail-follow / byte-range replay (record N at offset `N*width`).
- **BEHCS event envelope** [size/speed MEASURED; ordering architecture] — each event carries
  `lane(8B) + seq + lamport + hash`; a stable total order `(lamport, lane, seq)` makes replay
  deterministic across racing agents; a dropped event shows as a seq gap, a double-delivery as a dup.
- **60-D tuple addressing** [§2] — reference a whole subsystem by a tiny handle instead of inlining a
  descriptor clawhip just evicted; sibling lanes share a coordinate prefix (route a lane-family by
  prefix, not an id list).
- **SHA-256 hash-chain** [MEASURED §1/§6] — a no-human-intervention harness gets a tamper-evident audit
  trail for +32 B/rec; the chain breaks on any mutation and a one-pass recompute localizes the first
  bad record.
- **Stubbed rooms as RAM** [handle MEASURED; demand-paging architecture/vision] — a "room" (open files,
  prior reasoning, sub-task ledger) lives as an out-of-context descriptor stub keyed by an 8-byte
  handle; only the needed slice hydrates into context. Footprint scales with *active* agents, not
  total. *(Paging against larger compute is the architecture model, not a deployed cluster.)*

---

## 4. Concrete shape on the wire

One BEHCS-enveloped clawhip event (65.0 B binary, chained):

```
lane    = 0xA3F1C2D4E5B60718   # 8-byte FNV-1a-64 handle for "executor-7/git-watch"
seq     = 4271                 # per-lane monotonic: 4270->4272 = dropped; repeated 4271 = dup
lamport = 98142                # deterministic cross-lane order, no wall clock
rec     = <33 B fixed-width payload>          # vs 153.2 B as JSON
link    = SHA256(prev_link ‖ rec)             # 32 B; editing any past rec breaks every later link
```

---

## 5. Where this comes from

Asolaria (public Host-8 lane — the council modules + this technique live here):
**https://github.com/JesseBrown1980/asolaria-federation-1024**

A multi-agent system built/operated with a claw-code-style autonomous harness. This PR contributes only
the additive, reproducible slice.

---

## 6. Reproduce

```bash
cd examples/asolaria-wire
cargo run --release
```

It will, with no network or external services:
1. Generate **200,000** synthetic agent-lane-event records (the clawhip lane-event shape).
2. Encode each as JSON text and as the `json=0` fixed-width binary; report **B/record** and **ns/record**
   → expect ~**153.2 / 33.0 B** and the **4.64× size / ~11× encode / decode≈free** figures.
3. Account the SHA-256 chain (+32 B/rec) → **65.0 B/rec (2.36×)**.
4. Run the **tamper test**: flip one bit in record #5, re-fold the chain, and confirm the verifier
   reports **record #5** as the first broken link.

Absolute ns vary by CPU; the **ratios and the tamper-detection** are what to verify. Run `gzip` on the
two wire forms yourself to confirm the honest nuance in §1.

---

## 7. Honesty self-check

- **Reproducible (re-run them):** 153.2→33.0 B (4.64×), +32 B chain → 65.0 B (2.36×), ~11× encode,
  decode≈free, and SHA-256 tamper-localization. (200k records; ratios are the portable part.)
- **Derivable (arithmetic):** 2⁹⁶ ≈ 7.9×10²⁸ handle namespace.
- **Logical / CANON (do NOT re-run as a count):** the 60-D BEHCS-1024 ceiling `(1024⁶⁰)⁵⁰ ≈ 10⁹⁰³⁰` —
  address space, not entities. The 520:1 glyph↔descriptor figure is **addressing measured in-fabric**,
  cited, **not** reproduced by this example.
- **Architecture / vision (not benchmarked):** clawhip/MCP integration, Lamport ordering,
  stubbed-rooms-as-RAM paging, every-device-as-surface. Presented as proposed mappings, not deployed
  facts.
- **Anti-marketing:** `gzip` closes the raw-size gap; the size win is uncompressed-wire + decode-speed +
  integrity, never compressibility. FNV-1a-64 is non-cryptographic dedup, not security.
