/**
 * Documents Page
 * Browse, search, view and edit documentation
 */

class Documents {
  constructor(container) {
    this.container = container;
    this.documents = [];
    this.categories = ['workflow', 'standard', 'guide', 'reference'];
    this.table = null;
    this.currentDoc = null;
    this.selectedFiles = [];
    this.render();
    this.loadData();
  }

  render() {
    this.container.innerHTML = `
      <div class="page">
        <div class="page-header">
          <div>
            <h1 class="page-title">Documents</h1>
            <p class="page-description">Browse and manage documentation</p>
          </div>
          <button class="btn btn-primary" id="upload-btn">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 0.5rem;">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="17 8 12 3 7 8"/>
              <line x1="12" y1="3" x2="12" y2="15"/>
            </svg>
            Upload Files
          </button>
        </div>

        <div class="toolbar">
          <div class="toolbar-left">
            ${Forms.search({ placeholder: 'Search documents...', id: 'doc-search' })}
          </div>
          <div class="toolbar-right">
            <select class="form-select" id="category-filter" style="width: auto;">
              <option value="">All Categories</option>
              ${this.categories.map(c => `<option value="${c}">${this.capitalize(c)}</option>`).join('')}
            </select>
          </div>
        </div>

        <div id="documents-table"></div>
      </div>
    `;

    this.attachEvents();
  }

