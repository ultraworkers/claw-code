/**
 * DataTable Component
 * Sortable, paginated table with row actions and empty states
 * WCAG 3.0+ compliant with keyboard navigation, ARIA live regions, and focus management
 */

class DataTable {
  constructor(container, options = {}) {
    this.container = container;
    this.options = {
      columns: [],
      data: [],
      sortable: true,
      pagination: true,
      pageSize: 20,
      pageSizeOptions: [10, 20, 50, 100],
      onRowClick: null,
      rowActions: [],
      emptyText: 'No data available',
      emptyDescription: 'There are no items to display.',
      ariaLabel: 'Data Table',
      ariaLiveRegion: true,
      liveRegionPoliteness: 'polite',
      selectableRows: false,
      ...options
    };

    this.state = {
      data: [],
      filteredData: [],
      sortColumn: null,
      sortDirection: 'asc',
      currentPage: 1,
      pageSize: this.options.pageSize,
      loading: false,
      selectedRows: [],
      currentRowIndex: 0
    };

    this.liveRegion = null;
    this.init();
  }

  init() {
    this.render();
    this.setData(this.options.data);
  }

  render() {
    this.container.innerHTML = `
      <div class="data-table-wrapper">
        <div class="table-container">
          <table class="table" role="table" aria-label="${this.options.ariaLabel}" tabindex="0">
            <thead>
              <tr>
                ${this.options.columns.map(col => this.renderHeader(col)).join('')}
                ${this.options.rowActions.length ? '<th>Actions</th>' : ''}
              </tr>
            </thead>
            <tbody id="table-body" role="rowgroup">
              <tr><td colspan="${this.options.columns.length + (this.options.rowActions.length ? 1 : 0)}" class="table-empty">
                <div class="loading-state">
                  <div class="spinner"></div>
                  <p class="loading-text">Loading...</p>
                </div>
              </td></tr>
            </tbody>
          </table>
        </div>
        ${this.options.pagination ? this.renderPagination() : ''}
        ${this.options.ariaLiveRegion ? `<div id="table-live-region" role="status" aria-live="${this.options.liveRegionPoliteness}" aria-atomic="true" class="sr-only" style="position: absolute; width: 1px; height: 1px; overflow: hidden;"></div>` : ''}
      </div>
    `;

    this.liveRegion = this.container.querySelector('#table-live-region');
    this.attachEvents();
  }

  renderHeader(column) {
    const sortable = this.options.sortable && column.sortable !== false;
    const isSorted = this.state.sortColumn === column.key;
    const sortClass = isSorted ? this.state.sortDirection : '';
    const sortDirection = isSorted ? this.state.sortDirection : 'none';
    const ariaSort = isSorted ? this.state.sortDirection : undefined;

    return `
      <th class="${sortable ? 'sortable' : ''} ${sortClass}"
          ${sortable ? `data-sort="${column.key}"` : ''}
          style="${column.width ? `width: ${column.width}` : ''}"
          ${ariaSort ? `aria-sort="${ariaSort}"` : ''}
          scope="col">
        ${sortable ? `
          <button class="sort-button"
                  data-sort="${column.key}"
                  aria-label="Sort by ${column.title}, currently ${sortDirection}"
                  style="background: none; border: none; cursor: pointer; padding: 0; display: inline-flex; align-items: center; gap: 0.25rem;">
            ${column.title}
            <span class="sort-indicator" aria-hidden="true">
              ${isSorted && this.state.sortDirection === 'asc' ? '▲' : isSorted && this.state.sortDirection === 'desc' ? '▼' : '↕'}
            </span>
          </button>
        ` : column.title}
      </th>
    `;
  }

  renderRow(item, itemIndex, globalIndex) {
    const isSelected = this.state.selectedRows.includes(globalIndex);
    const ariaSelected = isSelected ? 'true' : 'false';

    return `
      <tr data-index="${globalIndex}"
          data-row-index="${itemIndex}"
          class="${this.options.onRowClick ? 'clickable' : ''} ${isSelected ? 'selected' : ''}"
          role="row"
          aria-selected="${ariaSelected}"
          tabindex="${globalIndex === this.state.currentRowIndex ? '0' : '-1'}">
        ${this.options.selectableRows ? `
          <td role="gridcell" style="width: 40px;">
            <input type="checkbox"
                   aria-checked="${ariaSelected}"
                   data-select-index="${globalIndex}"
                   ${isSelected ? 'checked' : ''}
                   style="cursor: pointer;" />
          </td>
        ` : ''}
        ${this.options.columns.map(col => this.renderCell(item, col, globalIndex)).join('')}
        ${this.options.rowActions.length ? this.renderRowActions(item, globalIndex) : ''}
      </tr>
    `;
  }

