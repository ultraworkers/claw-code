/**
 * Forms Component
 * Input fields, select dropdowns, textareas with validation support
 */

class Forms {
  /**
   * Create a form input element
   */
  static input(options = {}) {
    const {
      name,
      type = 'text',
      label,
      placeholder = '',
      value = '',
      required = false,
      disabled = false,
      hint = '',
      error = '',
      className = ''
    } = options;

    const id = `input-${name}-${Date.now()}`;

    return `
      <div class="form-group ${className}">
        ${label ? `<label class="form-label ${required ? 'form-label-required' : ''}" for="${id}">${label}</label>` : ''}
        <input type="${type}"
               id="${id}"
               name="${name}"
               class="form-input ${error ? 'is-error' : ''}"
               placeholder="${placeholder}"
               value="${this.escapeHtml(value)}"
               ${required ? 'required' : ''}
               ${disabled ? 'disabled' : ''}>
        ${hint ? `<p class="form-hint">${hint}</p>` : ''}
        ${error ? `<p class="form-error">${error}</p>` : ''}
      </div>
    `;
  }

  /**
   * Create a textarea element
   */
  static textarea(options = {}) {
    const {
      name,
      label,
      placeholder = '',
      value = '',
      rows = 4,
      required = false,
      disabled = false,
      hint = '',
      error = '',
      className = ''
    } = options;

    const id = `textarea-${name}-${Date.now()}`;

    return `
      <div class="form-group ${className}">
        ${label ? `<label class="form-label ${required ? 'form-label-required' : ''}" for="${id}">${label}</label>` : ''}
        <textarea id="${id}"
                  name="${name}"
                  class="form-textarea ${error ? 'is-error' : ''}"
                  placeholder="${placeholder}"
                  rows="${rows}"
                  ${required ? 'required' : ''}
                  ${disabled ? 'disabled' : ''}>${this.escapeHtml(value)}</textarea>
        ${hint ? `<p class="form-hint">${hint}</p>` : ''}
        ${error ? `<p class="form-error">${error}</p>` : ''}
      </div>
    `;
  }

  /**
   * Create a select dropdown
   */
  static select(options = {}) {
    const {
      name,
      label,
      options: selectOptions = [],
      value = '',
      placeholder = 'Select an option',
      required = false,
      disabled = false,
      hint = '',
      error = '',
      className = ''
    } = options;

    const id = `select-${name}-${Date.now()}`;

    const optionsHtml = selectOptions.map(opt => {
      const optValue = typeof opt === 'object' ? opt.value : opt;
      const optLabel = typeof opt === 'object' ? opt.label : opt;
      const selected = optValue === value ? 'selected' : '';
      return `<option value="${this.escapeHtml(optValue)}" ${selected}>${this.escapeHtml(optLabel)}</option>`;
    }).join('');

    return `
      <div class="form-group ${className}">
        ${label ? `<label class="form-label ${required ? 'form-label-required' : ''}" for="${id}">${label}</label>` : ''}
        <select id="${id}"
                name="${name}"
                class="form-select ${error ? 'is-error' : ''}"
                ${required ? 'required' : ''}
                ${disabled ? 'disabled' : ''}>
          <option value="" ${!value ? 'selected' : ''} disabled>${placeholder}</option>
          ${optionsHtml}
        </select>
        ${hint ? `<p class="form-hint">${hint}</p>` : ''}
        ${error ? `<p class="form-error">${error}</p>` : ''}
      </div>
    `;
  }

  /**
   * Create a checkbox
   */
  static checkbox(options = {}) {
    const {
      name,
      label,
      checked = false,
      disabled = false,
      hint = '',
      className = ''
    } = options;

    const id = `checkbox-${name}-${Date.now()}`;

    return `
      <div class="form-group ${className}">
        <label class="form-check">
          <input type="checkbox"
                 id="${id}"
                 name="${name}"
                 class="form-check-input"
                 ${checked ? 'checked' : ''}
                 ${disabled ? 'disabled' : ''}>
          <span class="form-check-label">${label}</span>
        </label>
        ${hint ? `<p class="form-hint">${hint}</p>` : ''}
      </div>
    `;
  }

