/**
 * Navigation Component
 * Sidebar navigation with icons, active state highlighting, and mobile support
 */

class Navigation {
  constructor(container, options = {}) {
    this.container = container;
    this.options = {
      currentPath: options.currentPath || '/',
      version: options.version || '1.0.0',
      ...options
    };

    this.items = [
      { path: '/', label: 'Dashboard', icon: this.icons.dashboard },
      { path: '/documents', label: 'Documents', icon: this.icons.documents },
      { path: '/rules', label: 'Rules', icon: this.icons.rules },
      { path: '/projects', label: 'Projects', icon: this.icons.projects },
      { path: '/failures', label: 'Failures', icon: this.icons.failures },
      { path: '/ide-tools', label: 'IDE Tools', icon: this.icons.ide }
    ];

    this.render();
  }

  get icons() {
    return {
      dashboard: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>',
      documents: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>',
      rules: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>',
      projects: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>',
      failures: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
      ide: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>',
      menu: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/></svg>'
    };
  }

  render() {
    this.container.innerHTML = `
      <aside class="sidebar" id="sidebar">
        <div class="sidebar-header">
          <a href="#/" class="sidebar-logo">
            <div class="sidebar-logo-icon">G</div>
            <span class="sidebar-logo-text">Guardrail</span>
          </a>
        </div>
        <nav class="sidebar-nav">
          <ul class="sidebar-nav-list">
            ${this.items.map(item => this.renderNavItem(item)).join('')}
          </ul>
        </nav>
        <div class="sidebar-footer">
          <div class="sidebar-version">v${this.options.version}</div>
        </div>
      </aside>
      <button class="mobile-menu-toggle" id="mobile-menu-toggle" aria-label="Toggle menu">
        ${this.icons.menu}
      </button>
    `;

    this.attachEvents();
    this.addMobileStyles();
  }

  renderNavItem(item) {
    const isActive = this.options.currentPath === item.path;
    return `
      <li class="sidebar-nav-item">
        <a href="#${item.path}"
           class="sidebar-nav-link ${isActive ? 'active' : ''}"
           data-path="${item.path}">
          <span class="icon">${item.icon}</span>
          <span>${item.label}</span>
        </a>
      </li>
    `;
  }

  attachEvents() {
    // Mobile menu toggle
    const toggle = this.container.querySelector('#mobile-menu-toggle');
    const sidebar = this.container.querySelector('#sidebar');

    if (toggle) {
      toggle.addEventListener('click', () => {
        sidebar.classList.toggle('open');
      });
    }

    // Close mobile menu on navigation
    const links = this.container.querySelectorAll('.sidebar-nav-link');
    links.forEach(link => {
      link.addEventListener('click', () => {
        if (window.innerWidth <= 768) {
          sidebar.classList.remove('open');
        }
      });
    });

    // Close mobile menu when clicking outside
    document.addEventListener('click', (e) => {
      if (window.innerWidth <= 768 &&
          sidebar.classList.contains('open') &&
          !sidebar.contains(e.target) &&
          !toggle.contains(e.target)) {
        sidebar.classList.remove('open');
      }
    });
  }

  updateActivePath(path) {
    this.options.currentPath = path;
    const links = this.container.querySelectorAll('.sidebar-nav-link');
    links.forEach(link => {
      link.classList.toggle('active', link.dataset.path === path);
    });
  }

  addMobileStyles() {
    if (!document.getElementById('mobile-nav-styles')) {
      const styles = document.createElement('style');
      styles.id = 'mobile-nav-styles';
      styles.textContent = `
        @media (max-width: 768px) {
          :root {
            --sidebar-width-mobile: 280px;
          }

          .sidebar {
            transform: translateX(-100%);
            transition: transform 0.3s ease;
            width: var(--sidebar-width-mobile);
          }

          .sidebar.open {
            transform: translateX(0);
          }

          .mobile-menu-toggle {
            display: flex;
            align-items: center;
            justify-content: center;
            position: fixed;
            top: var(--space-4);
            left: var(--space-4);
            z-index: calc(var(--z-sticky) + 1);
            width: 40px;
            height: 40px;
            background-color: var(--color-surface);
            border: 1px solid var(--color-border);
            border-radius: var(--radius-md);
            cursor: pointer;
            color: var(--color-text-primary);
          }

          .mobile-menu-toggle:hover {
            background-color: var(--color-surface-hover);
          }

          .mobile-menu-toggle svg {
            width: 20px;
            height: 20px;
          }

          .main {
            margin-left: 0;
          }

          .header {
            padding-left: calc(40px + var(--space-4) + var(--space-4));
          }
        }

        @media (min-width: 769px) {
          .mobile-menu-toggle {
            display: none;
          }
        }
      `;
      document.head.appendChild(styles);
    }
  }
}

window.Navigation = Navigation;
