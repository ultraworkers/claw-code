// claw-web — vanilla JS frontend.
//
// State is kept in a single object; every mutation calls render(). Trades
// performance for legibility (the UI is small enough that the cost is
// invisible). Replace with a real reactive layer if it ever stops scaling.

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

// Path selector: ?ui=legacy (default for now while we iterate) or ?ui=term
// (the phase-2 tmux+xterm path). Once the terminal path is verified, term
// becomes the default and legacy goes away.
const UI_MODE = (new URLSearchParams(location.search).get('ui') || 'term').toLowerCase();

const state = {
  mode: '…',
  health: null,
  projects: [],
  selectedProjectId: null,
  sessions: [],
  selectedSessionId: null,
  // Legacy (subprocess-per-turn) state
  conversation: [],     // [{role:'user'|'assistant'|'system', body, ts}]
  ws: null,
  wsStatus: 'disconnected',
  wsTurnInProgress: false,
  // Terminal-path state
  term: null,           // xterm.js Terminal instance
  termFit: null,        // FitAddon instance
  termWS: null,         // terminal-mode WebSocket
  termStatus: 'disconnected',
};

// ── API helpers ─────────────────────────────────────────────────────────
async function api(method, path, body) {
  const res = await fetch(path, {
    method,
    headers: body ? { 'content-type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const detail = await res.text();
    throw new Error(`${method} ${path} → ${res.status}: ${detail}`);
  }
  return res.json();
}

// ── Render ──────────────────────────────────────────────────────────────
function renderModeBadge() {
  const el = $('#mode-badge');
  el.textContent = state.mode;
  el.style.color =
    state.mode === 'subprocess' ? 'var(--ok)' :
    state.mode === 'mock'       ? 'var(--warn)' :
                                  'var(--fg-muted)';
}

function renderHealthLine() {
  const h = state.health;
  if (!h) { $('#health-line').textContent = 'connecting…'; return; }
  $('#health-line').textContent = `${h.mode} · ${h.claw_available ? 'claw ✓' : 'claw ✗'}`;
}

function renderProjects() {
  const ul = $('#project-list');
  ul.innerHTML = '';
  if (state.projects.length === 0) {
    const li = document.createElement('li');
    li.className = 'muted';
    li.style.fontStyle = 'italic';
    li.textContent = 'No projects yet — click +';
    ul.appendChild(li);
    return;
  }
  for (const p of state.projects) {
    const li = document.createElement('li');
    if (p.id === state.selectedProjectId) li.classList.add('active');
    li.innerHTML = `
      <span>${escapeHTML(p.name)}</span>
      <span class="meta">${escapeHTML(shortenPath(p.path))}</span>
    `;
    li.addEventListener('click', () => selectProject(p.id));
    ul.appendChild(li);
  }
}

function renderSessions() {
  const section = $('#sessions-section');
  if (!state.selectedProjectId) {
    section.hidden = true;
    return;
  }
  section.hidden = false;
  const proj = state.projects.find((p) => p.id === state.selectedProjectId);
  $('#sessions-heading').textContent =
    `Sessions — ${proj ? proj.name : ''}`;

  const ul = $('#session-list');
  ul.innerHTML = '';
  if (state.sessions.length === 0) {
    const li = document.createElement('li');
    li.className = 'muted';
    li.style.fontStyle = 'italic';
    li.textContent = 'No sessions — click +';
    ul.appendChild(li);
    return;
  }
  for (const s of state.sessions) {
    const li = document.createElement('li');
    if (s.id === state.selectedSessionId) li.classList.add('active');
    const claw = s.claw_session_id ? `· ${s.claw_session_id.slice(0, 8)}` : '';
    li.innerHTML = `
      <span>${escapeHTML(s.title)}</span>
      <span class="meta">${formatRelative(s.updated_at)} ${claw}</span>
    `;
    li.addEventListener('click', () => selectSession(s.id));
    ul.appendChild(li);
  }
}

function renderCrumbs() {
  const proj = state.projects.find((p) => p.id === state.selectedProjectId);
  const sess = state.sessions.find((s) => s.id === state.selectedSessionId);
  const cp = $('#crumb-project');
  const cs = $('#crumb-session');
  if (proj) {
    cp.textContent = proj.name;
    cp.classList.remove('crumb-empty');
  } else {
    cp.textContent = 'No project';
    cp.classList.add('crumb-empty');
  }
  if (sess) {
    cs.textContent = sess.title;
    cs.classList.remove('crumb-empty');
  } else {
    cs.textContent = 'No session';
    cs.classList.add('crumb-empty');
  }
}

function renderConversation() {
  const conv = $('#conversation');
  const empty = $('#empty-state');
  if (!state.selectedSessionId) {
    empty.hidden = false;
    conv.querySelectorAll('.turn').forEach((n) => n.remove());
    return;
  }
  empty.hidden = true;
  conv.querySelectorAll('.turn').forEach((n) => n.remove());
  for (const t of state.conversation) {
    const div = document.createElement('div');
    div.className = `turn ${t.role}`;
    const role =
      t.role === 'user' ? 'you'
      : t.role === 'assistant' ? 'claw'
      : t.role === 'pending' ? 'claw'
      : t.role === 'pending-approval' ? 'claw — awaiting approval'
      : t.role === 'error' ? 'error'
      : t.role;
    div.innerHTML = `
      <div class="role">${escapeHTML(role)}</div>
      <div class="body">${escapeHTML(t.body || '')}</div>
      ${t.status ? `<div class="status-line">${escapeHTML(t.status)}</div>` : ''}
    `;
    conv.appendChild(div);
  }
  // Auto-scroll to bottom on each render — newest at the foot.
  conv.scrollTop = conv.scrollHeight;
}

function renderComposer() {
  const form = $('#composer');
  form.hidden = !state.selectedSessionId;
  const btn = $('#send-btn');
  btn.disabled = !state.selectedSessionId || state.wsTurnInProgress;
  const abortBtn = $('#abort-btn');
  if (abortBtn) abortBtn.hidden = !state.wsTurnInProgress;
  $('#ws-status').textContent =
    state.wsStatus + (state.wsTurnInProgress ? ' · turn in progress' : '');
}

function renderTermStatus() {
  const el = document.querySelector('#term-status');
  if (el) el.textContent = state.termStatus;
}

function renderPanelVisibility() {
  // Single source of truth: UI_MODE picks which panel/composer is visible.
  const conv = document.querySelector('#conversation');
  const composer = document.querySelector('#composer');
  const termPanel = document.querySelector('#terminal-panel');
  const termHost = document.querySelector('#terminal');
  const termBar = document.querySelector('#terminal-bar');
  const termEmpty = document.querySelector('#terminal-empty');
  if (UI_MODE === 'term') {
    if (conv) conv.hidden = true;
    if (composer) composer.hidden = true;
    if (termPanel) termPanel.hidden = false;
    if (termHost) termHost.hidden = !state.selectedSessionId;
    if (termBar) termBar.hidden = !state.selectedSessionId;
    if (termEmpty) termEmpty.hidden = !!state.selectedSessionId;
  } else {
    if (termPanel) termPanel.hidden = true;
    if (conv) conv.hidden = false;
  }
}

function render() {
  renderModeBadge();
  renderHealthLine();
  renderProjects();
  renderSessions();
  renderCrumbs();
  renderPanelVisibility();
  if (UI_MODE === 'term') {
    renderTermStatus();
  } else {
    renderConversation();
    renderComposer();
  }
}

// ── Selection / loading ─────────────────────────────────────────────────
async function refreshHealth() {
  state.health = await api('GET', '/api/health');
  state.mode = state.health.mode;
}

async function refreshProjects() {
  const r = await api('GET', '/api/projects');
  state.projects = r.projects;
}

async function refreshSessions() {
  if (!state.selectedProjectId) { state.sessions = []; return; }
  const r = await api('GET', `/api/projects/${state.selectedProjectId}/sessions`);
  state.sessions = r.sessions;
}

async function selectProject(id) {
  state.selectedProjectId = id;
  state.selectedSessionId = null;
  state.conversation = [];
  await refreshSessions();
  closeWS();
  render();
}

async function selectSession(id) {
  state.selectedSessionId = id;
  state.conversation = [];
  closeWS();
  closeTerm();
  render();
  if (UI_MODE === 'term') {
    openTerm(id);
  } else {
    openWS(id);
  }
}

// ── Terminal-path WS + xterm.js ─────────────────────────────────────────
function openTerm(sessionId) {
  const empty = document.querySelector('#terminal-empty');
  const termHost = document.querySelector('#terminal');
  const termBar = document.querySelector('#terminal-bar');
  if (empty) empty.hidden = true;
  if (termHost) termHost.hidden = false;
  if (termBar) termBar.hidden = false;

  if (!state.term) {
    if (typeof Terminal === 'undefined') {
      console.error('xterm.js failed to load');
      state.termStatus = 'xterm-load-failed';
      render();
      return;
    }
    state.term = new Terminal({
      fontFamily: 'ui-monospace, Menlo, monospace',
      fontSize: 13,
      theme: { background: '#000000' },
      cursorBlink: true,
      scrollback: 5000,
      convertEol: true,
    });
    if (typeof FitAddon !== 'undefined' && FitAddon.FitAddon) {
      state.termFit = new FitAddon.FitAddon();
      state.term.loadAddon(state.termFit);
    }
    state.term.open(termHost);
    if (state.termFit) state.termFit.fit();
    // ResizeObserver keeps xterm sized + tells the server to resize the PTY.
    const ro = new ResizeObserver(() => {
      if (!state.termFit) return;
      try { state.termFit.fit(); } catch {}
      sendTermResize();
    });
    ro.observe(termHost);

    state.term.onData((data) => {
      if (state.termWS && state.termWS.readyState === WebSocket.OPEN) {
        state.termWS.send(JSON.stringify({ type: 'input', data }));
      }
    });
  }

  state.term.clear();
  state.term.write('\x1b[90mconnecting…\x1b[0m\r\n');

  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}/api/sessions/${sessionId}/terminal`;
  const ws = new WebSocket(url);
  state.termWS = ws;
  state.termStatus = 'connecting';
  render();

  ws.addEventListener('open', () => {
    state.termStatus = 'connected';
    render();
    // The terminal-panel just became visible (the [hidden] toggle ran in
    // render()), so the FitAddon's first measurement may have been zero.
    // Re-fit on the next frame, then once more after a beat to cover
    // late layout (e.g. scrollbar appearing). Then send the resize.
    requestAnimationFrame(() => {
      try { state.termFit && state.termFit.fit(); } catch {}
      sendTermResize();
      setTimeout(() => {
        try { state.termFit && state.termFit.fit(); } catch {}
        sendTermResize();
      }, 150);
    });
  });
  ws.addEventListener('close', () => {
    state.termStatus = 'disconnected';
    render();
    if (state.term) state.term.write('\r\n\x1b[90m[disconnected]\x1b[0m\r\n');
  });
  ws.addEventListener('error', () => {
    state.termStatus = 'error';
    render();
  });
  ws.addEventListener('message', (ev) => {
    let msg;
    try { msg = JSON.parse(ev.data); } catch { return; }
    if (msg.type === 'output' && state.term) {
      state.term.write(msg.data);
    } else if (msg.type === 'status') {
      // Surface lifecycle events as dim banners inside the terminal.
      if (state.term) {
        state.term.write(
          `\r\n\x1b[90m[${msg.state || 'status'}${msg.message ? ': ' + msg.message : ''}]\x1b[0m\r\n`
        );
      }
      state.termStatus = msg.state || state.termStatus;
      render();
    }
  });
}

function sendTermResize() {
  if (!state.term || !state.termWS || state.termWS.readyState !== WebSocket.OPEN) return;
  state.termWS.send(JSON.stringify({
    type: 'resize',
    cols: state.term.cols,
    rows: state.term.rows,
  }));
}

function closeTerm() {
  if (state.termWS) {
    try { state.termWS.close(); } catch {}
    state.termWS = null;
  }
  state.termStatus = 'disconnected';
}

// ── WebSocket ───────────────────────────────────────────────────────────
function openWS(sessionId) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}/api/sessions/${sessionId}/stream`;
  const ws = new WebSocket(url);
  state.ws = ws;
  state.wsStatus = 'connecting';
  render();

  ws.addEventListener('open', () => { state.wsStatus = 'connected'; render(); });
  ws.addEventListener('close', () => { state.wsStatus = 'disconnected'; state.wsTurnInProgress = false; render(); });
  ws.addEventListener('error', () => { state.wsStatus = 'error'; render(); });
  ws.addEventListener('message', (ev) => {
    let msg;
    try { msg = JSON.parse(ev.data); } catch { return; }
    handleWSMessage(msg);
  });
}

function closeWS() {
  if (state.ws) {
    try { state.ws.close(); } catch {}
    state.ws = null;
  }
  state.wsStatus = 'disconnected';
  state.wsTurnInProgress = false;
}

function handleWSMessage({ kind, payload }) {
  switch (kind) {
    case 'ready':
      // payload.session, payload.project, payload.mode
      break;
    case 'status':
      // Attach to the in-progress assistant turn or create one. Compose a
      // human line from `message`, `cmd`, and (when present) the env
      // fingerprint so routing failures are debuggable in-UI.
      attachStatusToCurrentAssistantTurn(formatStatus(payload));
      break;
    case 'approval_request':
      showApprovalDialog(payload);
      break;
    case 'final':
      state.wsTurnInProgress = false;
      {
        const hasText = !!payload.text;
        let body = payload.text || '(empty response)';
        let status;
        if (payload.mock) status = 'mock';
        else if (payload.raw) status = 'raw stdout (no JSON parse)';
        else if (!hasText && payload.debug_keys)
          status = `no text field; got keys: ${payload.debug_keys.join(', ') || '(none)'}`;
        else status = 'ok';
        // When parsing got JSON but no text, dump the blob into the body
        // so we can read it and pin the field name.
        if (!hasText && payload.blob && !payload.mock) {
          body = '(empty response)\n\n' + JSON.stringify(payload.blob, null, 2);
        }
        replaceCurrentAssistantTurn({ role: 'assistant', body, status });
      }
      refreshSessions().then(render);
      break;
    case 'error':
      state.wsTurnInProgress = false;
      replaceCurrentAssistantTurn({
        role: 'error',
        body: payload.message || 'unknown error',
        status: payload.stderr ? `stderr: ${payload.stderr.slice(0,200)}` : '',
      });
      break;
  }
  render();
}

function formatStatus(payload) {
  if (!payload) return '';
  const parts = [];
  if (payload.message) parts.push(payload.message);
  if (payload.cmd) parts.push(`cmd=${payload.cmd}`);
  if (payload.env) {
    const e = payload.env;
    const route = e.OPENAI_BASE_URL ? `openai→${e.OPENAI_BASE_URL}` : 'anthropic';
    const oa = e.OPENAI_API_KEY?.set ? `oa(${e.OPENAI_API_KEY.len})` : 'no-oa';
    const an = (e.ANTHROPIC_API_KEY?.set || e.ANTHROPIC_AUTH_TOKEN?.set) ? 'an' : 'no-an';
    parts.push(`route=${route} ${oa} ${an}`);
  }
  if (payload.chunks !== undefined) {
    parts.push(`chunks=${payload.chunks} bytes=${payload.bytes}`);
  }
  if (payload.preview) {
    parts.push(`«${payload.preview}»`);
  }
  return parts.join(' · ');
}

function showApprovalDialog(req) {
  // Mark the in-progress assistant turn as awaiting approval — gives the
  // user an unmistakable visual signal that the turn is paused.
  const last = state.conversation[state.conversation.length - 1];
  if (last && (last.role === 'pending' || last.role === 'assistant')) {
    last.role = 'pending-approval';
    last.status = `awaiting approval: ${req.tool} (${req.required_mode})`;
    render();
  }
  // Populate and open the dialog.
  $('#approval-tool').textContent = req.tool || '?';
  $('#approval-current').textContent = req.current_mode || '?';
  $('#approval-required').textContent = req.required_mode || '?';
  $('#approval-reason').textContent = req.reason || '';
  // Pretty-print the input if it's JSON-shaped, else show as-is.
  let inputText = req.input || '';
  try {
    inputText = JSON.stringify(JSON.parse(inputText), null, 2);
  } catch {}
  $('#approval-input').textContent = inputText;

  const dlg = $('#approval-dialog');
  if (!dlg.open) dlg.showModal();
}

function attachStatusToCurrentAssistantTurn(status) {
  const last = state.conversation[state.conversation.length - 1];
  if (last && (last.role === 'assistant' || last.role === 'pending' || last.role === 'pending-approval')) {
    last.status = status;
  } else {
    state.conversation.push({ role: 'pending', body: '…', status });
  }
}

function replaceCurrentAssistantTurn(turn) {
  const last = state.conversation[state.conversation.length - 1];
  if (last && (last.role === 'pending' || last.role === 'assistant')) {
    state.conversation[state.conversation.length - 1] = turn;
  } else {
    state.conversation.push(turn);
  }
}

// ── Compose / send ──────────────────────────────────────────────────────
function sendPrompt(text) {
  if (!state.ws || state.ws.readyState !== WebSocket.OPEN) {
    alert('Not connected. Pick a session first.');
    return;
  }
  state.conversation.push({ role: 'user', body: text });
  state.conversation.push({ role: 'pending', body: '…', status: 'queued' });
  state.wsTurnInProgress = true;
  render();
  state.ws.send(JSON.stringify({ prompt: text }));
}

// ── Dialogs ─────────────────────────────────────────────────────────────
function setupNewProjectDialog() {
  const dlg = $('#new-project-dialog');
  $('#new-project-btn').addEventListener('click', () => dlg.showModal());
  dlg.addEventListener('close', async () => {
    if (dlg.returnValue !== 'create') return;
    const fd = new FormData($('#new-project-form'));
    try {
      const proj = await api('POST', '/api/projects', {
        name: fd.get('name'),
        path: fd.get('path'),
      });
      await refreshProjects();
      await selectProject(proj.id);
    } catch (e) {
      alert(`Could not create project: ${e.message}`);
    }
    $('#new-project-form').reset();
  });
}

function setupApprovalDialog() {
  const dlg = $('#approval-dialog');
  dlg.addEventListener('close', () => {
    const approve = dlg.returnValue === 'allow';
    if (state.ws && state.ws.readyState === WebSocket.OPEN) {
      state.ws.send(JSON.stringify({
        kind: 'approval_response',
        approve,
      }));
    }
    // Bring the in-progress turn back to the normal pending state.
    const last = state.conversation[state.conversation.length - 1];
    if (last && last.role === 'pending-approval') {
      last.role = 'pending';
      last.status = approve ? 'approved · resuming…' : 'denied · resuming…';
      render();
    }
  });
}

function setupNewSessionDialog() {
  const dlg = $('#new-session-dialog');
  $('#new-session-btn').addEventListener('click', () => {
    if (!state.selectedProjectId) return;
    dlg.showModal();
  });
  dlg.addEventListener('close', async () => {
    if (dlg.returnValue !== 'create') return;
    const fd = new FormData($('#new-session-form'));
    try {
      const sess = await api(
        'POST',
        `/api/projects/${state.selectedProjectId}/sessions`,
        { title: fd.get('title') },
      );
      await refreshSessions();
      await selectSession(sess.id);
    } catch (e) {
      alert(`Could not create session: ${e.message}`);
    }
    $('#new-session-form').reset();
  });
}

// ── Composer wiring ─────────────────────────────────────────────────────
function setupComposer() {
  const form = $('#composer');
  const ta = $('#prompt-input');

  ta.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      form.requestSubmit();
    }
  });

  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const text = ta.value.trim();
    if (!text) return;
    if (state.wsTurnInProgress) return;
    sendPrompt(text);
    ta.value = '';
  });

  $('#abort-btn').addEventListener('click', () => {
    if (!state.wsTurnInProgress) return;
    // Closing the WS makes the server cancel the turn (subprocess killed
    // via task cancellation). Reopen on the same session immediately.
    const sid = state.selectedSessionId;
    closeWS();
    // Mark current turn as aborted in the conversation.
    const last = state.conversation[state.conversation.length - 1];
    if (last && (last.role === 'pending' || last.role === 'pending-approval')) {
      state.conversation[state.conversation.length - 1] = {
        role: 'error',
        body: 'Turn aborted by user.',
        status: 'aborted',
      };
    }
    state.wsTurnInProgress = false;
    render();
    if (sid) openWS(sid);
  });
}

