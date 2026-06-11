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
exports.registerCommands = void 0;
const vscode = __importStar(require("vscode"));
function registerCommands(context, client, diagnostics, statusBar, outputChannel) {
    context.subscriptions.push(vscode.commands.registerCommand('guardrail.validateFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('No active editor');
            return;
        }
        await diagnostics.validateDocument(editor.document);
        vscode.window.showInformationMessage('File validated');
    }), vscode.commands.registerCommand('guardrail.validateSelection', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('No active editor');
            return;
        }
        const selection = editor.selection;
        if (selection.isEmpty) {
            vscode.window.showWarningMessage('No text selected');
            return;
        }
        const text = editor.document.getText(selection);
        const language = editor.document.languageId;
        try {
            const response = await client.validateSelection(text, language);
            if (response.violations.length === 0) {
                vscode.window.showInformationMessage('Selection is valid');
            }
            else {
                const messages = response.violations.map(v => v.message).join('\n');
                vscode.window.showWarningMessage(`Violations found:\n${messages}`);
            }
        }
        catch (error) {
            vscode.window.showErrorMessage(`Validation failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    }), vscode.commands.registerCommand('guardrail.configure', async () => {
        const config = vscode.workspace.getConfiguration('guardrail');
        const serverUrl = await vscode.window.showInputBox({
            prompt: 'Guardrail MCP Server URL',
            value: config.get('serverUrl', 'http://localhost:8095'),
            validateInput: (value) => {
                try {
                    new URL(value);
                    return null;
                }
                catch {
                    return 'Please enter a valid URL';
                }
            }
        });
        if (serverUrl === undefined)
            return;
        const apiKey = await vscode.window.showInputBox({
            prompt: 'API Key (leave empty to use current)',
            password: true,
            value: config.get('apiKey', '')
        });
        if (apiKey === undefined)
            return;
        const projectSlug = await vscode.window.showInputBox({
            prompt: 'Project Slug (optional)',
            value: config.get('projectSlug', '')
        });
        if (projectSlug === undefined)
            return;
        await config.update('serverUrl', serverUrl, true);
        await config.update('projectSlug', projectSlug, true);
        // Store API key in SecretStorage, not plain settings
        if (apiKey !== undefined) {
            await context.secrets.store('guardrail.apiKey', apiKey);
        }
        client.updateConfiguration(context);
        const connected = await client.testConnection();
        if (connected) {
            statusBar.setConnected();
            vscode.window.showInformationMessage('Guardrail configured and connected');
        }
        else {
            statusBar.setDisconnected('Connection failed');
            vscode.window.showErrorMessage('Failed to connect to Guardrail server');
        }
    }), vscode.commands.registerCommand('guardrail.showOutput', () => {
        outputChannel.show();
    }), vscode.commands.registerCommand('guardrail.testConnection', async () => {
        const connected = await client.testConnection();
        if (connected) {
            statusBar.setConnected();
            vscode.window.showInformationMessage('Connected to Guardrail server');
        }
        else {
            statusBar.setDisconnected('Connection failed');
            vscode.window.showErrorMessage('Failed to connect to Guardrail server');
        }
    }));
}
exports.registerCommands = registerCommands;
//# sourceMappingURL=commands.js.map