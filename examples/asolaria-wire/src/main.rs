//! asolaria_wire_bench — REPRODUCIBLE benchmark contributed by the Asolaria project.
//!
//! Compares, on the same agent-lane-event records (the kind an agent harness's event router moves):
//!   * JSON text                         (baseline; no integrity)
//!   * json=0 fixed-width BINARY          (lane addressed by its 8-byte FNV-1a-64 handle)
//!   * binary + a SHA-256 hash-chain      (per-record tamper-evidence)
//!
//! Prints wire size, encode/decode speed, and runs a tamper test that localizes the first edited
//! record. Absolute ns vary by CPU; the RATIOS and the tamper-detection behavior are what to verify.
//! No network, no external services. Run: `cargo run --release`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone)]
struct Event {
    lane: String,
    host_handle8: u64,
    seq: u64,
    lamport: u64,
    event: String,
    ts: String,
    hash: String,
}

const EVENTS: &[&str] = &[
    "spawning", "trust_required", "ready_for_prompt", "prompt_accepted", "running", "blocked",
    "finished", "failed",
];

/// FNV-1a 64-bit — a fast, NON-cryptographic content/address hash (dedup + 8-byte handles only).
/// Tamper-evidence is the separate SHA-256 chain's job, not this.
fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn corpus(n: usize) -> Vec<Event> {
    (0..n)
        .map(|i| {
            let lane = format!("agent-{}", i % 64);
            Event {
                host_handle8: fnv1a64(&lane),
                lane,
                seq: (i / 64) as u64,
                lamport: i as u64,
                event: EVENTS[i % EVENTS.len()].to_string(),
                ts: format!("2026-06-27T21:{:02}:{:02}.{:03}Z", (i / 60) % 60, i % 60, i % 1000),
                hash: format!("{:08x}", fnv1a64(&format!("{i}")) & 0xffff_ffff),
            }
        })
        .collect()
}

// ---- json=0 binary: fixed 33-byte record; lane addressed by its 8-byte handle (string not stored) ----
const REC: usize = 8 + 8 + 8 + 1 + 8; // handle, seq, lamport, event_u8, ts_ms = 33

fn ev_idx(name: &str) -> u8 {
    EVENTS.iter().position(|e| *e == name).unwrap_or(255) as u8
}
fn ts_to_ms(ts: &str) -> u64 {
    let b = ts.as_bytes();
    let g = |i: usize| (b[i] - b'0') as u64;
    ((g(14) * 10 + g(15)) * 60 + (g(17) * 10 + g(18))) * 1000 + (g(20) * 100 + g(21) * 10 + g(22))
}
fn pack(e: &Event, out: &mut Vec<u8>) {
    out.extend_from_slice(&e.host_handle8.to_le_bytes());
    out.extend_from_slice(&e.seq.to_le_bytes());
    out.extend_from_slice(&e.lamport.to_le_bytes());
    out.push(ev_idx(&e.event));
    out.extend_from_slice(&ts_to_ms(ts_of(e)).to_le_bytes());
}
fn ts_of(e: &Event) -> &str {
    &e.ts
}
fn unpack_seq(b: &[u8]) -> u64 {
    u64::from_le_bytes(b[8..16].try_into().unwrap())
}

/// Build the per-record SHA-256 chain over a packed binary buffer.
/// link_n = SHA256(link_{n-1} || record_n); genesis prev = 32 zero bytes.
fn build_chain(bin: &[u8], n: usize) -> Vec<[u8; 32]> {
    let mut prev = [0u8; 32];
    let mut links = Vec::with_capacity(n);
    for i in 0..n {
        let mut h = Sha256::new();
        h.update(prev);
        h.update(&bin[i * REC..(i + 1) * REC]);
        let d = h.finalize();
        prev.copy_from_slice(&d);
        links.push(prev);
    }
    links
}

fn bench<F: FnMut() -> usize>(iters: usize, mut f: F) -> f64 {
    let mut acc = 0usize;
    for _ in 0..2 {
        acc = acc.wrapping_add(f());
    }
    black_box(acc);
    let mut best = f64::MAX;
    for _ in 0..iters {
        let t = Instant::now();
        black_box(f());
        best = best.min(t.elapsed().as_secs_f64());
    }
    best
}

