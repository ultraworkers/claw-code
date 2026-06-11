/**
 * Dashboard Page
 * System stats cards, quick actions, and overview
 */

class Dashboard {
  constructor(container) {
    this.container = container;
    this.stats = {
      documents: 0,
      rules: 0,
      projects: 0,
      failures: 0
    };
    this.render();
    this.loadData();
  }

  async render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">Dashboard</h1>
            <p class="page-description">Overview of your guardrail system</p>
          </div>
          <div class="page-actions">
            <button class="btn btn-secondary" id="refresh-btn">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="23 4 23 10 17 10"/>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
              </svg>
              Refresh
            </button>
          </div>
        </div>

        <div class="stats-grid">
          <div class="stat-card" data-stat="documents">
            <div class="stat-label">Documents</div>
            <div class="stat-value">${this.stats.documents}</div>
            <div class="stat-change">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
                <line x1="16" y1="13" x2="8" y2="13"/>
                <line x1="16" y1="17" x2="8" y2="17"/>
                <polyline points="10 9 9 9 8 9"/>
              </svg>
              Knowledge base
            </div>
          </div>

          <div class="stat-card" data-stat="rules">
            <div class="stat-label">Prevention Rules</div>
            <div class="stat-value">${this.stats.rules}</div>
            <div class="stat-change">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
              Active guardrails
            </div>
          </div>

          <div class="stat-card" data-stat="projects">
            <div class="stat-label">Projects</div>
            <div class="stat-value">${this.stats.projects}</div>
            <div class="stat-change">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
              </svg>
              Configured
            </div>
          </div>

          <div class="stat-card" data-stat="failures">
            <div class="stat-label">Active Failures</div>
            <div class="stat-value" style="color: var(--color-error);">${this.stats.failures}</div>
            <div class="stat-change">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/>
                <line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
              Need attention
            </div>
          </div>
        </div>

        <div class="content-section" style="margin-top: var(--space-8);">
          <div class="section-header">
            <h2 class="section-title">Quick Actions</h2>
          </div>
          <div class="card-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--space-4);">
            <div class="card">
              <div class="card-body">
                <h3 style="font-size: var(--text-base); font-weight: var(--font-semibold); margin-bottom: var(--space-2);">
                  Ingest Documents
                </h3>
                <p style="font-size: var(--text-sm); color: var(--color-text-secondary); margin-bottom: var(--space-4);">
                  Scan and index all markdown files from the repository.
                </p>
                <button class="btn btn-primary" id="ingest-btn">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="17 8 12 3 7 8"/>
                    <line x1="12" y1="3" x2="12" y2="15"/>
                  </svg>
                  Start Ingestion
                </button>
              </div>
            </div>

            <div class="card">
              <div class="card-body">
                <h3 style="font-size: var(--text-base); font-weight: var(--font-semibold); margin-bottom: var(--space-2);">
                  Validate Code
                </h3>
                <p style="font-size: var(--text-sm); color: var(--color-text-secondary); margin-bottom: var(--space-4);">
                  Test code snippets against prevention rules.
                </p>
                <a href="#/ide-tools" class="btn btn-secondary">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="16 18 22 12 16 6"/>
                    <polyline points="8 6 2 12 8 18"/>
                  </svg>
                  Open IDE Tools
                </a>
              </div>
            </div>

            <div class="card">
              <div class="card-body">
                <h3 style="font-size: var(--text-base); font-weight: var(--font-semibold); margin-bottom: var(--space-2);">
                  View Documentation
                </h3>
                <p style="font-size: var(--text-sm); color: var(--color-text-secondary); margin-bottom: var(--space-4);">
                  Browse and search the knowledge base.
                </p>
                <a href="#/documents" class="btn btn-secondary">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="m21 21-4.35-4.35"/>
                  </svg>
                  Browse Documents
                </a>
              </div>
            </div>
          </div>
        </div>

        <div class="content-section" style="margin-top: var(--space-8);">
          <div class="section-header">
            <h2 class="section-title">System Status</h2>
          </div>
          <div class="card">
            <div class="card-body" id="system-status">
              <div class="loading-state" style="padding: var(--space-8);">
                <div class="spinner"></div>
                <p class="loading-text">Checking system status...</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    const refreshBtn = this.container.querySelector('#refresh-btn');
    if (refreshBtn) {
      refreshBtn.addEventListener('click', () => this.loadData());
    }

