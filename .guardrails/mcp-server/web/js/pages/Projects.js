/**
 * Projects Page
 * CRUD operations for projects with context editor
 */

class Projects {
  constructor(container) {
    this.container = container;
    this.projects = [];
    this.rules = [];
    this.table = null;
    this.render();
    this.loadData();
  }

  render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">Projects</h1>
            <p class="page-description">Manage projects and their guardrail contexts</p>
          </div>
          <div class="page-actions">
            <button class="btn btn-primary" id="create-project-btn">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19"/>
                <line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
              Create Project
            </button>
          </div>
        </div>

        <div class="toolbar">
          <div class="toolbar-left">
            ${Forms.search({ placeholder: 'Search projects...', id: 'project-search' })}
          </div>
        </div>

        <div id="projects-table"></div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    const createBtn = this.container.querySelector('#create-project-btn');
    if (createBtn) {
      createBtn.addEventListener('click', () => this.createProject());
    }

    const searchInput = this.container.querySelector('#project-search');
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
      const [projectsRes, rulesRes] = await Promise.all([
        window.api.listProjects({ limit: 100 }),
        window.api.listRules({ enabled: true, limit: 100 })
      ]);

      this.projects = projectsRes.data || [];
      this.rules = rulesRes.data || [];
      this.renderTable();
    } catch (error) {
      Toast.error('Failed to load projects: ' + error.message);
    }
  }

  renderTable() {
    const tableContainer = this.container.querySelector('#projects-table');

    this.table = new DataTable(tableContainer, {
      columns: [
        {
          key: 'name',
          title: 'Name',
          sortable: true,
          formatter: (val, row) => `
            <div>
              <div style="font-weight: 500;">${val}</div>
              <div style="font-size: var(--text-xs); color: var(--color-text-tertiary);">${row.slug}</div>
            </div>
          `
        },
        {
          key: 'active_rules',
          title: 'Active Rules',
          sortable: true,
          width: '150px',
          formatter: (val) => `<span class="badge badge-neutral">${val?.length || 0} rules</span>`
        },
        {
          key: 'guardrail_context',
          title: 'Context',
          sortable: false,
          formatter: (val) => {
            if (!val) return '<span style="color: var(--color-text-tertiary);">-</span>';
            const preview = val.substring(0, 50) + (val.length > 50 ? '...' : '');
            return `<span style="color: var(--color-text-secondary); font-size: var(--text-sm);">${this.escapeHtml(preview)}</span>`;
          }
        },
        {
          key: 'updated_at',
          title: 'Updated',
          sortable: true,
          width: '150px',
          formatter: (val) => this.formatDate(val)
        }
      ],
      data: this.projects,
      rowActions: [
        {
          key: 'edit',
          label: 'Edit',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>`,
          handler: (project) => this.editProject(project)
        },
        {
          key: 'delete',
          label: 'Delete',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>`,
          handler: (project) => this.deleteProject(project)
        }
      ],
      emptyText: 'No projects found',
      emptyDescription: 'Create your first project to configure guardrail contexts.'
    });
  }

  filterTable(query) {
    if (!this.table) return;

    const lowerQuery = query.toLowerCase();
    this.table.filter(project =>
      project.name.toLowerCase().includes(lowerQuery) ||
      project.slug.toLowerCase().includes(lowerQuery)
    );
  }

  createProject() {
    Modal.form({
      title: 'Create Project',
      size: 'lg',
      fields: this.renderProjectForm(),
      validate: (data) => Forms.validate(data, {
        name: { required: true, minLength: 2 },
        slug: { required: true, pattern: /^[a-z0-9-]+$/, message: 'Slug must be lowercase letters, numbers, and hyphens only' }
      }),
      onSubmit: async (data, modal) => {
        await window.api.createProject({
          ...data,
          active_rules: Array.isArray(data.active_rules) ? data.active_rules : (data.active_rules ? [data.active_rules] : [])
        });
        Toast.success('Project created successfully');
        modal.close();
        this.loadData();
      }
    });
  }

  editProject(project) {
    Modal.form({
      title: `Edit Project: ${project.name}`,
      size: 'lg',
      fields: this.renderProjectForm(project),
      validate: (data) => Forms.validate(data, {
        name: { required: true, minLength: 2 }
      }),
      onSubmit: async (data, modal) => {
        await window.api.updateProject(project.id, {
          ...data,
          active_rules: Array.isArray(data.active_rules) ? data.active_rules : (data.active_rules ? [data.active_rules] : [])
        });
        Toast.success('Project updated successfully');
        modal.close();
        this.loadData();
      }
    });
  }

  renderProjectForm(project = {}) {
    const ruleOptions = this.rules.map(r => ({
      value: r.rule_id,
      label: `${r.rule_id}: ${r.name}`
    }));

    const selectedRules = project.active_rules || [];

    return `
      ${Forms.input({
        name: 'name',
        label: 'Project Name',
        value: project.name || '',
        placeholder: 'My Project',
        required: true
      })}
      ${Forms.input({
        name: 'slug',
        label: 'Slug',
        value: project.slug || '',
        placeholder: 'my-project',
        required: true,
        hint: project.id ? 'Slug cannot be changed' : 'Used in URLs and API calls',
        disabled: !!project.id
      })}
      ${Forms.textarea({
        name: 'guardrail_context',
        label: 'Guardrail Context',
        value: project.guardrail_context || '',
        placeholder: '# Project Context\n\nThis project uses...',
        rows: 10,
        hint: 'Markdown content providing context for AI guardrails'
      })}
      <div class="form-group">
        <label class="form-label">Active Rules</label>
        <div style="max-height: 200px; overflow-y: auto; border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: var(--space-3);">
          ${ruleOptions.length === 0 ?
            '<p style="color: var(--color-text-tertiary); font-size: var(--text-sm); margin: 0;">No enabled rules available</p>' :
            ruleOptions.map(rule => `
              <label class="form-check" style="margin-bottom: var(--space-2);">
                <input type="checkbox"
                       name="active_rules"
                       value="${rule.value}"
                       class="form-check-input"
                       ${selectedRules.includes(rule.value) ? 'checked' : ''}>
                <span class="form-check-label" style="font-size: var(--text-sm);">${this.escapeHtml(rule.label)}</span>
              </label>
            `).join('')
          }
        </div>
      </div>
    `;
  }

  deleteProject(project) {
    Modal.confirm({
      title: 'Delete Project?',
      message: `Are you sure you want to delete <strong>${project.name}</strong>?<br><br>This action cannot be undone.`,
      confirmText: 'Delete Project',
      confirmClass: 'btn-danger',
      onConfirm: async () => {
        try {
          await window.api.deleteProject(project.id);
          Toast.success('Project deleted successfully');
          this.loadData();
        } catch (error) {
          Toast.error('Failed to delete project: ' + error.message);
        }
      }
    });
  }

  formatDate(dateStr) {
    if (!dateStr) return '-';
    const date = new Date(dateStr);
    return date.toLocaleDateString();
  }

  escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

window.Projects = Projects;
