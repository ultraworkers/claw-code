<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Claw Code — Local Fork Overview</title>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
  <style>
    :root {
      --bg: #0b0f17;
      --surface: #111827;
      --surface-2: #1a2332;
      --border: #1f2937;
      --text: #e5e7eb;
      --text-muted: #9ca3af;
      --text-dim: #6b7280;
      --accent: #f59e0b;
      --accent-2: #d97706;
      --green: #10b981;
      --green-dim: #059669;
      --blue: #3b82f6;
      --red: #ef4444;
      --radius: 12px;
      --radius-sm: 8px;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Inter', system-ui, -apple-system, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.6;
      min-height: 100vh;
    }

    .container {
      max-width: 1100px;
      margin: 0 auto;
      padding: 0 24px;
    }

    /* HERO */
    .hero {
      padding: 80px 0 60px;
      text-align: center;
      border-bottom: 1px solid var(--border);
    }

    .hero-badge {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      background: var(--surface-2);
      border: 1px solid var(--border);
      padding: 6px 16px;
      border-radius: 999px;
      font-size: 0.85rem;
      color: var(--text-muted);
      margin-bottom: 24px;
    }

    .hero-badge .dot {
      width: 8px;
      height: 8px;
      background: var(--green);
      border-radius: 50%;
      display: inline-block;
      animation: pulse 2s infinite;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.4; }
    }

    .hero h1 {
      font-size: 3.2rem;
      font-weight: 800;
      letter-spacing: -0.03em;
      margin-bottom: 16px;
      background: linear-gradient(135deg, #fff 0%, var(--accent) 100%);
      background-clip: text;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }

    .hero p.lead {
      font-size: 1.25rem;
      color: var(--text-muted);
      max-width: 640px;
      margin: 0 auto 40px;
      font-weight: 300;
    }

    .hero-stats {
      display: flex;
      justify-content: center;
      gap: 48px;
      flex-wrap: wrap;
    }

    .stat {
      text-align: center;
    }

    .stat-value {
      font-size: 2.4rem;
      font-weight: 700;
      color: var(--accent);
      line-height: 1;
    }

    .stat-label {
      font-size: 0.85rem;
      color: var(--text-dim);
      margin-top: 6px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    /* COMPARISON */
    .comparison {
      padding: 60px 0;
      border-bottom: 1px solid var(--border);
    }

    .section-title {
      font-size: 1.75rem;
      font-weight: 700;
      margin-bottom: 8px;
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .section-subtitle {
      color: var(--text-muted);
      margin-bottom: 32px;
      font-size: 1rem;
    }

    .compare-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 24px;
    }

    @media (max-width: 768px) {
      .compare-grid { grid-template-columns: 1fr; }
      .hero h1 { font-size: 2.2rem; }
    }

    .compare-card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      padding: 28px;
      position: relative;
      overflow: hidden;
    }

    .compare-card::before {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 3px;
    }

    .compare-card.upstream::before { background: var(--blue); }
    .compare-card.local::before { background: var(--green); }

    .compare-card h3 {
      font-size: 1.1rem;
      font-weight: 600;
      margin-bottom: 4px;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .compare-card .tag {
      font-size: 0.7rem;
      padding: 2px 8px;
      border-radius: 999px;
      font-weight: 600;
      text-transform: uppercase;
    }

    .tag-blue { background: rgba(59,130,246,0.15); color: var(--blue); }
    .tag-green { background: rgba(16,185,129,0.15); color: var(--green); }

    .compare-card p.meta {
      font-size: 0.85rem;
      color: var(--text-dim);
      margin-bottom: 16px;
    }

    .compare-card ul {
      list-style: none;
      font-size: 0.9rem;
      color: var(--text-muted);
    }

    .compare-card ul li {
      padding: 6px 0;
      padding-left: 20px;
      position: relative;
    }

    .compare-card ul li::before {
      content: '—';
      position: absolute;
      left: 0;
      color: var(--text-dim);
    }

    .compare-card.local ul li::before {
      content: '+';
      color: var(--green);
      font-weight: 700;
    }

    /* DELTA BAR */
    .delta-bar {
      margin-top: 24px;
      background: var(--surface-2);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      padding: 16px 20px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 12px;
    }

    .delta-bar code {
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 0.85rem;
      color: var(--accent);
    }

    .delta-bar span {
      font-size: 0.9rem;
      color: var(--text-muted);
    }

    .delta-bar .highlight {
      color: var(--green);
      font-weight: 600;
    }

    /* ACCOMPLISHMENTS */
    .accomplishments {
      padding: 60px 0;
      border-bottom: 1px solid var(--border);
    }

    .milestone-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 20px;
    }

    .milestone-card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      padding: 24px;
      transition: border-color 0.2s, transform 0.2s;
    }

    .milestone-card:hover {
      border-color: var(--accent-2);
      transform: translateY(-2px);
    }

    .milestone-card .status {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      margin-bottom: 10px;
    }

    .status-done { color: var(--green); }
    .status-todo { color: var(--text-dim); }

    .milestone-card h4 {
      font-size: 1rem;
      font-weight: 600;
      margin-bottom: 8px;
      color: var(--text);
    }

    .milestone-card p {
      font-size: 0.875rem;
      color: var(--text-muted);
      line-height: 1.5;
    }

    .milestone-card .files {
      margin-top: 12px;
      font-size: 0.8rem;
      color: var(--text-dim);
      font-family: 'SF Mono', Monaco, monospace;
    }

    /* SUMMARY */
    .summary {
      padding: 60px 0 80px;
    }

    .summary-box {
      background: linear-gradient(135deg, var(--surface) 0%, var(--surface-2) 100%);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      padding: 40px;
      text-align: center;
    }

    .summary-box h2 {
      font-size: 1.5rem;
      font-weight: 700;
      margin-bottom: 16px;
    }

    .summary-box p {
      color: var(--text-muted);
      max-width: 600px;
      margin: 0 auto 24px;
    }

    .cta {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      background: var(--accent);
      color: var(--bg);
      padding: 12px 24px;
      border-radius: var(--radius-sm);
      font-weight: 600;
      font-size: 0.95rem;
      text-decoration: none;
      transition: background 0.2s;
    }

    .cta:hover {
      background: var(--accent-2);
    }

    footer {
      text-align: center;
      padding: 40px 0;
      border-top: 1px solid var(--border);
      color: var(--text-dim);
      font-size: 0.85rem;
    }

    footer a {
      color: var(--accent);
      text-decoration: none;
    }
  </style>
