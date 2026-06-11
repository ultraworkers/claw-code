/**
 * Update Notifier Component
 * Checks for updates on page load, shows notification badges, and handles sync actions
 */

class UpdateNotifier {
  constructor(options = {}) {
    this.options = {
      checkInterval: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
      storageKey: 'guardrail_last_update_check',
      ...options
    };

    this.state = {
      updateAvailable: false,
      updateType: null, // 'docker' or 'guardrail'
      updateData: null,
      isChecking: false,
      isSyncing: false
    };

    this.elements = {};
    this.init();
  }

  /**
   * Initialize the update notifier
   */
  init() {
    this.createStyles();

    // Defer the initial check to avoid race conditions during app initialization
    // Ensures window.api and other dependencies are fully ready
    if (this.shouldCheckOnLoad()) {
      setTimeout(() => {
        // Double-check API availability before making the call
        if (window.api && typeof window.api.getUpdateStatus === 'function') {
          this.checkForUpdates();
        } else {
          console.warn('UpdateNotifier: API not available, skipping update check');
        }
      }, 100);
    }
  }

  /**
   * Check if we should check for updates (daily limit)
   */
  shouldCheckOnLoad() {
    const lastCheck = localStorage.getItem(this.options.storageKey);
    if (!lastCheck) return true;

    const lastCheckTime = parseInt(lastCheck, 10);
    const now = Date.now();
    return (now - lastCheckTime) >= this.options.checkInterval;
  }

  /**
   * Update last check timestamp
   */
  recordCheck() {
    localStorage.setItem(this.options.storageKey, Date.now().toString());
  }

  /**
   * Check for updates via API
   */
  async checkForUpdates() {
    if (this.state.isChecking) return;

    this.state.isChecking = true;

    try {
      const status = await window.api.getUpdateStatus();
      this.recordCheck();

      if (status && status.has_update) {
        this.state.updateAvailable = true;
        this.state.updateType = status.type; // 'docker' or 'guardrail'
        this.state.updateData = status;
        this.showNotification();
      }
    } catch (error) {
      console.error('Failed to check for updates:', error);
    } finally {
      this.state.isChecking = false;
    }
  }

  /**
   * Show notification badge in header
   */
  showNotification() {
    const header = document.querySelector('.header');
    if (!header) return;

    // Check if notifier already exists
    let notifier = header.querySelector('.update-notifier');
    if (notifier) {
      this.updateBadge(notifier);
      return;
    }

    // Create notifier element
    notifier = document.createElement('div');
    notifier.className = 'update-notifier';
    notifier.innerHTML = this.renderBadge();

    // Insert before the first child or append to header
    const headerActions = header.querySelector('.header-actions');
    if (headerActions) {
      headerActions.insertBefore(notifier, headerActions.firstChild);
    } else {
      header.appendChild(notifier);
    }

    this.elements.notifier = notifier;
    this.attachBadgeEvents(notifier);
  }

  /**
   * Render notification badge
   */
  renderBadge() {
    const isDocker = this.state.updateType === 'docker';
    const icon = isDocker
      ? '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>'
      : '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';

    return `
      <button class="update-badge ${this.state.updateType}" aria-label="Updates available">
        ${icon}
        <span class="update-badge-dot"></span>
      </button>
    `;
  }

  /**
   * Update existing badge
   */
  updateBadge(notifier) {
    const badge = notifier.querySelector('.update-badge');
    if (badge) {
      badge.className = `update-badge ${this.state.updateType}`;
    }
  }

  /**
   * Attach events to badge
   */
  attachBadgeEvents(notifier) {
    const badge = notifier.querySelector('.update-badge');
    if (badge) {
      badge.addEventListener('click', () => this.showUpdateModal());
    }
  }

  /**
   * Show update details modal
   */
  showUpdateModal() {
    if (this.state.updateType === 'docker') {
      this.showDockerUpdateModal();
    } else {
      this.showGuardrailUpdateModal();
    }
  }