  /**
   * Create a toggle switch
   */
  static toggle(options = {}) {
    const {
      name,
      label,
      checked = false,
      disabled = false,
      className = ''
    } = options;

    const id = `toggle-${name}-${Date.now()}`;

    return `
      <div class="form-group ${className}">
        <label class="toggle">
          <input type="checkbox"
                 id="${id}"
                 name="${name}"
                 class="toggle-input"
                 ${checked ? 'checked' : ''}
                 ${disabled ? 'disabled' : ''}>
          <span class="toggle-slider"></span>
          <span class="toggle-label">${label}</span>
        </label>
      </div>
    `;
  }

  /**
   * Create a search input with icon
   */
  static search(options = {}) {
    const {
      name = 'search',
      placeholder = 'Search...',
      value = '',
      disabled = false,
      className = ''
    } = options;

    const id = `search-${name}-${Date.now()}`;

    return `
      <div class="search-input-wrapper ${className}">
        <span class="icon">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.35-4.35"/>
          </svg>
        </span>
        <input type="search"
               id="${id}"
               name="${name}"
               class="search-input"
               placeholder="${placeholder}"
               value="${this.escapeHtml(value)}"
               ${disabled ? 'disabled' : ''}>
      </div>
    `;
  }

  /**
   * Create a form group with multiple fields in a row
   */
  static row(fieldsHtml) {
    return `
      <div class="form-row" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-4);">
        ${fieldsHtml}
      </div>
    `;
  }

  /**
   * Create a form section
   */
  static section(title, fieldsHtml, options = {}) {
    const { description = '' } = options;

    return `
      <fieldset class="form-section" style="border: none; padding: 0; margin: 0 0 var(--space-6);">
        <legend class="form-section-title" style="font-size: var(--text-lg); font-weight: var(--font-semibold); margin-bottom: var(--space-4); padding: 0;">
          ${title}
        </legend>
        ${description ? `<p class="form-section-description" style="color: var(--color-text-secondary); margin-bottom: var(--space-4);">${description}</p>` : ''}
        ${fieldsHtml}
      </fieldset>
    `;
  }

  /**
   * Extract form data from a form element
   */
  static getData(formElement) {
    const formData = new FormData(formElement);
    const data = {};

    for (const [key, value] of formData.entries()) {
      // Handle multiple values (checkboxes, multi-select)
      if (data[key] !== undefined) {
        if (!Array.isArray(data[key])) {
          data[key] = [data[key]];
        }
        data[key].push(value);
      } else {
        data[key] = value;
      }
    }

    // Handle checkboxes (unchecked checkboxes are not in FormData)
    formElement.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
      if (!formData.has(checkbox.name)) {
        data[checkbox.name] = false;
      } else if (data[checkbox.name] === 'on') {
        data[checkbox.name] = true;
      }
    });

    return data;
  }

  /**
   * Validate form data
   */
  static validate(data, rules) {
    const errors = {};

    for (const [field, rule] of Object.entries(rules)) {
      const value = data[field];

      if (rule.required && (!value || (typeof value === 'string' && !value.trim()))) {
        errors[field] = rule.message || `${field} is required`;
        continue;
      }

      if (value && rule.minLength && value.length < rule.minLength) {
        errors[field] = rule.message || `${field} must be at least ${rule.minLength} characters`;
      }

      if (value && rule.maxLength && value.length > rule.maxLength) {
        errors[field] = rule.message || `${field} must be at most ${rule.maxLength} characters`;
      }

      if (value && rule.pattern && !rule.pattern.test(value)) {
        errors[field] = rule.message || `${field} format is invalid`;
      }

      if (value && rule.custom && !rule.custom(value)) {
        errors[field] = rule.message || `${field} is invalid`;
      }
    }

    return {
      valid: Object.keys(errors).length === 0,
      errors
    };
  }

  /**
   * Show field error
   */
  static showError(inputElement, message) {
    inputElement.classList.add('is-error');

    let errorEl = inputElement.parentElement.querySelector('.form-error');
    if (!errorEl) {
      errorEl = document.createElement('p');
      errorEl.className = 'form-error';
      inputElement.parentElement.appendChild(errorEl);
    }
    errorEl.textContent = message;
  }

  /**
   * Clear field error
   */
  static clearError(inputElement) {
    inputElement.classList.remove('is-error');
    const errorEl = inputElement.parentElement.querySelector('.form-error');
    if (errorEl) {
      errorEl.remove();
    }
  }

  /**
   * Escape HTML to prevent XSS
   */
  static escapeHtml(text) {
    if (typeof text !== 'string') return text;
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

window.Forms = Forms;