// ── Utilities ───────────────────────────────────────────────────────────
function escapeHTML(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[c]);
}

function shortenPath(p) {
  if (!p) return '';
  const home = '/home/';
  const segs = p.split('/');
  if (segs.length <= 4) return p;
  return `…/${segs.slice(-3).join('/')}`;
}

function formatRelative(ms) {
  if (!ms) return '';
  const diff = Date.now() - ms;
  const m = Math.floor(diff / 60_000);
  if (m < 1) return 'just now';
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.floor(h / 24);
  return `${d}d ago`;
}

// ── Boot ────────────────────────────────────────────────────────────────
function setupKillTermBtn() {
  const btn = document.querySelector('#kill-term-btn');
  if (!btn) return;
  btn.addEventListener('click', async () => {
    if (!state.selectedSessionId) return;
    if (!confirm('Kill the tmux session? This terminates claw and loses any in-flight work.')) return;
    try {
      await fetch(`/api/sessions/${state.selectedSessionId}/terminal`, { method: 'DELETE' });
    } catch (e) {
      console.error(e);
    }
    closeTerm();
    if (state.term) state.term.write('\r\n\x1b[31m[session killed]\x1b[0m\r\n');
    // Reopen so the user can start a fresh REPL.
    if (state.selectedSessionId) openTerm(state.selectedSessionId);
  });
}

async function boot() {
  setupNewProjectDialog();
  setupNewSessionDialog();
  setupApprovalDialog();
  setupComposer();
  setupKillTermBtn();
  try {
    await refreshHealth();
    await refreshProjects();
  } catch (e) {
    console.error(e);
    state.wsStatus = 'health-error';
  }
  render();
}

boot();