  /**
   * Show Docker update modal with upgrade instructions
     */
  showDockerUpdateModal() {
    const data = this.state.updateData || {};
    const currentVersion = data.current_version || 'unknown';
    const latestVersion = data.latest_version || 'unknown';
    const releaseNotes = data.release_notes || '';

    const content = `
      <div class="update-modal-content">
        <div class="update-modal-icon docker">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        <h3 class="update-modal-title">Docker Update Available</h3>
        <div class="update-modal-versions">
          <div class="version-item">
            <span class="version-label">Current:</span>
            <code class="version-value">${this.escapeHtml(currentVersion)}</code>
          </div>
          <div class="version-item">
            <span class="version-label">Latest:</span>
            <code class="version-value latest">${this.escapeHtml(latestVersion)}</code>
          </div>
        </div>
        ${releaseNotes ? `
          <div class="update-modal-notes">
            <h4>Release Notes</h4>
            <div class="release-notes-text">${this.escapeHtml(releaseNotes)}</div>
          </div>
        ` : ''}
        <div class="update-modal-command">
          <h4>Upgrade Command</h4>
          <div class="command-box">
            <code>docker pull guardrail/mcp-server:${this.escapeHtml(latestVersion)}</code>
            <button class="command-copy" data-command="docker pull guardrail/mcp-server:${this.escapeHtml(latestVersion)}" title="Copy to clipboard">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    `;

    const modal = new Modal({
      title: 'Update Available',
      size: 'md',
      showFooter: false,
      closable: true
    });

    modal.open(content);
    this.attachModalEvents(modal);
  }

  /**
   * Show Guardrail update modal with sync button
   */
  showGuardrailUpdateModal() {
    const data = this.state.updateData || {};
    const files = data.files || { new: 0, modified: 0, deleted: 0 };
    const lastSync = data.last_sync ? new Date(data.last_sync).toLocaleString() : 'Unknown';

    const content = `
      <div class="update-modal-content">
        <div class="update-modal-icon guardrail">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
          </svg>
        </div>
        <h3 class="update-modal-title">Guardrail Updates Available</h3>
        <p class="update-modal-subtitle">New guardrail definitions are available for synchronization.</p>
        <div class="update-modal-stats">
          <div class="stat-item new">
            <span class="stat-value">${files.new || 0}</span>
            <span class="stat-label">New</span>
          </div>
          <div class="stat-item modified">
            <span class="stat-value">${files.modified || 0}</span>
            <span class="stat-label">Modified</span>
          </div>
          <div class="stat-item deleted">
            <span class="stat-value">${files.deleted || 0}</span>
            <span class="stat-label">Deleted</span>
          </div>
        </div>
        <div class="update-modal-info">
          <p><strong>Last sync:</strong> ${lastSync}</p>
        </div>
        <div class="update-modal-actions">
          <button class="btn btn-secondary" onclick="this.closest('.modal').querySelector('.modal-close').click()">Later</button>
          <button class="btn btn-primary update-sync-btn" ${this.state.isSyncing ? 'disabled' : ''}>
            ${this.state.isSyncing ? `
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="animation: spin 1s linear infinite;">
                <circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20"/>
              </svg>
              Syncing...
            ` : `
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="23 4 23 10 17 10"/>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
              </svg>
              Sync Now
            `}
          </button>
        </div>
      </div>
    `;

    const modal = new Modal({
      title: 'Sync Guardrails',
      size: 'sm',
      showFooter: false,
      closable: true
    });

    modal.open(content);
    this.attachSyncEvent(modal);
  }

  /**
   * Attach sync button event
   */
  attachSyncEvent(modal) {
    const syncBtn = modal.element.querySelector('.update-sync-btn');
    if (syncBtn) {
      syncBtn.addEventListener('click', () => this.handleSync(modal));
    }
  }

