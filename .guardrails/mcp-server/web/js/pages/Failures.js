/**
 * Failures Page
 * List and update failure registry entries
 */

class Failures {
  constructor(container) {
    this.container = container;
    this.failures = [];
    this.statuses = ['active', 'resolved', 'deprecated'];
    this.severities = ['critical', 'high', 'medium', 'low'];
    this.table = null;
    this.render();
    this.loadData();
  }

  render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">Failure Registry</h1>
            <p class="page-description">Track and manage recorded failures</p>
          </div>
        </div>

        <div class="toolbar">
          <div class="toolbar-left">
            <select class="form-select" id="status-filter" style="width: auto;">
              <option value="">All Statuses</option>
              ${this.statuses.map(s => `<option value="${s}">${this.capitalize(s)}</option>`).join('')}
            </select>
            <select class="form-select" id="severity-filter" style="width: auto;">
              <option value="">All Severities</option>
              ${this.severities.map(s => `<option value="${s}">${this.capitalize(s)}</option>`).join('')}
            </select>
          </div>
          <div class="toolbar-right">
            ${Forms.search({ placeholder: 'Search failures...', id: 'failure-search' })}
          </div>
        </div>

        <div id="failures-table"></div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    const statusFilter = this.container.querySelector('#status-filter');
    if (statusFilter) {
      statusFilter.addEventListener('change', () => this.loadData());
    }

    const severityFilter = this.container.querySelector('#severity-filter');
    if (severityFilter) {
      severityFilter.addEventListener('change', () => this.loadData());
    }