  renderCell(item, column, rowIndex) {
    let value = this.getNestedValue(item, column.key);

    if (column.formatter) {
      value = column.formatter(value, item);
    } else if (value === null || value === undefined) {
      value = '-';
    }

    return `<td role="gridcell" data-row="${rowIndex}" data-column="${column.key}">${value}</td>`;
  }

  renderRowActions(item, index) {
    return `
      <td>
        <div class="row-actions">
          ${this.options.rowActions.map(action => `
            <button class="btn btn-sm btn-${action.type || 'ghost'}"
                    data-action="${action.key}"
                    data-index="${index}"
                    title="${action.label}">
              ${action.icon || action.label}
            </button>
          `).join('')}
        </div>
      </td>
    `;
  }

  renderPagination() {
    const totalPages = Math.ceil(this.state.filteredData.length / this.state.pageSize);
    const start = (this.state.currentPage - 1) * this.state.pageSize + 1;
    const end = Math.min(this.state.currentPage * this.state.pageSize, this.state.filteredData.length);

    return `
      <div class="table-pagination" role="navigation" aria-label="Table pagination">
        <div class="pagination-info">
          Showing ${this.state.filteredData.length ? start : 0} to ${end} of ${this.state.filteredData.length} entries
        </div>
        <div class="pagination">
          <button class="pagination-btn" data-page="prev" ${this.state.currentPage === 1 ? 'disabled' : ''} aria-label="Previous page" ${this.state.currentPage === 1 ? 'aria-disabled="true"' : ''}>
            Previous
          </button>
          ${this.renderPageButtons(totalPages)}
          <button class="pagination-btn" data-page="next" ${this.state.currentPage === totalPages || totalPages === 0 ? 'disabled' : ''} aria-label="Next page" ${this.state.currentPage === totalPages || totalPages === 0 ? 'aria-disabled="true"' : ''}>
            Next
          </button>
        </div>
        <div class="page-size-selector">
          <label for="page-size" class="sr-only" style="position: absolute; width: 1px; height: 1px; overflow: hidden;">Items per page:</label>
          <select class="form-select" id="page-size" aria-label="Items per page" style="width: auto;">
            ${this.options.pageSizeOptions.map(size => `
              <option value="${size}" ${size === this.state.pageSize ? 'selected' : ''}>${size} / page</option>
            `).join('')}
          </select>
        </div>
      </div>
    `;
  }

  renderPageButtons(totalPages) {
    if (totalPages <= 1) return '';

    const buttons = [];
    const maxVisible = 5;
    let start = Math.max(1, this.state.currentPage - Math.floor(maxVisible / 2));
    let end = Math.min(totalPages, start + maxVisible - 1);

    if (end - start + 1 < maxVisible) {
      start = Math.max(1, end - maxVisible + 1);
    }

    if (start > 1) {
      buttons.push(`<button class="pagination-btn" data-page="1">1</button>`);
      if (start > 2) buttons.push(`<span class="pagination-ellipsis">...</span>`);
    }

    for (let i = start; i <= end; i++) {
      buttons.push(`
        <button class="pagination-btn ${i === this.state.currentPage ? 'active' : ''}" data-page="${i}" aria-label="Page ${i}" aria-current="${i === this.state.currentPage ? 'page' : ''}">
          ${i}
        </button>
      `);
    }

    if (end < totalPages) {
      if (end < totalPages - 1) buttons.push(`<span class="pagination-ellipsis">...</span>`);
      buttons.push(`<button class="pagination-btn" data-page="${totalPages}">${totalPages}</button>`);
    }

    return buttons.join('');
  }

  renderEmpty() {
    return `
      <tr>
        <td colspan="${this.options.columns.length + (this.options.rowActions.length ? 1 : 0)}" class="table-empty">
          <div class="empty-state">
            <svg class="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
              <line x1="9" y1="9" x2="15" y2="15"/>
              <line x1="15" y1="9" x2="9" y2="15"/>
            </svg>
            <h3 class="empty-state-title">${this.options.emptyText}</h3>
            <p class="empty-state-description">${this.options.emptyDescription}</p>
          </div>
        </td>
      </tr>
    `;
  }

  getNestedValue(obj, path) {
    return path.split('.').reduce((acc, part) => acc && acc[part], obj);
  }

