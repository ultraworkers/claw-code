import * as vscode from 'vscode';
import { GuardrailClient } from '../utils/client';
import { Violation } from '../types';

export class GuardrailDiagnostics {
    private client: GuardrailClient;
    private outputChannel: vscode.OutputChannel;
    private diagnosticCollection: vscode.DiagnosticCollection;

    constructor(client: GuardrailClient, outputChannel: vscode.OutputChannel) {
        this.client = client;
        this.outputChannel = outputChannel;
        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('guardrail');
    }

    async validateDocument(document: vscode.TextDocument): Promise<void> {
        if (!this.client.isEnabled()) {
            return;
        }

        const config = this.client.getConfig();
        const language = this.mapLanguage(document.languageId);

        try {
            const response = await this.client.validateFile({
                file_path: document.fileName,
                content: document.getText(),
                language: language,
                project_slug: config.projectSlug || undefined
            });

            const diagnostics = this.convertToDiagnostics(response.violations);
            this.diagnosticCollection.set(document.uri, diagnostics);

            this.outputChannel.appendLine(
                `Validated ${document.fileName}: ${response.violations.length} violation(s)`
            );
        } catch (error) {
            this.outputChannel.appendLine(
                `Validation error for ${document.fileName}: ${error instanceof Error ? error.message : 'Unknown error'}`
            );
            this.diagnosticCollection.delete(document.uri);
        }
    }

    clear(uri: vscode.Uri): void {
        this.diagnosticCollection.delete(uri);
    }

    getProvider(): vscode.DiagnosticCollection {
        return this.diagnosticCollection;
    }

    dispose(): void {
        this.diagnosticCollection.dispose();
    }

    private convertToDiagnostics(violations: Violation[]): vscode.Diagnostic[] {
        return violations.map(violation => {
            const range = new vscode.Range(
                violation.line - 1,
                violation.column - 1,
                violation.line - 1,
                violation.column
            );

            const severity = this.mapSeverity(violation.severity);
            const diagnostic = new vscode.Diagnostic(
                range,
                violation.message,
                severity
            );

            diagnostic.code = violation.rule_id;
            diagnostic.source = 'Guardrail';

            if (violation.suggestion) {
                diagnostic.relatedInformation = [
                    new vscode.DiagnosticRelatedInformation(
                        new vscode.Location(vscode.Uri.file(''), range),
                        `Suggestion: ${violation.suggestion}`
                    )
                ];
            }

            return diagnostic;
        });
    }

    private mapSeverity(severity: Violation['severity']): vscode.DiagnosticSeverity {
        const config = this.client.getConfig();
        const threshold = config.severityThreshold;

        const levels: Record<string, number> = {
            'error': 3,
            'warning': 2,
            'info': 1
        };

        if (levels[severity] < levels[threshold]) {
            return vscode.DiagnosticSeverity.Hint;
        }

        switch (severity) {
            case 'error':
                return vscode.DiagnosticSeverity.Error;
            case 'warning':
                return vscode.DiagnosticSeverity.Warning;
            case 'info':
                return vscode.DiagnosticSeverity.Information;
            default:
                return vscode.DiagnosticSeverity.Hint;
        }
    }

    private mapLanguage(vscodeLanguageId: string): string {
        const mapping: Record<string, string> = {
            'javascript': 'javascript',
            'typescript': 'javascript',
            'python': 'python',
            'go': 'go',
            'rust': 'rust',
            'java': 'java',
            'kotlin': 'kotlin',
            'bash': 'bash',
            'shellscript': 'bash',
            'yaml': 'yaml',
            'json': 'json',
            'markdown': 'markdown',
            'dockerfile': 'dockerfile'
        };

        return mapping[vscodeLanguageId] || vscodeLanguageId;
    }
}