  /**
   * Handle guardrail sync
   */
  async handleSync(modal) {
    if (this.state.isSyncing) return;

    this.state.isSyncing = true;

    // Update button state
    const syncBtn = modal.element.querySelector('.update-sync-btn');
    if (syncBtn) {
      syncBtn.disabled = true;
      syncBtn.innerHTML = `
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="animation: spin 1s linear infinite;">
          <circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20"/>
        </svg>
        Syncing...
      `;
    }

    try {
      const result = await window.api.syncGuardrails();

      modal.close();
      Toast.success('Guardrails synchronized successfully', 'Sync Complete');

      // Clear update state
      this.state.updateAvailable = false;
      this.state.updateType = null;
      this.state.updateData = null;

      // Remove badge
      this.removeBadge();

      // Refresh the page if we're on the rules/documents page
      const currentPath = window.location.hash.slice(1);
      if (currentPath === '/rules' || currentPath === '/documents') {
        window.router.refresh();
      }
    } catch (error) {
      Toast.error(error.message || 'Failed to sync guardrails', 'Sync Failed');

      // Reset button state
      if (syncBtn) {
        syncBtn.disabled = false;
        syncBtn.innerHTML = `
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="23 4 23 10 17 10"/>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
          </svg>
          Sync Now
        `;
      }
    } finally {
      this.state.isSyncing = false;
    }
  }

  /**
   * Attach modal events (copy button)
   */
  attachModalEvents(modal) {
    const copyBtn = modal.element.querySelector('.command-copy');
    if (copyBtn) {
      copyBtn.addEventListener('click', () => {
        const command = copyBtn.dataset.command;
        navigator.clipboard.writeText(command).then(() => {
          Toast.success('Command copied to clipboard');
        }).catch(() => {
          Toast.error('Failed to copy command');
        });
      });
    }
  }

  /**
   * Remove notification badge
   */
  removeBadge() {
    const notifier = document.querySelector('.update-notifier');
    if (notifier) {
      notifier.remove();
    }
    this.elements.notifier = null;
  }

