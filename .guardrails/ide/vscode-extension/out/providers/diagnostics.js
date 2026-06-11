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
exports.GuardrailDiagnostics = void 0;
const vscode = __importStar(require("vscode"));
class GuardrailDiagnostics {
    constructor(client, outputChannel) {
        this.client = client;
        this.outputChannel = outputChannel;
        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('guardrail');
    }
    async validateDocument(document) {
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
            this.outputChannel.appendLine(`Validated ${document.fileName}: ${response.violations.length} violation(s)`);
        }
        catch (error) {
            this.outputChannel.appendLine(`Validation error for ${document.fileName}: ${error instanceof Error ? error.message : 'Unknown error'}`);
            this.diagnosticCollection.delete(document.uri);
        }
    }
    clear(uri) {
        this.diagnosticCollection.delete(uri);
    }
    getProvider() {
        return this.diagnosticCollection;
    }
    dispose() {
        this.diagnosticCollection.dispose();
    }
    convertToDiagnostics(violations) {
        return violations.map(violation => {
            const range = new vscode.Range(violation.line - 1, violation.column - 1, violation.line - 1, violation.column);
            const severity = this.mapSeverity(violation.severity);
            const diagnostic = new vscode.Diagnostic(range, violation.message, severity);
            diagnostic.code = violation.rule_id;
            diagnostic.source = 'Guardrail';
            if (violation.suggestion) {
                diagnostic.relatedInformation = [
                    new vscode.DiagnosticRelatedInformation(new vscode.Location(vscode.Uri.file(''), range), `Suggestion: ${violation.suggestion}`)
                ];
            }
            return diagnostic;
        });
    }
    mapSeverity(severity) {
        const config = this.client.getConfig();
        const threshold = config.severityThreshold;
        const levels = {
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
    mapLanguage(vscodeLanguageId) {
        const mapping = {
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
exports.GuardrailDiagnostics = GuardrailDiagnostics;
//# sourceMappingURL=diagnostics.js.map