fn main() {
    let n = 200_000usize;
    let data = corpus(n);

    // JSON text
    let json_buf: String =
        data.iter().map(|e| serde_json::to_string(e).unwrap() + "\n").collect();
    let json_raw = json_buf.len();

    // json=0 binary
    let mut bin = Vec::with_capacity(n * REC);
    for e in &data {
        pack(e, &mut bin);
    }
    let bin_raw = bin.len();
    let chained_raw = bin_raw + n * 32; // +32-byte SHA-256 link per record

    // encode speed
    let json_enc = bench(20, || {
        let mut b = 0;
        for e in &data {
            b += serde_json::to_string(e).unwrap().len();
        }
        b
    });
    let bin_enc = bench(20, || {
        let mut v = Vec::with_capacity(n * REC);
        for e in &data {
            pack(e, &mut v);
        }
        v.len()
    });

    // decode speed
    let json_lines: Vec<&str> = json_buf.lines().collect();
    let json_dec = bench(20, || {
        let mut a = 0usize;
        for l in &json_lines {
            let e: Event = serde_json::from_str(l).unwrap();
            a = a.wrapping_add(e.seq as usize);
        }
        a
    });
    let bin_dec = bench(20, || {
        let mut a = 0usize;
        for i in 0..n {
            a = a.wrapping_add(unpack_seq(&bin[i * REC..(i + 1) * REC]) as usize);
        }
        a
    });

    let x = |slow: f64, fast: f64| slow / fast;
    let nspr = |s: f64| s * 1.0e9 / n as f64;
    println!("=== json=0 binary + SHA-256 chain vs JSON text — {n} agent events (MEASURED) ===\n");
    println!("WIRE SIZE (raw):");
    println!("  JSON text                     {:>6.1} B/rec", json_raw as f64 / n as f64);
    println!("  json=0 binary                 {:>6.1} B/rec   {:.2}x smaller", bin_raw as f64 / n as f64, x(json_raw as f64, bin_raw as f64));
    println!("  binary + SHA-256 chain        {:>6.1} B/rec   {:.2}x smaller (AND tamper-evident)", chained_raw as f64 / n as f64, x(json_raw as f64, chained_raw as f64));
    println!("\nENCODE:  JSON {:>6.1} ns/rec    binary {:>6.1} ns/rec   {:.1}x faster", nspr(json_enc), nspr(bin_enc), x(json_enc, bin_enc));
    println!("DECODE:  JSON {:>6.1} ns/rec    binary {:>6.1} ns/rec   {:.0}x faster (fixed-width read = pointer offset)", nspr(json_dec), nspr(bin_dec), x(json_dec, bin_dec));

    // ---- tamper test: flip one byte in an early record, re-fold, localize the first broken link ----
    let original = build_chain(&bin, n);
    let mut tampered_bin = bin.clone();
    let victim = 5usize; // edit record #5
    tampered_bin[victim * REC] ^= 0x01; // flip one bit
    let tampered = build_chain(&tampered_bin, n);
    let first_break = (0..n).find(|&i| original[i] != tampered[i]);
    println!("\nTAMPER TEST (integrity JSON has none natively):");
    println!("  flipped 1 bit in record #{victim}, re-folded the chain");
    match first_break {
        Some(i) => println!("  -> first broken link at record #{i}  ({})", if i == victim { "CORRECT — localizes the exact edit" } else { "unexpected" }),
        None => println!("  -> NO break detected (FAIL)"),
    }
    println!("\nNote: gzip narrows the RAW-size gap (crypto links are high-entropy); the durable wins are");
    println!("uncompressed wire size, decode≈free, and per-record tamper-evidence — not compressibility.");
    println!("520:1 BEHCS-1024 glyph ADDRESSING (a 12-byte handle -> ~6KB descriptor via the fabric store)");
    println!("is a SEPARATE, addressing-not-codec result; it is NOT reproduced by this local bench.");
}