    const searchInput = this.container.querySelector('#failure-search');
    if (searchInput) {
      let debounceTimer;
      searchInput.addEventListener('input', (e) => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
          this.filterTable(e.target.value);
        }, 300);
      });
    }
  }

  async loadData() {
    try {
      const statusFilter = this.container.querySelector('#status-filter');
      const severityFilter = this.container.querySelector('#severity-filter');

      const params = {};
      if (statusFilter?.value) params.status = statusFilter.value;
      if (severityFilter?.value) params.severity = severityFilter.value;

      const response = await window.api.listFailures({
        limit: 100,
        ...params
      });

      this.failures = response.data || [];
      this.renderTable();
    } catch (error) {
      Toast.error('Failed to load failures: ' + error.message);
    }
  }

  renderTable() {
    const tableContainer = this.container.querySelector('#failures-table');

    this.table = new DataTable(tableContainer, {
      columns: [
        {
          key: 'failure_id',
          title: 'Failure ID',
          sortable: true,
          width: '120px'
        },
        {
          key: 'error_message',
          title: 'Error',
          sortable: true,
          formatter: (val) => `<span style="font-weight: 500;">${this.escapeHtml(val)}</span>`
        },
        {
          key: 'category',
          title: 'Category',
          sortable: true,
          width: '120px',
          formatter: (val) => this.capitalize(val)
        },
        {
          key: 'severity',
          title: 'Severity',
          sortable: true,
          width: '100px',
          formatter: (val) => `<span class="badge badge-${this.getSeverityColor(val)}">${this.capitalize(val)}</span>`
        },
        {
          key: 'status',
          title: 'Status',
          sortable: true,
          width: '100px',
          formatter: (val) => `<span class="badge badge-${this.getStatusColor(val)}">${this.capitalize(val)}</span>`
        },
        {
          key: 'project_slug',
          title: 'Project',
          sortable: true,
          width: '150px'
        },
        {
          key: 'created_at',
          title: 'Created',
          sortable: true,
          width: '150px',
          formatter: (val) => this.formatDate(val)
        }
      ],
      data: this.failures,
      rowActions: [
        {
          key: 'view',
          label: 'View',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>`,
          handler: (failure) => this.viewFailure(failure)
        },
        {
          key: 'status',
          label: 'Update Status',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>`,
          handler: (failure) => this.updateStatus(failure)
        }
      ],
      emptyText: 'No failures found',
      emptyDescription: 'The failure registry is empty.'
    });
  }

  filterTable(query) {
    if (!this.table) return;

    const lowerQuery = query.toLowerCase();
    this.table.filter(failure =>
      failure.failure_id.toLowerCase().includes(lowerQuery) ||
      failure.error_message.toLowerCase().includes(lowerQuery) ||
      failure.category.toLowerCase().includes(lowerQuery)
    );
  }

  viewFailure(failure) {
    Modal.form({
      title: `Failure: ${failure.failure_id}`,
      size: 'lg',
      fields: `
        <div style="display: grid; gap: var(--space-4);">
          <div class="info-grid" style="display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--space-4);">
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Category</label>
              <div>${this.capitalize(failure.category)}</div>
            </div>
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Severity</label>
              <div><span class="badge badge-${this.getSeverityColor(failure.severity)}">${this.capitalize(failure.severity)}</span></div>
            </div>
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Status</label>
              <div><span class="badge badge-${this.getStatusColor(failure.status)}">${this.capitalize(failure.status)}</span></div>
            </div>
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Project</label>
              <div>${failure.project_slug || '-'}</div>
            </div>
          </div>

          <div>
            <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Error Message</label>
            <div style="background-color: var(--color-bg-secondary); padding: var(--space-3); border-radius: var(--radius-md); margin-top: var(--space-2);">
              ${this.escapeHtml(failure.error_message)}
            </div>
          </div>

          ${failure.root_cause ? `
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Root Cause</label>
              <div style="background-color: var(--color-bg-secondary); padding: var(--space-3); border-radius: var(--radius-md); margin-top: var(--space-2);">
                ${this.escapeHtml(failure.root_cause)}
              </div>
            </div>
          ` : ''}

          ${failure.affected_files?.length ? `
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Affected Files</label>
              <ul style="margin: var(--space-2) 0 0; padding-left: var(--space-4); font-size: var(--text-sm);">
                ${failure.affected_files.map(f => `<li>${this.escapeHtml(f)}</li>`).join('')}
              </ul>
            </div>
          ` : ''}

          ${failure.regression_pattern ? `
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Regression Pattern</label>
              <code style="display: block; background-color: var(--color-bg-tertiary); padding: var(--space-2); border-radius: var(--radius-sm); margin-top: var(--space-2); font-size: var(--text-xs);">
                ${this.escapeHtml(failure.regression_pattern)}
              </code>
            </div>
          ` : ''}

          <div class="info-grid" style="display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--space-4);">
            <div>
              <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Created</label>
              <div>${this.formatDate(failure.created_at)}</div>
            </div>
            ${failure.updated_at ? `
              <div>
                <label style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-transform: uppercase;">Updated</label>
                <div>${this.formatDate(failure.updated_at)}</div>
              </div>
            ` : ''}
          </div>
        </div>
      `,
      showFooter: true,
      confirmText: 'Close',
      cancelText: null,
      confirmClass: 'btn-secondary'
    });
  }

  updateStatus(failure) {
    Modal.form({
      title: `Update Status: ${failure.failure_id}`,
      fields: `
        <p style="color: var(--color-text-secondary); margin-bottom: var(--space-4);">
          Current status: <span class="badge badge-${this.getStatusColor(failure.status)}">${this.capitalize(failure.status)}</span>
        </p>
        ${Forms.select({
          name: 'status',
          label: 'New Status',
          options: this.statuses.map(s => ({ value: s, label: this.capitalize(s) })),
          value: failure.status,
          required: true
        })}
      `,
      validate: (data) => Forms.validate(data, {
        status: { required: true }
      }),
      onSubmit: async (data, modal) => {
        await window.api.updateFailure(failure.id, data);
        Toast.success('Failure status updated');
        modal.close();
        this.loadData();
      }
    });
  }

  getSeverityColor(severity) {
    const colors = {
      critical: 'error',
      high: 'error',
      medium: 'warning',
      low: 'info'
    };
    return colors[severity] || 'neutral';
  }

  getStatusColor(status) {
    const colors = {
      active: 'error',
      resolved: 'success',
      deprecated: 'neutral'
    };
    return colors[status] || 'neutral';
  }

  capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
  }

  formatDate(dateStr) {
    if (!dateStr) return '-';
    const date = new Date(dateStr);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

window.Failures = Failures;