    const ingestBtn = this.container.querySelector('#ingest-btn');
    if (ingestBtn) {
      ingestBtn.addEventListener('click', () => this.handleIngest());
    }
  }

  async loadData() {
    try {
      // Load stats
      const stats = await window.api.getStats();
      this.stats = {
        documents: stats.documents_count || 0,
        rules: stats.rules_count || 0,
        projects: stats.projects_count || 0,
        failures: stats.failures_count || 0
      };
      this.updateStats();

      // Load system status
      await this.loadSystemStatus();
    } catch (error) {
      Toast.error('Failed to load dashboard data: ' + error.message);
    }
  }

  updateStats() {
    const cards = this.container.querySelectorAll('.stat-card');
    cards.forEach(card => {
      const stat = card.dataset.stat;
      const valueEl = card.querySelector('.stat-value');
      if (valueEl && this.stats[stat] !== undefined) {
        valueEl.textContent = this.stats[stat].toLocaleString();
      }
    });
  }

  async loadSystemStatus() {
    const statusContainer = this.container.querySelector('#system-status');

    try {
      const [live, ready, version] = await Promise.all([
        window.api.getHealthLive().catch(() => ({ status: 'error' })),
        window.api.getHealthReady().catch(() => ({ status: 'error' })),
        window.api.getVersion().catch(() => ({ version: 'unknown' }))
      ]);

      statusContainer.innerHTML = `
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-6);">
          <div style="display: flex; align-items: center; gap: var(--space-3);">
            <div style="
              width: 12px;
              height: 12px;
              border-radius: var(--radius-full);
              background-color: ${live.status === 'alive' ? 'var(--color-success)' : 'var(--color-error)'}"></div>
            <div>
              <div style="font-weight: var(--font-medium);">Liveness</div>
              <div style="font-size: var(--text-sm); color: var(--color-text-secondary);">${live.status}</div>
            </div>
          </div>

          <div style="display: flex; align-items: center; gap: var(--space-3);">
            <div style="
              width: 12px;
              height: 12px;
              border-radius: var(--radius-full);
              background-color: ${ready.status === 'ready' ? 'var(--color-success)' : 'var(--color-warning)'}"></div>
            <div>
              <div style="font-weight: var(--font-medium);">Readiness</div>
              <div style="font-size: var(--text-sm); color: var(--color-text-secondary);">${ready.status}</div>
            </div>
          </div>

          <div style="display: flex; align-items: center; gap: var(--space-3);">
            <div style="
              width: 12px;
              height: 12px;
              border-radius: var(--radius-full);
              background-color: var(--color-info)"></div>
            <div>
              <div style="font-weight: var(--font-medium);">Version</div>
              <div style="font-size: var(--text-sm); color: var(--color-text-secondary);">${version.version || 'unknown'}</div>
            </div>
          </div>

          <div style="display: flex; align-items: center; gap: var(--space-3);">
            <div style="
              width: 12px;
              height: 12px;
              border-radius: var(--radius-full);
              background-color: var(--color-success)"></div>
            <div>
              <div style="font-weight: var(--font-medium);">Web UI</div>
              <div style="font-size: var(--text-sm); color: var(--color-text-secondary);">Connected</div>
            </div>
          </div>
        </div>
      `;
    } catch (error) {
      statusContainer.innerHTML = `
        <div style="text-align: center; color: var(--color-error); padding: var(--space-4);">
          Failed to load system status: ${error.message}
        </div>
      `;
    }
  }

  async handleIngest() {
    const btn = this.container.querySelector('#ingest-btn');
    btn.disabled = true;
    btn.innerHTML = `
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="animation: spin 1s linear infinite;">
        <circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20"/>
      </svg>
      Ingesting...
    `;

    try {
      await window.api.triggerIngest();
      Toast.success('Document ingestion started successfully');
      setTimeout(() => this.loadData(), 2000);
    } catch (error) {
      Toast.error('Failed to start ingestion: ' + error.message);
    } finally {
      btn.disabled = false;
      btn.innerHTML = `
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="17 8 12 3 7 8"/>
          <line x1="12" y1="3" x2="12" y2="15"/>
        </svg>
        Start Ingestion
      `;
    }
  }
}

window.Dashboard = Dashboard;
