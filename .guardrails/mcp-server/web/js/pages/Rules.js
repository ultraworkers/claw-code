/**
 * Rules Page
 * CRUD operations for prevention rules with toggle support
 */

class Rules {
  constructor(container) {
    this.container = container;
    this.rules = [];
    this.categories = ['git', 'bash', 'docker', 'security', 'general'];
    this.severities = ['error', 'warning', 'info'];
    this.table = null;
    this.render();
    this.loadData();
  }

  render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">Prevention Rules</h1>
            <p class="page-description">Manage guardrail prevention rules</p>
          </div>
          <div class="page-actions">
            <button class="btn btn-primary" id="create-rule-btn">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19"/>
                <line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
              Create Rule
            </button>
          </div>
        </div>

        <div class="toolbar">
          <div class="toolbar-left">
            <select class="form-select" id="enabled-filter" style="width: auto;">
              <option value="">All Rules</option>
              <option value="true">Enabled Only</option>
              <option value="false">Disabled Only</option>
            </select>
            <select class="form-select" id="category-filter" style="width: auto;">
              <option value="">All Categories</option>
              ${this.categories.map(c => `<option value="${c}">${this.capitalize(c)}</option>`).join('')}
            </select>
          </div>
          <div class="toolbar-right">
            ${Forms.search({ placeholder: 'Search rules...', id: 'rule-search' })}
          </div>
        </div>

        <div id="rules-table"></div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    // Create button
    const createBtn = this.container.querySelector('#create-rule-btn');
    if (createBtn) {
      createBtn.addEventListener('click', () => this.createRule());
    }

    // Filters
    const enabledFilter = this.container.querySelector('#enabled-filter');
    if (enabledFilter) {
      enabledFilter.addEventListener('change', () => this.loadData());
    }

    const categoryFilter = this.container.querySelector('#category-filter');
    if (categoryFilter) {
      categoryFilter.addEventListener('change', () => this.loadData());
    }

    // Search
    const searchInput = this.container.querySelector('#rule-search');
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
      const enabledFilter = this.container.querySelector('#enabled-filter');
      const categoryFilter = this.container.querySelector('#category-filter');

      const params = {};
      if (enabledFilter?.value) {
        params.enabled = enabledFilter.value === 'true';
      }
      if (categoryFilter?.value) {
        params.category = categoryFilter.value;
      }

      const response = await window.api.listRules({
        limit: 100,
        ...params
      });

