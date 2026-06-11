"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.GuardrailClient = void 0;
const vscode = __importStar(require("vscode"));
class GuardrailClient {
    constructor(outputChannel) {
        this.outputChannel = outputChannel;
    }
    async updateConfiguration(context) {
        this.context = context;
        const config = vscode.workspace.getConfiguration('guardrail');
        // Get API key from SecretStorage if available
        let apiKey = '';
        if (context) {
            apiKey = await context.secrets.get('guardrail.apiKey') || '';
        }
        this.config = {
            enabled: config.get('enabled', true),
            serverUrl: config.get('serverUrl', 'http://localhost:8095'),
            apiKey: apiKey,
            projectSlug: config.get('projectSlug', ''),
            validateOnSave: config.get('validateOnSave', true),
            validateOnType: config.get('validateOnType', false),
            severityThreshold: config.get('severityThreshold', 'warning')
        };
    }
    async testConnection() {
        try {
            const response = await this.fetch('/health/ready');
            return response.status === 200;
        }
        catch {
            return false;
        }
    }
    async validateFile(request) {
        return this.post('/ide/validate/file', request);
    }
    async validateSelection(code, language) {
        return this.post('/ide/validate/selection', { code, language });
    }
    async getRules(projectSlug) {
        const url = projectSlug
            ? `/ide/rules?project=${encodeURIComponent(projectSlug)}`
            : '/ide/rules';
        return this.get(url);
    }
    async fetch(path, options) {
        const url = `${this.config.serverUrl}${path}`;
        const headers = {
            'Content-Type': 'application/json'
        };
        if (this.config.apiKey) {
            headers['Authorization'] = `Bearer ${this.config.apiKey}`;
        }
        this.outputChannel.appendLine(`Fetching: ${path}`);
        return fetch(url, {
            ...options,
            headers: {
                ...headers,
                ...options?.headers
            }
        });
    }
    async get(path) {
        const response = await this.fetch(path);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return response.json();
    }
    async post(path, body) {
        const response = await this.fetch(path, {
            method: 'POST',
            body: JSON.stringify(body)
        });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return response.json();
    }
    getConfig() {
        return this.config;
    }
    isEnabled() {
        return this.config.enabled;
    }
    dispose() { }
}
exports.GuardrailClient = GuardrailClient;
//# sourceMappingURL=client.js.map