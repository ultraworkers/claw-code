/**
 * IDE Tools Page
 * Code validation interface for testing against guardrails
 */

class IDETools {
  constructor(container) {
    this.container = container;
    this.projects = [];
    this.rules = [];
    this.languages = ['bash', 'go', 'javascript', 'python', 'dockerfile', 'yaml', 'json', 'markdown'];
    this.render();
    this.loadData();
  }

  render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">IDE Tools</h1>
            <p class="page-description">Validate code against prevention rules</p>
          </div>
        </div>

        <div class="split-pane" style="height: calc(100vh - 200px);">
          <div class="split-pane-left">
            <div class="card" style="height: 100%; display: flex; flex-direction: column;">
              <div class="card-header">
                <h3 class="card-title">Code Validator</h3>
              </div>
              <div class="card-body" style="flex: 1; display: flex; flex-direction: column;">
                <div style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                  <select class="form-select" id="validate-language" style="width: 150px;">
                    ${this.languages.map(l => `<option value="${l}">${this.capitalize(l)}</option>`).join('')}
                  </select>
                  <select class="form-select" id="validate-project" style="flex: 1;">
                    <option value="">All Rules (no project context)</option>
                  </select>
                </div>

                <textarea
                  id="validate-code"
                  class="form-textarea"
                  placeholder="Enter code to validate..."
                  style="flex: 1; font-family: var(--font-family-mono); min-height: 200px;"
                ></textarea>

                <div style="margin-top: var(--space-4); display: flex; gap: var(--space-3);">
                  <button class="btn btn-primary" id="validate-file-btn">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                      <polyline points="14 2 14 8 20 8"/>
                      <polyline points="9 15 12 12 15 15"/>
                    </svg>
                    Validate as File
                  </button>
                  <button class="btn btn-secondary" id="validate-selection-btn">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                      <line x1="9" y1="9" x2="15" y2="9"/>
                      <line x1="9" y1="15" x2="15" y2="15"/>
                    </svg>
                    Validate Selection
                  </button>
                  <button class="btn btn-ghost" id="clear-code-btn">
                    Clear
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="split-pane-right">
            <div class="card" style="height: 100%; display: flex; flex-direction: column;">
              <div class="card-header">
                <h3 class="card-title">Results</h3>
                <span id="result-badge" class="badge badge-neutral" style="display: none;"></span>
              </div>
              <div class="card-body" id="results-container" style="flex: 1; overflow-y: auto;">
                <div class="empty-state" style="padding: var(--space-12);">
                  <svg class="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="16 18 22 12 16 6"/>
                    <polyline points="8 6 2 12 8 18"/>
                  </svg>
                  <h3 class="empty-state-title">Ready to Validate</h3>
                  <p class="empty-state-description">
                    Enter code on the left and click "Validate" to check against prevention rules.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="content-section" style="margin-top: var(--space-6);">
          <div class="card">
            <div class="card-header">
              <h3 class="card-title">Quick Reference</h3>
              <button class="btn btn-sm btn-ghost" id="refresh-ref-btn">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="23 4 23 10 17 10"/>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
                Refresh
              </button>
            </div>
            <div class="card-body" id="quick-ref-content">
              <div class="loading-state" style="padding: var(--space-8);">
                <div class="spinner"></div>
                <p class="loading-text">Loading quick reference...</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    // Validate file button
    const validateFileBtn = this.container.querySelector('#validate-file-btn');
    if (validateFileBtn) {
      validateFileBtn.addEventListener('click', () => this.validateFile());
    }

    // Validate selection button
    const validateSelectionBtn = this.container.querySelector('#validate-selection-btn');
    if (validateSelectionBtn) {
      validateSelectionBtn.addEventListener('click', () => this.validateSelection());
    }

    // Clear button
    const clearBtn = this.container.querySelector('#clear-code-btn');
    if (clearBtn) {
      clearBtn.addEventListener('click', () => this.clearCode());
    }

