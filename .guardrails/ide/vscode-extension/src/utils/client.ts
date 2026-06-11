import * as vscode from 'vscode';
import { ValidationRequest, ValidationResponse, GuardrailConfig } from '../types';

export class GuardrailClient {
    private config!: GuardrailConfig;
    private outputChannel: vscode.OutputChannel;
    private context: vscode.ExtensionContext | undefined;

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
    }

    async updateConfiguration(context?: vscode.ExtensionContext): Promise<void> {
        this.context = context;
        const config = vscode.workspace.getConfiguration('guardrail');
        // Get API key from SecretStorage if available
        let apiKey = '';
        if (context) {
            apiKey = await context.secrets.get('guardrail.apiKey') || '';
        }
        this.config = {
            enabled: config.get<boolean>('enabled', true),
            serverUrl: config.get<string>('serverUrl', 'http://localhost:8095'),
            apiKey: apiKey,
            projectSlug: config.get<string>('projectSlug', ''),
            validateOnSave: config.get<boolean>('validateOnSave', true),
            validateOnType: config.get<boolean>('validateOnType', false),
            severityThreshold: config.get<'error' | 'warning' | 'info'>('severityThreshold', 'warning')
        };
    }

    async testConnection(): Promise<boolean> {
        try {
            const response = await this.fetch('/health/ready');
            return response.status === 200;
        } catch {
            return false;
        }
    }

    async validateFile(request: ValidationRequest): Promise<ValidationResponse> {
        return this.post('/ide/validate/file', request);
    }

    async validateSelection(code: string, language: string): Promise<ValidationResponse> {
        return this.post('/ide/validate/selection', { code, language });
    }

    async getRules(projectSlug?: string): Promise<unknown> {
        const url = projectSlug 
            ? `/ide/rules?project=${encodeURIComponent(projectSlug)}`
            : '/ide/rules';
        return this.get(url);
    }

    private async fetch(path: string, options?: RequestInit): Promise<Response> {
        const url = `${this.config.serverUrl}${path}`;
        const headers: Record<string, string> = {
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

    private async get(path: string): Promise<unknown> {
        const response = await this.fetch(path);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return response.json();
    }

    private async post(path: string, body: unknown): Promise<ValidationResponse> {
        const response = await this.fetch(path, {
            method: 'POST',
            body: JSON.stringify(body)
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        return response.json() as Promise<ValidationResponse>;
    }

    getConfig(): GuardrailConfig {
        return this.config;
    }

    isEnabled(): boolean {
        return this.config.enabled;
    }

    dispose(): void {}
}