  /**
   * Create component styles
   */
  createStyles() {
    if (document.getElementById('update-notifier-styles')) return;

    const styles = document.createElement('style');
    styles.id = 'update-notifier-styles';
    styles.textContent = `
      /* Badge Styles */
      .update-notifier {
        display: inline-flex;
        align-items: center;
      }

      .update-badge {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 36px;
        height: 36px;
        padding: 0;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        cursor: pointer;
        color: var(--color-text-secondary);
        transition: all 0.15s ease;
      }

      .update-badge:hover {
        background: var(--color-surface-hover);
        color: var(--color-text-primary);
      }

      .update-badge.docker {
        color: var(--color-info);
      }

      .update-badge.guardrail {
        color: var(--color-warning);
      }

      .update-badge-dot {
        position: absolute;
        top: 6px;
        right: 6px;
        width: 8px;
        height: 8px;
        background: var(--color-error);
        border-radius: 50%;
        animation: pulse 2s infinite;
      }

      @keyframes pulse {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.5; }
      }

      /* Modal Content Styles */
      .update-modal-content {
        text-align: center;
        padding: 0.5rem;
      }

      .update-modal-icon {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 80px;
        height: 80px;
        border-radius: 50%;
        margin-bottom: 1rem;
      }

      .update-modal-icon.docker {
        background: rgba(59, 130, 246, 0.1);
        color: var(--color-info);
      }

      .update-modal-icon.guardrail {
        background: rgba(245, 158, 11, 0.1);
        color: var(--color-warning);
      }

      .update-modal-title {
        font-size: var(--text-xl);
        font-weight: var(--font-semibold);
        color: var(--color-text-primary);
        margin: 0 0 0.5rem;
      }

      .update-modal-subtitle {
        color: var(--color-text-secondary);
        margin: 0 0 1.5rem;
      }

      /* Version Info */
      .update-modal-versions {
        display: flex;
        justify-content: center;
        gap: 2rem;
        margin-bottom: 1.5rem;
        padding: 1rem;
        background: var(--color-surface-elevated);
        border-radius: var(--radius-lg);
      }

      .version-item {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }

      .version-label {
        font-size: var(--text-xs);
        color: var(--color-text-tertiary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .version-value {
        font-family: var(--font-mono);
        font-size: var(--text-sm);
        color: var(--color-text-secondary);
        background: var(--color-surface);
        padding: 0.25rem 0.5rem;
        border-radius: var(--radius-sm);
      }

      .version-value.latest {
        color: var(--color-success);
        background: rgba(34, 197, 94, 0.1);
      }

      /* Release Notes */
      .update-modal-notes {
        text-align: left;
        margin-bottom: 1.5rem;
      }

      .update-modal-notes h4 {
        font-size: var(--text-sm);
        font-weight: var(--font-semibold);
        color: var(--color-text-primary);
        margin: 0 0 0.75rem;
      }

      .release-notes-text {
        font-size: var(--text-sm);
        color: var(--color-text-secondary);
        line-height: 1.6;
        max-height: 150px;
        overflow-y: auto;
        padding: 0.75rem;
        background: var(--color-surface-elevated);
        border-radius: var(--radius-md);
        white-space: pre-wrap;
      }

      /* Command Box */
      .update-modal-command {
        text-align: left;
        margin-bottom: 1rem;
      }

      .update-modal-command h4 {
        font-size: var(--text-sm);
        font-weight: var(--font-semibold);
        color: var(--color-text-primary);
        margin: 0 0 0.75rem;
      }

      .command-box {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1rem;
        background: var(--color-surface-elevated);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
      }

      .command-box code {
        flex: 1;
        font-family: var(--font-mono);
        font-size: var(--text-sm);
        color: var(--color-text-primary);
        word-break: break-all;
      }

      .command-copy {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 32px;
        height: 32px;
        padding: 0;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        cursor: pointer;
        color: var(--color-text-secondary);
        transition: all 0.15s ease;
        flex-shrink: 0;
      }

      .command-copy:hover {
        background: var(--color-surface-hover);
        color: var(--color-text-primary);
      }

      /* Stats Grid */
      .update-modal-stats {
        display: flex;
        justify-content: center;
        gap: 1rem;
        margin-bottom: 1.5rem;
      }

      .stat-item {
        display: flex;
        flex-direction: column;
        align-items: center;
        min-width: 80px;
        padding: 1rem;
        background: var(--color-surface-elevated);
        border-radius: var(--radius-lg);
        border: 2px solid transparent;
      }

      .stat-item.new {
        border-color: rgba(34, 197, 94, 0.3);
      }

      .stat-item.modified {
        border-color: rgba(245, 158, 11, 0.3);
      }

      .stat-item.deleted {
        border-color: rgba(239, 68, 68, 0.3);
      }

      .stat-value {
        font-size: var(--text-2xl);
        font-weight: var(--font-bold);
        color: var(--color-text-primary);
        line-height: 1;
      }

      .stat-item.new .stat-value {
        color: var(--color-success);
      }

      .stat-item.modified .stat-value {
        color: var(--color-warning);
      }

      .stat-item.deleted .stat-value {
        color: var(--color-error);
      }

      .stat-label {
        font-size: var(--text-xs);
        color: var(--color-text-tertiary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-top: 0.5rem;
      }

      .update-modal-info {
        margin-bottom: 1.5rem;
        padding: 0.75rem;
        background: var(--color-surface-elevated);
        border-radius: var(--radius-md);
      }

      .update-modal-info p {
        margin: 0;
        font-size: var(--text-sm);
        color: var(--color-text-secondary);
      }

      .update-modal-actions {
        display: flex;
        justify-content: center;
        gap: 0.75rem;
      }

      .update-modal-actions .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
      }

      /* Spin animation for loading */
      @keyframes spin {
        to { transform: rotate(360deg); }
      }
    `;

    document.head.appendChild(styles);
  }

  /**
   * Escape HTML to prevent XSS
   */
  escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  /**
   * Force a check for updates (can be called manually)
   */
  async forceCheck() {
    this.state.updateAvailable = false;
    this.state.updateType = null;
    this.state.updateData = null;
    this.removeBadge();
    await this.checkForUpdates();
  }

  /**
   * Get current update status
   */
  getStatus() {
    return {
      hasUpdate: this.state.updateAvailable,
      type: this.state.updateType,
      isChecking: this.state.isChecking,
      isSyncing: this.state.isSyncing
    };
  }
}

window.UpdateNotifier = UpdateNotifier;