      this.rules = response.data || [];
      this.renderTable();
    } catch (error) {
      Toast.error('Failed to load rules: ' + error.message);
    }
  }

  renderTable() {
    const tableContainer = this.container.querySelector('#rules-table');

    this.table = new DataTable(tableContainer, {
      columns: [
        {
          key: 'rule_id',
          title: 'Rule ID',
          sortable: true,
          width: '120px'
        },
        {
          key: 'name',
          title: 'Name',
          sortable: true
        },
        {
          key: 'pattern',
          title: 'Pattern',
          sortable: true,
          formatter: (val) => `<code style="background-color: var(--color-bg-tertiary); padding: 0.125rem 0.375rem; border-radius: var(--radius-sm); font-size: var(--text-xs);">${this.escapeHtml(val)}</code>`
        },
        {
          key: 'severity',
          title: 'Severity',
          sortable: true,
          width: '100px',
          formatter: (val) => `<span class="badge badge-${val === 'error' ? 'error' : val === 'warning' ? 'warning' : 'info'}">${this.capitalize(val)}</span>`
        },
        {
          key: 'category',
          title: 'Category',
          sortable: true,
          width: '120px',
          formatter: (val) => this.capitalize(val)
        },
        {
          key: 'enabled',
          title: 'Enabled',
          sortable: true,
          width: '100px',
          formatter: (val, row) => `
            <label class="toggle" style="margin: 0;">
              <input type="checkbox" class="toggle-input rule-toggle" data-id="${row.id}" ${val ? 'checked' : ''}>
              <span class="toggle-slider"></span>
            </label>
          `
        }
      ],
      data: this.rules,
      rowActions: [
        {
          key: 'edit',
          label: 'Edit',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>`,
          handler: (rule) => this.editRule(rule)
        },
        {
          key: 'delete',
          label: 'Delete',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>`,
          handler: (rule) => this.deleteRule(rule)
        }
      ],
      emptyText: 'No rules found',
      emptyDescription: 'Create your first prevention rule to get started.'
    });

    // Attach toggle events
    tableContainer.querySelectorAll('.rule-toggle').forEach(toggle => {
      toggle.addEventListener('change', (e) => {
        const id = e.target.dataset.id;
        const enabled = e.target.checked;
        this.toggleRule(id, enabled, e.target);
      });
    });
  }

  filterTable(query) {
    if (!this.table) return;

    const lowerQuery = query.toLowerCase();
    this.table.filter(rule =>
      rule.name.toLowerCase().includes(lowerQuery) ||
      rule.pattern.toLowerCase().includes(lowerQuery) ||
      rule.rule_id.toLowerCase().includes(lowerQuery)
    );
  }

  async toggleRule(id, enabled, toggleEl) {
    try {
      await window.api.toggleRule(id, enabled);
      Toast.success(`Rule ${enabled ? 'enabled' : 'disabled'} successfully`);
    } catch (error) {
      // Revert toggle on error
      toggleEl.checked = !enabled;
      Toast.error('Failed to toggle rule: ' + error.message);
    }
  }

  createRule() {
    Modal.form({
      title: 'Create Prevention Rule',
      fields: this.renderRuleForm(),
      validate: (data) => Forms.validate(data, {
        rule_id: { required: true, pattern: /^PREVENT-\d+$/, message: 'Rule ID must be like PREVENT-001' },
        name: { required: true, minLength: 2 },
        pattern: { required: true },
        message: { required: true },
        severity: { required: true },
        category: { required: true }
      }),
      onSubmit: async (data, modal) => {
        await window.api.createRule({
          ...data,
          enabled: true
        });
        Toast.success('Rule created successfully');
        modal.close();
        this.loadData();
      }
    });
  }

  editRule(rule) {
    Modal.form({
      title: `Edit Rule: ${rule.rule_id}`,
      fields: this.renderRuleForm(rule),
      validate: (data) => Forms.validate(data, {
        name: { required: true, minLength: 2 },
        pattern: { required: true },
        message: { required: true },
        severity: { required: true },
        category: { required: true }
      }),
      onSubmit: async (data, modal) => {
        await window.api.updateRule(rule.id, data);
        Toast.success('Rule updated successfully');
        modal.close();
        this.loadData();
      }
    });
  }

  renderRuleForm(rule = {}) {
    return `
      ${Forms.input({
        name: 'rule_id',
        label: 'Rule ID',
        value: rule.rule_id || '',
        placeholder: 'PREVENT-001',
        required: true,
        hint: rule.id ? 'Rule ID cannot be changed' : 'Format: PREVENT-001, PREVENT-002, etc.',
        disabled: !!rule.id
      })}
      ${Forms.input({
        name: 'name',
        label: 'Name',
        value: rule.name || '',
        placeholder: 'No Force Push',
        required: true
      })}
      ${Forms.input({
        name: 'pattern',
        label: 'Pattern (Regex)',
        value: rule.pattern || '',
        placeholder: 'git push --force',
        required: true,
        hint: 'Regular expression to match against commands/code'
      })}
      ${Forms.textarea({
        name: 'message',
        label: 'Message',
        value: rule.message || '',
        placeholder: 'Force push is not allowed',
        rows: 3,
        required: true,
        hint: 'Message shown when pattern is matched'
      })}
      ${Forms.row(`
        ${Forms.select({
          name: 'severity',
          label: 'Severity',
          options: this.severities.map(s => ({ value: s, label: this.capitalize(s) })),
          value: rule.severity || 'error',
          required: true
        })}
        ${Forms.select({
          name: 'category',
          label: 'Category',
          options: this.categories.map(c => ({ value: c, label: this.capitalize(c) })),
          value: rule.category || 'general',
          required: true
        })}
      `)}
    `;
  }

  deleteRule(rule) {
    Modal.confirm({
      title: 'Delete Rule?',
      message: `Are you sure you want to delete <strong>${rule.rule_id}</strong>: ${rule.name}?<br><br>This action cannot be undone.`,
      confirmText: 'Delete Rule',
      confirmClass: 'btn-danger',
      onConfirm: async () => {
        try {
          await window.api.deleteRule(rule.id);
          Toast.success('Rule deleted successfully');
          this.loadData();
        } catch (error) {
          Toast.error('Failed to delete rule: ' + error.message);
        }
      }
    });
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

window.Rules = Rules;
