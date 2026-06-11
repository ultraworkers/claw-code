/**
 * Router
 * Hash-based SPA routing with navigation state management
 */

class Router {
  constructor() {
    this.routes = {
      '/': {
        component: 'Dashboard',
        title: 'Dashboard'
      },
      '/documents': {
        component: 'Documents',
        title: 'Documents'
      },
      '/rules': {
        component: 'Rules',
        title: 'Prevention Rules'
      },
      '/projects': {
        component: 'Projects',
        title: 'Projects'
      },
      '/failures': {
        component: 'Failures',
        title: 'Failure Registry'
      },
      '/ide-tools': {
        component: 'IDETools',
        title: 'IDE Tools'
      }
    };

    this.currentPage = null;
    this.currentPath = '/';
    this.navigation = null;
    this.mainContainer = null;

    this.init();
  }

  init() {
    // Listen for hash changes
    window.addEventListener('hashchange', () => this.handleRoute());

    // Handle initial route after DOM is ready
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => this.handleRoute());
    } else {
      this.handleRoute();
    }
  }

  setNavigation(navigation) {
    this.navigation = navigation;
  }

  setMainContainer(container) {
    this.mainContainer = container;
  }

  handleRoute() {
    const hash = window.location.hash.slice(1) || '/';
    const route = this.routes[hash];

    if (route) {
      this.currentPath = hash;
      this.loadPage(route);

      // Update navigation active state
      if (this.navigation) {
        this.navigation.updateActivePath(hash);
      }

      // Update page title
      document.title = `${route.title} | Guardrail`;

      // Scroll to top
      window.scrollTo(0, 0);
    } else {
      // Unknown route, redirect to dashboard
      this.navigate('/');
    }
  }

  loadPage(route) {
    if (!this.mainContainer) return;

    // Clear current page
    this.mainContainer.innerHTML = '';

    // Create header
    const header = document.createElement('header');
    header.className = 'header';
    header.innerHTML = `
      <h1 class="header-title">${route.title}</h1>
      <div class="header-actions">
        <div id="api-key-status" style="display: flex; align-items: center; gap: var(--space-2);">
          ${this.renderApiKeyStatus()}
        </div>
      </div>
    `;
    this.mainContainer.appendChild(header);

    // Create page container
    const pageContainer = document.createElement('div');
    pageContainer.id = 'page-content';
    this.mainContainer.appendChild(pageContainer);

    // Initialize page component
    const PageClass = window[route.component];
    if (PageClass) {
      try {
        this.currentPage = new PageClass(pageContainer);
      } catch (error) {
        console.error('Failed to load page:', error);
        pageContainer.innerHTML = `
          <div class="empty-state" style="padding: var(--space-16);">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="15" y1="9" x2="9" y2="15"/>
              <line x1="9" y1="9" x2="15" y2="15"/>
            </svg>
            <h3 class="empty-state-title">Error Loading Page</h3>
            <p class="empty-state-description">${error.message}</p>
            <button class="btn btn-secondary" onclick="window.router.navigate('/')">Go to Dashboard</button>
          </div>
        `;
      }
    } else {
      pageContainer.innerHTML = `
        <div class="empty-state" style="padding: var(--space-16);">
          <h3 class="empty-state-title">Page Not Found</h3>
          <p class="empty-state-description">The requested page could not be loaded.</p>
          <button class="btn btn-secondary" onclick="window.router.navigate('/')">Go to Dashboard</button>
        </div>
      `;
    }
  }

  renderApiKeyStatus() {
    const hasKey = !!window.api.apiKey;
    return `
      <span style="
        width: 8px;
        height: 8px;
        border-radius: var(--radius-full);
        background-color: ${hasKey ? 'var(--color-success)' : 'var(--color-warning)'}
      "></span>
      <span style="font-size: var(--text-sm); color: var(--color-text-secondary);">
        ${hasKey ? 'API Key Set' : 'No API Key'}
      </span>
      <button class="btn btn-sm btn-ghost" id="change-api-key-btn">
        ${hasKey ? 'Change' : 'Set Key'}
      </button>
    `;
  }

  navigate(path) {
    window.location.hash = path;
  }

  getCurrentPath() {
    return this.currentPath;
  }

  refresh() {
    this.handleRoute();
  }
}

window.Router = Router;
window.router = new Router();