  attachEvents() {
    // Search
    const searchInput = this.container.querySelector('#doc-search');
    if (searchInput) {
      let debounceTimer;
      searchInput.addEventListener('input', (e) => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
          this.handleSearch(e.target.value);
        }, 300);
      });
    }

    // Category filter
    const categoryFilter = this.container.querySelector('#category-filter');
    if (categoryFilter) {
      categoryFilter.addEventListener('change', (e) => {
        this.loadData({ category: e.target.value });
      });
    }

    // Upload button
    const uploadBtn = this.container.querySelector('#upload-btn');
    if (uploadBtn) {
      uploadBtn.addEventListener('click', () => this.openUploadModal());
    }
  }

  async loadData(params = {}) {
    try {
      if (this.table) {
        this.table.setLoading(true);
      }

      const response = await window.api.listDocuments({
        limit: 100,
        ...params
      });

      this.documents = response.data || [];
      this.renderTable();
    } catch (error) {
      Toast.error('Failed to load documents: ' + error.message);
    }
  }

  renderTable() {
    const tableContainer = this.container.querySelector('#documents-table');

    this.table = new DataTable(tableContainer, {
      columns: [
        {
          key: 'title',
          title: 'Title',
          sortable: true,
          formatter: (val, row) => `<span style="font-weight: 500;">${val}</span>`
        },
        {
          key: 'slug',
          title: 'Slug',
          sortable: true
        },
        {
          key: 'category',
          title: 'Category',
          sortable: true,
          formatter: (val) => `<span class="badge badge-${this.getCategoryColor(val)}">${this.capitalize(val)}</span>`
        },
        {
          key: 'version',
          title: 'Version',
          sortable: true,
          width: '100px'
        },
        {
          key: 'updated_at',
          title: 'Updated',
          sortable: true,
          formatter: (val) => this.formatDate(val)
        }
      ],
      data: this.documents,
      onRowClick: (doc) => this.openDocument(doc),
      rowActions: [
        {
          key: 'edit',
          label: 'Edit',
          type: 'ghost',
          icon: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>`,
          handler: (doc) => this.openDocument(doc, true)
        }
      ],
      emptyText: 'No documents found',
      emptyDescription: 'Try adjusting your search or filter criteria.'
    });
  }

  async handleSearch(query) {
    if (!query.trim()) {
      this.loadData();
      return;
    }

    try {
      this.table.setLoading(true);
      const response = await window.api.searchDocuments(query);
      this.documents = response.data || [];
      this.table.setData(this.documents);
    } catch (error) {
      Toast.error('Search failed: ' + error.message);
    }
  }

  openDocument(doc, editMode = false) {
    this.currentDoc = doc;

    const content = editMode ? this.renderEditForm(doc) : this.renderViewContent(doc);

    Modal.form({
      title: editMode ? `Edit: ${doc.title}` : doc.title,
      size: 'lg',
      fields: content,
      confirmText: editMode ? 'Save Changes' : 'Close',
      cancelText: editMode ? 'Cancel' : null,
      confirmClass: editMode ? 'btn-primary' : 'btn-secondary',
      onConfirm: editMode ? (data, modal) => this.saveDocument(doc.id, data, modal) : null
    });
  }

  renderViewContent(doc) {
    return `
      <div style="display: flex; flex-direction: column; gap: var(--space-4);">
        <div style="display: flex; gap: var(--space-4); flex-wrap: wrap;">
          <span class="badge badge-${this.getCategoryColor(doc.category)}">${this.capitalize(doc.category)}</span>
          <span style="font-size: var(--text-sm); color: var(--color-text-secondary);">
            Version ${doc.version}
          </span>
          <span style="font-size: var(--text-sm); color: var(--color-text-secondary);">
            Updated ${this.formatDate(doc.updated_at)}
          </span>
        </div>
        <div class="document-content" style="
          background-color: var(--color-bg-secondary);
          border: 1px solid var(--color-border);
          border-radius: var(--radius-md);
          padding: var(--space-4);
          max-height: 500px;
          overflow-y: auto;
          font-family: var(--font-family-mono);
          font-size: var(--text-sm);
          white-space: pre-wrap;
          line-height: 1.6;
        ">${this.escapeHtml(doc.content)}</div>
      </div>
    `;
  }

  renderEditForm(doc) {
    return `
      ${Forms.input({
        name: 'title',
        label: 'Title',
        value: doc.title,
        required: true
      })}
      ${Forms.select({
        name: 'category',
        label: 'Category',
        options: this.categories.map(c => ({ value: c, label: this.capitalize(c) })),
        value: doc.category,
        required: true
      })}
      ${Forms.textarea({
        name: 'content',
        label: 'Content',
        value: doc.content,
        rows: 20,
        required: true,
        hint: 'Markdown content'
      })}
    `;
  }

  async saveDocument(id, data, modal) {
    try {
      modal.setLoading(true);
      await window.api.updateDocument(id, data);
      Toast.success('Document updated successfully');
      modal.close();
      this.loadData();
    } catch (error) {
      modal.setLoading(false);
      throw error;
    }
  }

  getCategoryColor(category) {
    const colors = {
      workflow: 'primary',
      standard: 'success',
      guide: 'warning',
      reference: 'info'
    };
    return colors[category] || 'neutral';
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

  // ==================== FILE UPLOAD METHODS ====================

  openUploadModal() {
    this.selectedFiles = [];

    const modal = new Modal({
      title: 'Upload Documents',
      size: 'lg',
      confirmText: 'Upload',
      cancelText: 'Cancel',
      confirmClass: 'btn-primary',
      onConfirm: (m) => this.handleUploadConfirm(m)
    });

    const content = this.renderUploadForm();
    modal.open(content);
    this.attachUploadEvents(modal);
  }

  renderUploadForm() {
    return `
      <div id="upload-form" style="display: flex; flex-direction: column; gap: var(--space-4);">
        <div id="dropzone" style="
          border: 2px dashed var(--color-border);
          border-radius: var(--radius-lg);
          padding: var(--space-8);
          text-align: center;
          cursor: pointer;
          transition: all 0.2s ease;
          background-color: var(--color-bg-secondary);
        " onmouseover="this.style.borderColor='var(--color-primary)'; this.style.backgroundColor='var(--color-bg-hover)'"
           onmouseout="this.style.borderColor='var(--color-border)'; this.style.backgroundColor='var(--color-bg-secondary)'">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="
            color: var(--color-text-secondary);
            margin-bottom: var(--space-4);
          ">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
          <p style="font-weight: var(--font-medium); color: var(--color-text-primary); margin-bottom: var(--space-2);">
            Drag and drop files here
          </p>
          <p style="font-size: var(--text-sm); color: var(--color-text-secondary); margin-bottom: var(--space-4);">
            or click to browse
          </p>
          <input type="file" id="file-input" multiple accept=".md,.markdown" style="display: none;">
          <button type="button" class="btn btn-secondary" id="browse-btn">Browse Files</button>
        </div>

        <div id="file-restrictions" style="
          font-size: var(--text-sm);
          color: var(--color-text-secondary);
          text-align: center;
        ">
          Accepted formats: <strong>.md</strong>, <strong>.markdown</strong>
        </div>

        <div id="file-preview" style="display: none;">
          <h4 style="font-size: var(--text-sm); font-weight: var(--font-semibold); margin-bottom: var(--space-3); color: var(--color-text-secondary);">
            Selected Files (<span id="file-count">0</span>)
          </h4>
          <div id="file-list" style="
            max-height: 200px;
            overflow-y: auto;
            border: 1px solid var(--color-border);
            border-radius: var(--radius-md);
            padding: var(--space-3);
          "></div>
        </div>

        <div id="upload-progress" style="display: none;">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-2);">
            <span id="progress-text" style="font-size: var(--text-sm); color: var(--color-text-secondary);">Uploading...</span>
            <span id="progress-percent" style="font-size: var(--text-sm); font-weight: var(--font-medium);">0%</span>
          </div>
          <div style="
            width: 100%;
            height: 8px;
            background-color: var(--color-bg-secondary);
            border-radius: var(--radius-full);
            overflow: hidden;
          ">
            <div id="progress-bar" style="
              width: 0%;
              height: 100%;
              background-color: var(--color-primary);
              border-radius: var(--radius-full);
              transition: width 0.3s ease;
            "></div>
          </div>
        </div>
      </div>
    `;
  }

  attachUploadEvents(modal) {
    const dropzone = modal.element.querySelector('#dropzone');
    const fileInput = modal.element.querySelector('#file-input');
    const browseBtn = modal.element.querySelector('#browse-btn');

    // Browse button click
    browseBtn.addEventListener('click', () => fileInput.click());
    dropzone.addEventListener('click', (e) => {
      if (e.target !== browseBtn) fileInput.click();
    });

    // File input change
    fileInput.addEventListener('change', (e) => {
      this.handleFiles(Array.from(e.target.files));
      this.updateFilePreview(modal);
    });

    // Drag and drop
    dropzone.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropzone.style.borderColor = 'var(--color-primary)';
      dropzone.style.backgroundColor = 'var(--color-bg-hover)';
    });

    dropzone.addEventListener('dragleave', (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropzone.style.borderColor = 'var(--color-border)';
      dropzone.style.backgroundColor = 'var(--color-bg-secondary)';
    });

    dropzone.addEventListener('drop', (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropzone.style.borderColor = 'var(--color-border)';
      dropzone.style.backgroundColor = 'var(--color-bg-secondary)';

      const files = Array.from(e.dataTransfer.files);
      this.handleFiles(files);
      this.updateFilePreview(modal);
    });
  }

  handleFiles(files) {
    const validExtensions = ['.md', '.markdown'];

    const validFiles = files.filter(file => {
      const ext = '.' + file.name.split('.').pop().toLowerCase();
      return validExtensions.includes(ext);
    });

    const invalidFiles = files.filter(file => {
      const ext = '.' + file.name.split('.').pop().toLowerCase();
      return !validExtensions.includes(ext);
    });

    if (invalidFiles.length > 0) {
      Toast.warning(`${invalidFiles.length} file(s) skipped. Only .md and .markdown files are allowed.`);
    }

    this.selectedFiles = [...this.selectedFiles, ...validFiles];
  }

  updateFilePreview(modal) {
    const previewContainer = modal.element.querySelector('#file-preview');
    const fileList = modal.element.querySelector('#file-list');
    const fileCount = modal.element.querySelector('#file-count');

    if (this.selectedFiles.length === 0) {
      previewContainer.style.display = 'none';
      return;
    }

    previewContainer.style.display = 'block';
    fileCount.textContent = this.selectedFiles.length;

    fileList.innerHTML = this.selectedFiles.map((file, index) => `
      <div style="
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: var(--space-2) 0;
        border-bottom: 1px solid var(--color-border);
      " ${index === this.selectedFiles.length - 1 ? 'style="border-bottom: none;"' : ''}>
        <div style="display: flex; align-items: center; gap: var(--space-3);">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="color: var(--color-text-secondary);">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
            <polyline points="10 9 9 9 8 9"/>
          </svg>
          <div>
            <div style="font-weight: var(--font-medium); font-size: var(--text-sm);">${this.escapeHtml(file.name)}</div>
            <div style="font-size: var(--text-xs); color: var(--color-text-secondary);">${this.formatFileSize(file.size)}</div>
          </div>
        </div>
        <button type="button" class="btn btn-ghost btn-sm" data-remove="${index}" style="padding: 0.25rem;">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    `).join('');

    // Attach remove handlers
    fileList.querySelectorAll('[data-remove]').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.currentTarget.dataset.remove);
        this.selectedFiles.splice(index, 1);
        this.updateFilePreview(modal);
      });
    });
  }

  async handleUploadConfirm(modal) {
    if (this.selectedFiles.length === 0) {
      Toast.error('Please select at least one file to upload');
      return;
    }

    // Show confirmation dialog
    Modal.confirm({
      title: 'Confirm Upload',
      message: `Are you sure you want to upload ${this.selectedFiles.length} file(s)?`,
      confirmText: 'Upload',
      cancelText: 'Cancel',
      confirmClass: 'btn-primary',
      onConfirm: () => this.performUpload(modal)
    });
  }

  async performUpload(modal) {
    const progressContainer = modal.element.querySelector('#upload-progress');
    const progressBar = modal.element.querySelector('#progress-bar');
    const progressText = modal.element.querySelector('#progress-text');
    const progressPercent = modal.element.querySelector('#progress-percent');
    const confirmBtn = modal.element.querySelector('.modal-confirm');

    // Show progress
    progressContainer.style.display = 'block';
    confirmBtn.disabled = true;

    // Simulate progress updates
    let progress = 0;
    const progressInterval = setInterval(() => {
      progress += Math.random() * 15;
      if (progress > 90) progress = 90;
      progressBar.style.width = `${progress}%`;
      progressPercent.textContent = `${Math.round(progress)}%`;
    }, 300);

    try {
      await window.api.uploadFiles(this.selectedFiles);

      clearInterval(progressInterval);
      progressBar.style.width = '100%';
      progressPercent.textContent = '100%';
      progressText.textContent = 'Upload complete!';

      Toast.success(`Successfully uploaded ${this.selectedFiles.length} file(s)`);
      modal.close();
      this.loadData(); // Refresh document list
    } catch (error) {
      clearInterval(progressInterval);
      progressText.textContent = 'Upload failed';
      progressBar.style.backgroundColor = 'var(--color-error)';
      confirmBtn.disabled = false;
      Toast.error('Upload failed: ' + error.message);
    }
  }

  formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
}

window.Documents = Documents;