    // Refresh reference button
    const refreshRefBtn = this.container.querySelector('#refresh-ref-btn');
    if (refreshRefBtn) {
      refreshRefBtn.addEventListener('click', () => this.loadQuickReference());
    }
  }

  async loadData() {
    try {
      const projectsRes = await window.api.listProjects({ limit: 100 });
      this.projects = projectsRes.data || [];

      // Populate project dropdown
      const projectSelect = this.container.querySelector('#validate-project');
      if (projectSelect) {
        projectSelect.innerHTML = `
          <option value="">All Rules (no project context)</option>
          ${this.projects.map(p => `<option value="${p.slug}">${p.name}</option>`).join('')}
        `;
      }

      // Load quick reference
      this.loadQuickReference();
    } catch (error) {
      console.error('Failed to load IDE tools data:', error);
    }
  }

  async loadQuickReference() {
    const contentContainer = this.container.querySelector('#quick-ref-content');
    contentContainer.innerHTML = `
      <div class="loading-state" style="padding: var(--space-8);">
        <div class="spinner"></div>
        <p class="loading-text">Loading quick reference...</p>
      </div>
    `;

    try {
      const response = await window.api.getQuickReference();
      const reference = response.data?.reference || 'No quick reference available.';

      contentContainer.innerHTML = `
        <div style="font-family: var(--font-family-mono); font-size: var(--text-sm); line-height: 1.6; white-space: pre-wrap;">
          ${this.escapeHtml(reference)}
        </div>
      `;
    } catch (error) {
      contentContainer.innerHTML = `
        <div style="text-align: center; color: var(--color-error); padding: var(--space-4);">
          Failed to load quick reference: ${error.message}
        </div>
      `;
    }
  }

  async validateFile() {
    const code = this.container.querySelector('#validate-code').value.trim();
    if (!code) {
      Toast.warning('Please enter code to validate');
      return;
    }

    const language = this.container.querySelector('#validate-language').value;
    const projectSlug = this.container.querySelector('#validate-project').value;

    this.showLoadingResults();

    try {
      const response = await window.api.validateFile({
        file_path: `test.${language}`,
        content: code,
        language,
        project_slug: projectSlug || undefined
      });

      this.showResults(response);
    } catch (error) {
      this.showError(error.message);
    }
  }

  async validateSelection() {
    const code = this.container.querySelector('#validate-code').value.trim();
    if (!code) {
      Toast.warning('Please enter code to validate');
      return;
    }

    const language = this.container.querySelector('#validate-language').value;

    this.showLoadingResults();

    try {
      const response = await window.api.validateSelection({
        code,
        language,
        context: 'IDE validation'
      });

      this.showResults(response);
    } catch (error) {
      this.showError(error.message);
    }
  }

  showLoadingResults() {
    const container = this.container.querySelector('#results-container');
    const badge = this.container.querySelector('#result-badge');

    badge.style.display = 'none';
    container.innerHTML = `
      <div class="loading-state" style="padding: var(--space-8);">
        <div class="spinner"></div>
        <p class="loading-text">Validating...</p>
      </div>
    `;
  }

  showResults(response) {
    const container = this.container.querySelector('#results-container');
    const badge = this.container.querySelector('#result-badge');

    const violations = response.violations || [];
    const isValid = violations.length === 0;

    // Update badge
    badge.style.display = 'inline-flex';
    badge.className = `badge badge-${isValid ? 'success' : 'error'}`;
    badge.textContent = isValid ? 'Valid' : `${violations.length} Violations`;

    if (isValid) {
      container.innerHTML = `
        <div class="empty-state" style="padding: var(--space-12);">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="2" style="margin-bottom: var(--space-4);">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
            <polyline points="22 4 12 14.01 9 11.01"/>
          </svg>
          <h3 class="empty-state-title" style="color: var(--color-success);">No Violations Found</h3>
          <p class="empty-state-description">
            Your code passed all prevention rules.
          </p>
        </div>
      `;
    } else {
      container.innerHTML = `
        <div style="display: flex; flex-direction: column; gap: var(--space-3);">
          ${violations.map((v, i) => this.renderViolation(v, i)).join('')}
        </div>
      `;
    }
  }

  renderViolation(violation, index) {
    const severityColor = violation.severity === 'error' ? 'error' : violation.severity === 'warning' ? 'warning' : 'info';

    return `
      <div class="violation-card" style="
        border: 1px solid var(--color-border);
        border-left: 4px solid var(--color-${severityColor});
        border-radius: var(--radius-md);
        padding: var(--space-4);
        background-color: var(--color-bg-secondary);
      ">
        <div style="display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2);">
          <span class="badge badge-${severityColor}">${this.capitalize(violation.severity)}</span>
          <strong>${violation.rule_id}: ${this.escapeHtml(violation.rule_name)}</strong>
        </div>
        <p style="margin: 0 0 var(--space-2); color: var(--color-text-secondary);">
          ${this.escapeHtml(violation.message)}
        </p>
        ${violation.line ? `
          <div style="font-size: var(--text-xs); color: var(--color-text-tertiary);">
            Line ${violation.line}${violation.column ? `, Column ${violation.column}` : ''}
          </div>
        ` : ''}
        ${violation.suggestion ? `
          <div style="
            margin-top: var(--space-2);
            padding: var(--space-2) var(--space-3);
            background-color: var(--color-${severityColor}-subtle);
            border-radius: var(--radius-sm);
            font-size: var(--text-sm);
          ">
            <strong>Suggestion:</strong> ${this.escapeHtml(violation.suggestion)}
          </div>
        ` : ''}
      </div>
    `;
  }

  showError(message) {
    const container = this.container.querySelector('#results-container');
    const badge = this.container.querySelector('#result-badge');

    badge.style.display = 'inline-flex';
    badge.className = 'badge badge-error';
    badge.textContent = 'Error';

    container.innerHTML = `
      <div style="text-align: center; color: var(--color-error); padding: var(--space-8);">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-bottom: var(--space-4);">
          <circle cx="12" cy="12" r="10"/>
          <line x1="15" y1="9" x2="9" y2="15"/>
          <line x1="9" y1="9" x2="15" y2="15"/>
        </svg>
        <p>Validation failed: ${this.escapeHtml(message)}</p>
      </div>
    `;
  }

  clearCode() {
    this.container.querySelector('#validate-code').value = '';
    this.container.querySelector('#results-container').innerHTML = `
      <div class="empty-state" style="padding: var(--space-12);">
        <svg class="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="16 18 22 12 16 6"/>
          <polyline points="8 6 2 12 8 18"/>
        </svg>
        <h3 class="empty-state-title">Ready to Validate</h3>
        <p class="empty-state-description">
          Enter code on the left and click "Validate" to check against prevention rules.
        </p>
      </div>
    `;
    this.container.querySelector('#result-badge').style.display = 'none';
  }

  capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
  }

  escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

window.IDETools = IDETools;