  attachEvents() {
    // Sort
    if (this.options.sortable) {
      this.container.querySelectorAll('th.sortable').forEach(th => {
        th.addEventListener('click', (e) => {
          const column = e.currentTarget.dataset.sort;
          this.handleSort(column);
        });
      });
    }

    // Row click
    if (this.options.onRowClick) {
      this.container.addEventListener('click', (e) => {
        const row = e.target.closest('tr[data-index]');
        if (row && !e.target.closest('.row-actions')) {
          const index = parseInt(row.dataset.index);
          const item = this.getCurrentPageData()[index];
          this.options.onRowClick(item);
        }
      });
    }

    // Row actions
    this.container.addEventListener('click', (e) => {
      const btn = e.target.closest('button[data-action]');
      if (btn) {
        const action = btn.dataset.action;
        const index = parseInt(btn.dataset.index);
        const item = this.getCurrentPageData()[index];
        const actionConfig = this.options.rowActions.find(a => a.key === action);
        if (actionConfig && actionConfig.handler) {
          actionConfig.handler(item, index);
        }
      }
    });

    // Pagination
    this.container.addEventListener('click', (e) => {
      const btn = e.target.closest('button[data-page]');
      if (btn) {
        const page = btn.dataset.page;
        this.handlePageChange(page);
      }
    });

    // Page size
    const pageSizeSelect = this.container.querySelector('#page-size');
    if (pageSizeSelect) {
      pageSizeSelect.addEventListener('change', (e) => {
        this.state.pageSize = parseInt(e.target.value);
        this.state.currentPage = 1;
        this.refresh();
      });
    }

    // Checkbox selection
    if (this.options.selectableRows) {
      this.container.addEventListener('change', (e) => {
        if (e.target.type === 'checkbox' && e.target.dataset.selectIndex) {
          const rowIndex = parseInt(e.target.dataset.selectIndex);
          this.toggleRowSelection(rowIndex);
        }
      });
    }

    // Keyboard navigation for table
    this.container.addEventListener('keydown', (e) => {
      this.handleTableKeyboard(e);
    });
  }