</head>
<body>

  <section class="hero">
    <div class="container">
      <div class="hero-badge">
        <span class="dot"></span>
        Active Development Fork
      </div>
      <h1>Claw Code Local</h1>
      <p class="lead">
        A hardened, feature-enriched fork of <code>ultraworkers/claw-code</code>
        with real diffing, atomic file operations, local LLM support, and
        a rigorous code-editing workflow board.
      </p>
      <div class="hero-stats">
        <div class="stat">
          <div class="stat-value">5</div>
          <div class="stat-label">Commits Ahead</div>
        </div>
        <div class="stat">
          <div class="stat-value">21</div>
          <div class="stat-label">User Stories Done</div>
        </div>
        <div class="stat">
          <div class="stat-value">7/14</div>
          <div class="stat-label">Milestones Complete</div>
        </div>
        <div class="stat">
          <div class="stat-value">900+</div>
          <div class="stat-label">Tests Passing</div>
        </div>
      </div>
    </div>
  </section>

  <section class="comparison">
    <div class="container">
      <h2 class="section-title">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s-8-4.5-8-11.8A8 8 0 0 1 12 2a8 8 0 0 1 8 8.2c0 7.3-8 11.8-8 11.8z"/><circle cx="12" cy="10" r="3"/></svg>
        Fork Comparison
      </h2>
      <p class="section-subtitle">What changed between the upstream repo and this local fork.</p>

      <div class="compare-grid">
        <div class="compare-card upstream">
          <h3>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--blue)" stroke-width="2"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>
            ultraworkers/claw-code
            <span class="tag tag-blue">Upstream</span>
          </h3>
          <p class="meta">1,043 commits &middot; 9-lane parity checkpoint &middot; 40 tool specs</p>
          <ul>
            <li>Bash validation, CI fix, file-tool edge cases</li>
            <li>TaskRegistry + Team/Cron registries (in-memory)</li>
            <li>MCP lifecycle bridge + LSP client dispatch</li>
            <li>Permission enforcement (read-only vs workspace-write)</li>
            <li>Mock parity harness with 10 scripted scenarios</li>
            <li>Basic <code>edit_file</code> with naive string replace</li>
            <li>No diff engine — degenerate hunks on every edit</li>
            <li>No atomic writes — direct <code>fs::write</code> calls</li>
            <li><code>/undo</code> registered but unimplemented (no-op)</li>
            <li>No read-before-edit tracking</li>
          </ul>
        </div>

        <div class="compare-card local">
          <h3>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>
            claw-code-local (this fork)
            <span class="tag tag-green">+5 commits</span>
          </h3>
          <p class="meta">All upstream features + local LLM + hardened editing pipeline</p>
          <ul>
            <li><strong>Embedded local LLM</strong> — llama.cpp provider with tool calling &amp; streaming</li>
            <li><strong>Real diff engine</strong> — <code>similar</code>-powered structured patches &amp; git-style unified diffs</li>
            <li><strong>Ambiguity guard</strong> — <code>edit_file</code> rejects non-unique matches unless <code>replace_all=true</code></li>
            <li><strong>Atomic writes</strong> — temp-file + <code>rename</code> + <code>fsync</code> + permission preservation</li>
            <li><strong>Encoding fidelity</strong> — CRLF, BOM, and trailing-newline round-trip preservation</li>
            <li><strong>SHA tracker</strong> — read-before-edit enforcement &amp; conflict detection</li>
            <li><strong>MultiEdit tool</strong> — atomic multi-hunk edits, all-or-nothing semantics</li>
            <li><strong>/undo works</strong> — per-session edit history, SHA-256 validation, <code>--force</code> override</li>
            <li><strong>Typed errors</strong> — structured <code>TypedError</code> envelopes across API + runtime + CLI</li>
            <li><strong>Model compatibility</strong> — Kimi, reasoning models, GPT-5, Qwen/DashScope fixes</li>
          </ul>
        </div>
      </div>

      <div class="delta-bar">
        <code>git diff upstream/main..HEAD</code>
        <span>
          <span class="highlight">+19,328</span> lines added &nbsp;&middot;&nbsp;
          <span class="highlight">-6,802</span> lines removed &nbsp;&middot;&nbsp;
          <span class="highlight">91</span> files changed
        </span>
      </div>
    </div>
  </section>

  <section class="accomplishments">
    <div class="container">
      <h2 class="section-title">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
        Accomplishments
      </h2>
      <p class="section-subtitle">Completed milestones and delivered user stories.</p>

      <div class="milestone-grid">
        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M1 — Real structuredPatch diff</h4>
          <p>Replaced degenerate hunks with real diffing via <code>similar::TextDiff</code>. Every edit now produces accurate <code>oldStart/oldLines/newStart/newLines</code> hunks and a populated <code>gitDiff</code> field.</p>
          <div class="files">diff.rs, file_ops.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M2 — edit_file uniqueness check</h4>
          <p>Ambiguous matches (2+ occurrences with <code>replace_all=false</code>) now return a typed <code>EditError::Ambiguous</code> instead of silently mutating the first hit.</p>
          <div class="files">file_ops.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M3 — Atomic write (tempfile + rename + fsync)</h4>
          <p>Crash-safe file edits via <code>NamedTempFile</code>, explicit <code>sync_all</code>, atomic <code>persist</code>, and Unix permission preservation. No more truncated files on power loss.</p>
          <div class="files">atomic_write.rs, file_ops.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M4 — CRLF / BOM / trailing-newline preservation</h4>
          <p>New <code>text_encoding.rs</code> detects <code>FileShape</code> (line ending, BOM, trailing newline), normalises to LF for the model, and restores the original shape on write.</p>
          <div class="files">text_encoding.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M5 — Read-before-edit tracking</h4>
          <p><code>FileTracker</code> records SHA-256 + mtime on every <code>read_file</code>. Subsequent edits are denied if the file was never read or was modified externally since the read.</p>
          <div class="files">file_tracker.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M6 — MultiEdit tool</h4>
          <p>Atomic multi-hunk edits. Applies N sequential <code>EditOp</code>s in-memory first; if any op fails, the file is never touched. One combined <code>structuredPatch</code> is returned on success.</p>
          <div class="files">tools/src/lib.rs, multi_edit.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M7 — /undo actually undoes</h4>
          <p>Per-session <code>EditHistory</code> stack (cap 50). <code>/undo</code> restores original bytes with SHA-256 validation. <code>/undo --force</code> skips the check. Stack cleared across sessions.</p>
          <div class="files">edit_history.rs, commands/src/lib.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>M9 — SHA conflict detection</h4>
          <p>Optional <code>expected_sha256</code> on <code>EditFileInput</code> / <code>WriteFileInput</code>. Mismatch yields <code>EditError::Conflict</code> with expected vs actual hashes, preventing stale overwrites.</p>
          <div class="files">tools/src/lib.rs, file_ops.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>Local LLM Provider (llama.cpp)</h4>
          <p>Embedded llama.cpp backend with tool calling, streaming, and thinking-budget control for reasoning models. Includes nightly CI workflow and container support.</p>
          <div class="files">llama_cpp.rs, nightly-local-llama.yml</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>Typed Error Envelope Contract</h4>
          <p><code>TypedError</code> with 9 kinds, structured fields (<code>kind, operation, target, detail, hint, retryable</code>), JSON + text rendering, and downcasting from <code>ApiError</code> / <code>SessionControlError</code>.</p>
          <div class="files">typed_error.rs, main.rs</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>Model Compatibility Hardening</h4>
          <p>Kimi <code>is_error</code> exclusion, reasoning-model tuning stripping, GPT-5 <code>max_completion_tokens</code>, Qwen/DashScope routing, and request-body size pre-flight checks.</p>
          <div class="files">openai_compat.rs, MODEL_COMPATIBILITY.md</div>
        </div>

        <div class="milestone-card">
          <div class="status status-done"><span>●</span> Done</div>
          <h4>Performance Benchmarks</h4>
          <p>Criterion benchmark suite for request-building hot paths. <code>flatten_tool_result_content</code> optimised with pre-allocated capacity — ~17 ns single-text, ~11.7 µs large-content.</p>
          <div class="files">benches/request_building.rs</div>
        </div>
      </div>
    </div>
  </section>

  <section class="summary">
    <div class="container">
      <div class="summary-box">
        <h2>What This Fork Demonstrates</h2>
        <p>
          Beyond the original <strong>9-lane parity checkpoint</strong>, this fork proves that a small,
          focused set of incremental milestones can transform a stub-heavy codebase into a
          production-resilient editing harness — with real diffs, atomic safety, encoding fidelity,
          and undo — all validated by 900+ tests and a deterministic mock parity harness.
        </p>
        <a class="cta" href="https://github.com/ultraworkers/claw-code" target="_blank">
          View Upstream Repo
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
        </a>
      </div>
    </div>
  </section>

  <footer>
    <div class="container">
      Generated from <code>claw-code-local</code> workspace &middot;
      Comparing against <a href="https://github.com/ultraworkers/claw-code">github.com/ultraworkers/claw-code</a>
    </div>
  </footer>

</body>
</html>
