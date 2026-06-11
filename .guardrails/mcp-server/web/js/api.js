/**
 * Guardrail API Client
 * Handles all 26 API endpoints with authentication, error handling, and response parsing
 */

class GuardrailAPI {
  constructor(baseURL = '') {
    this.baseURL = baseURL;
    this.apiKey = localStorage.getItem('guardrail_api_key') || '';
  }

  setApiKey(key) {
    this.apiKey = key;
    localStorage.setItem('guardrail_api_key', key);
  }

  clearApiKey() {
    this.apiKey = '';
    localStorage.removeItem('guardrail_api_key');
  }

  /**
   * Validate the current API key by making a test request
   * @returns {Promise<{valid: boolean, error?: string}>}
   */
  async validateApiKey() {
    if (!this.apiKey) {
      return { valid: false, error: 'No API key configured' };
    }
    try {
      // Use a lightweight endpoint to validate the key
      await this.getStats();
      return { valid: true };
    } catch (error) {
      if (error.message.includes('401') || error.message.includes('Unauthorized')) {
        return { valid: false, error: 'Invalid API key' };
      }
      if (error.message.includes('Network error')) {
        return { valid: false, error: 'Unable to connect to server. Please check your network.' };
      }
      return { valid: false, error: error.message };
    }
  }

  /**
   * Base request method with error handling
   */
  async request(endpoint, options = {}) {
    const url = `${this.baseURL}${endpoint}`;
    const headers = {
      'Content-Type': 'application/json',
      ...(this.apiKey && { 'Authorization': `Bearer ${this.apiKey}` }),
      ...options.headers
    };

    try {
      const response = await fetch(url, { ...options, headers });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}: ${response.statusText}` }));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      if (response.status === 204) {
        return null;
      }

      return await response.json();
    } catch (error) {
      if (error.message.includes('Failed to fetch')) {
        throw new Error('Network error: Unable to connect to server');
      }
      throw error;
    }
  }

  // ==================== HEALTH ENDPOINTS ====================

  async getHealthLive() {
    return this.request('/health/live');
  }

  async getHealthReady() {
    return this.request('/health/ready');
  }

  async getVersion() {
    return this.request('/version');
  }

  async getStats() {
    return this.request('/api/stats');
  }

  // ==================== DOCUMENTS ENDPOINTS ====================

  async listDocuments(params = {}) {
    const query = new URLSearchParams();
    if (params.category) query.append('category', params.category);
    if (params.limit) query.append('limit', params.limit);
    if (params.offset) query.append('offset', params.offset);

    const queryString = query.toString();
    return this.request(`/api/documents${queryString ? `?${queryString}` : ''}`);
  }

  async getDocument(id) {
    return this.request(`/api/documents/${id}`);
  }

  async updateDocument(id, data) {
    return this.request(`/api/documents/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  async searchDocuments(query, limit = 20) {
    const params = new URLSearchParams({ q: query });
    if (limit) params.append('limit', limit);
    return this.request(`/api/documents/search?${params.toString()}`);
  }

  // ==================== RULES ENDPOINTS ====================

  async listRules(params = {}) {
    const query = new URLSearchParams();
    if (params.enabled !== undefined) query.append('enabled', params.enabled);
    if (params.category) query.append('category', params.category);
    if (params.limit) query.append('limit', params.limit);
    if (params.offset) query.append('offset', params.offset);

    const queryString = query.toString();
    return this.request(`/api/rules${queryString ? `?${queryString}` : ''}`);
  }

  async getRule(id) {
    return this.request(`/api/rules/${id}`);
  }

  async createRule(data) {
    return this.request('/api/rules', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  async updateRule(id, data) {
    return this.request(`/api/rules/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  async deleteRule(id) {
    return this.request(`/api/rules/${id}`, {
      method: 'DELETE'
    });
  }

  async toggleRule(id, enabled) {
    return this.request(`/api/rules/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ enabled })
    });
  }

  // ==================== PROJECTS ENDPOINTS ====================

  async listProjects(params = {}) {
    const query = new URLSearchParams();
    if (params.limit) query.append('limit', params.limit);
    if (params.offset) query.append('offset', params.offset);

    const queryString = query.toString();
    return this.request(`/api/projects${queryString ? `?${queryString}` : ''}`);
  }

  async getProject(id) {
    return this.request(`/api/projects/${id}`);
  }

  async createProject(data) {
    return this.request('/api/projects', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  async updateProject(id, data) {
    return this.request(`/api/projects/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  async deleteProject(id) {
    return this.request(`/api/projects/${id}`, {
      method: 'DELETE'
    });
  }

  // ==================== FAILURES ENDPOINTS ====================

  async listFailures(params = {}) {
    const query = new URLSearchParams();
    if (params.status) query.append('status', params.status);
    if (params.category) query.append('category', params.category);
    if (params.project) query.append('project', params.project);
    if (params.limit) query.append('limit', params.limit);
    if (params.offset) query.append('offset', params.offset);

    const queryString = query.toString();
    return this.request(`/api/failures${queryString ? `?${queryString}` : ''}`);
  }

  async getFailure(id) {
    return this.request(`/api/failures/${id}`);
  }

  async updateFailure(id, data) {
    return this.request(`/api/failures/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  async createFailure(data) {
    return this.request('/api/failures', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  // ==================== IDE TOOLS ENDPOINTS ====================

  async getIDEHealth() {
    return this.request('/ide/health');
  }

  async getIDERules(project) {
    const params = new URLSearchParams();
    if (project) params.append('project', project);
    return this.request(`/ide/rules?${params.toString()}`);
  }

  async validateFile(data) {
    return this.request('/ide/validate/file', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  async validateSelection(data) {
    return this.request('/ide/validate/selection', {
      method: 'POST',
      body: JSON.stringify(data)
    });
  }

  async getQuickReference() {
    return this.request('/ide/quick-reference');
  }

  // ==================== SYSTEM ENDPOINTS ====================

  async triggerIngest() {
    return this.request('/api/ingest', {
      method: 'POST'
    });
  }

  // ==================== UPDATE ENDPOINTS ====================

  async getUpdateStatus() {
    return this.request('/api/updates/status');
  }

  async syncGuardrails() {
    return this.request('/api/ingest/sync', {
      method: 'POST'
    });
  }

  // ==================== FILE UPLOAD ENDPOINTS ====================

  async uploadFiles(files, onProgress = null) {
    const formData = new FormData();
    for (const file of files) {
      formData.append('files', file);
    }

    const url = `${this.baseURL}/api/ingest/upload`;
    const headers = {
      ...(this.apiKey && { 'Authorization': `Bearer ${this.apiKey}` })
    };

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers,
        body: formData
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}: ${response.statusText}` }));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      if (error.message.includes('Failed to fetch')) {
        throw new Error('Network error: Unable to connect to server');
      }
      throw error;
    }
  }
}

// Create global API instance
window.api = new GuardrailAPI();