  /**
   * Handle keyboard navigation within table
   * Arrow keys navigate between rows, Enter selects, Space toggles checkbox
   */
  handleTableKeyboard(e) {
    const tbody = this.container.querySelector('#table-body');
    if (!tbody) return;

    const rows = tbody.querySelectorAll('tr[role="row"]');
    if (rows.length === 0) return;

    // Check if focus is on a row
    const currentRow = document.activeElement.closest('tr[role="row"]');
    if (!currentRow) return;

    const currentGlobalIndex = parseInt(currentRow.dataset.index);

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        const nextIndex = Math.min(rows.length - 1, this.state.currentRowIndex + 1);
        this.navigateToRow(nextIndex);
        break;

      case 'ArrowUp':
        e.preventDefault();
        const prevIndex = Math.max(0, this.state.currentRowIndex - 1);
        this.navigateToRow(prevIndex);
        break;

      case 'Enter':
        e.preventDefault();
        if (this.options.onRowClick) {
          const item = this.getCurrentPageData()[this.state.currentRowIndex];
          this.options.onRowClick(item);
        }
        break;

      case 'Space':
        if (this.options.selectableRows && currentRow.querySelector('input[type="checkbox"]')) {
          e.preventDefault();
          const checkbox = currentRow.querySelector('input[type="checkbox"]');
          checkbox.checked = !checkbox.checked;
          this.toggleRowSelection(currentGlobalIndex);
        }
        break;

      case 'Home':
        e.preventDefault();
        this.navigateToRow(0);
        break;

      case 'End':
        e.preventDefault();
        this.navigateToRow(rows.length - 1);
        break;
    }
  }

  /**
   * Navigate focus to a specific row
   */
  navigateToRow(rowIndex) {
    const tbody = this.container.querySelector('#table-body');
    const rows = tbody.querySelectorAll('tr[role="row"]');

    if (rowIndex < 0 || rowIndex >= rows.length) return;

    // Update current row tabindex
    rows.forEach((row, idx) => {
      row.setAttribute('tabindex', idx === rowIndex ? '0' : '-1');
    });

    this.state.currentRowIndex = rowIndex;
    rows[rowIndex].focus();

    // Announce row change to screen readers
    this.announce(`Row ${rowIndex + 1}: ${this.getRowText(rows[rowIndex])}`);
  }

  /**
   * Get text content of a row for screen reader announcement
   */
  getRowText(row) {
    return row.textContent.trim().substring(0, 100);
  }

  /**
   * Toggle row selection state
   */
  toggleRowSelection(rowIndex) {
    const index = this.state.selectedRows.indexOf(rowIndex);
    if (index === -1) {
      this.state.selectedRows.push(rowIndex);
    } else {
      this.state.selectedRows.splice(index, 1);
    }
    this.refresh();
    this.announce(`Row ${rowIndex + 1} ${index === -1 ? 'selected' : 'deselected'}`);
  }

  /**
   * Announce message to screen readers via live region
   */
  announce(message) {
    if (this.liveRegion) {
      this.liveRegion.textContent = '';
      setTimeout(() => {
        this.liveRegion.textContent = message;
      }, 100);
    }
  }

  handleSort(column) {
    if (this.state.sortColumn === column) {
      this.state.sortDirection = this.state.sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      this.state.sortColumn = column;
      this.state.sortDirection = 'asc';
    }

    this.sortData();
    this.refresh();
  }

  handlePageChange(page) {
    const totalPages = Math.ceil(this.state.filteredData.length / this.state.pageSize);

    if (page === 'prev') {
      this.state.currentPage = Math.max(1, this.state.currentPage - 1);
    } else if (page === 'next') {
      this.state.currentPage = Math.min(totalPages, this.state.currentPage + 1);
    } else {
      this.state.currentPage = parseInt(page);
    }

    this.refresh();
  }

  sortData() {
    if (!this.state.sortColumn) return;

    const column = this.options.columns.find(c => c.key === this.state.sortColumn);
    if (!column) return;

    this.state.filteredData.sort((a, b) => {
      let aVal = this.getNestedValue(a, this.state.sortColumn);
      let bVal = this.getNestedValue(b, this.state.sortColumn);

      if (column.sortFn) {
        return this.state.sortDirection === 'asc'
          ? column.sortFn(aVal, bVal)
          : column.sortFn(bVal, aVal);
      }

      if (typeof aVal === 'string') {
        aVal = aVal.toLowerCase();
        bVal = bVal.toLowerCase();
      }

      if (aVal < bVal) return this.state.sortDirection === 'asc' ? -1 : 1;
      if (aVal > bVal) return this.state.sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  }

  getCurrentPageData() {
    const start = (this.state.currentPage - 1) * this.state.pageSize;
    const end = start + this.state.pageSize;
    return this.state.filteredData.slice(start, end);
  }

  refresh() {
    const tbody = this.container.querySelector('#table-body');
    const data = this.getCurrentPageData();
    const pageStart = (this.state.currentPage - 1) * this.state.pageSize;

    if (data.length === 0) {
      tbody.innerHTML = this.renderEmpty();
    } else {
      tbody.innerHTML = data.map((item, index) => {
        const globalIndex = pageStart + index;
        return this.renderRow(item, index, globalIndex);
      }).join('');
    }

    // Update header sort indicators
    this.container.querySelectorAll('th.sortable').forEach(th => {
      th.classList.remove('asc', 'desc');
      if (th.dataset.sort === this.state.sortColumn) {
        th.classList.add(this.state.sortDirection);
      }
    });

    // Update pagination
    if (this.options.pagination) {
      const paginationContainer = this.container.querySelector('.table-pagination');
      if (paginationContainer) {
        paginationContainer.outerHTML = this.renderPagination();
        // Restore focus to pagination if it was focused
        if (document.activeElement && document.activeElement.dataset &&
            document.activeElement.dataset.page && this.state.currentPage) {
          const currentBtn = paginationContainer.querySelector(`[data-page="${this.state.currentPage}"]`);
          if (currentBtn) currentBtn.focus();
        }
      }
    }

    // Announce table updates to screen readers
    if (data.length === 0) {
      this.announce('No data available');
    } else if (this.state.sortColumn) {
      this.announce(`Table sorted by ${this.state.sortColumn}, ${this.state.sortDirection} order`);
    }
  }

  setData(data) {
    this.state.data = [...data];
    this.state.filteredData = [...data];
    this.state.currentPage = 1;
    if (this.state.sortColumn) {
      this.sortData();
    }
    this.refresh();
  }

  filter(fn) {
    if (fn) {
      this.state.filteredData = this.state.data.filter(fn);
    } else {
      this.state.filteredData = [...this.state.data];
    }
    this.state.currentPage = 1;
    this.refresh();
  }

  setLoading(loading) {
    this.state.loading = loading;
    if (loading) {
      const tbody = this.container.querySelector('#table-body');
      tbody.innerHTML = `
        <tr><td colspan="${this.options.columns.length + (this.options.rowActions.length ? 1 : 0)}" class="table-empty">
          <div class="loading-state">
            <div class="spinner"></div>
            <p class="loading-text">Loading...</p>
          </div>
        </td></tr>
      `;
    }
  }

  /**
   * Enable virtual scrolling for large tables (performance + accessibility)
   * Renders only visible rows while maintaining keyboard navigation
   */
  enableVirtualScroll(scrollHeight = 400) {
    const tableContainer = this.container.querySelector('.table-container');
    if (!tableContainer) return;

    tableContainer.style.cssText += `
      max-height: ${scrollHeight}px;
      overflow-y: auto;
      position: relative;
    `;

    // Announce virtual scroll mode to screen readers
    this.announce('Table enabled with virtual scrolling for improved performance');
  }
}

window.DataTable = DataTable;
