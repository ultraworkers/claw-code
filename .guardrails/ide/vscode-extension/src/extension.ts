import * as vscode from 'vscode';
import { GuardrailClient } from './utils/client';
import { GuardrailDiagnostics } from './providers/diagnostics';
import { GuardrailStatusBar } from './providers/statusBar';
import { registerCommands } from './commands';

let client: GuardrailClient;
let diagnostics: GuardrailDiagnostics;
let statusBar: GuardrailStatusBar;
let outputChannel: vscode.OutputChannel;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    outputChannel = vscode.window.createOutputChannel('Guardrail');
    outputChannel.appendLine('Guardrail extension activated');

    client = new GuardrailClient(outputChannel);
    diagnostics = new GuardrailDiagnostics(client, outputChannel);
    statusBar = new GuardrailStatusBar(client);

    registerCommands(context, client, diagnostics, statusBar, outputChannel);
    registerEventHandlers(context, client, diagnostics);

    context.subscriptions.push(
        diagnostics.getProvider(),
        statusBar.getItem()
    );

    await testConnection();

    outputChannel.appendLine('Guardrail extension ready');
}

export function deactivate(): void {
    outputChannel?.appendLine('Guardrail extension deactivated');
    client?.dispose();
    diagnostics?.dispose();
    statusBar?.dispose();
}

function registerEventHandlers(
    context: vscode.ExtensionContext,
    client: GuardrailClient,
    diagnostics: GuardrailDiagnostics
): void {
    const config = vscode.workspace.getConfiguration('guardrail');

    if (config.get<boolean>('validateOnSave', true)) {
        context.subscriptions.push(
            vscode.workspace.onDidSaveTextDocument(async (document) => {
                await diagnostics.validateDocument(document);
            })
        );
    }

    if (config.get<boolean>('validateOnType', false)) {
        let timeout: NodeJS.Timeout;
        context.subscriptions.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                clearTimeout(timeout);
                timeout = setTimeout(async () => {
                    await diagnostics.validateDocument(event.document);
                }, 500);
            })
        );
    }

    context.subscriptions.push(
        vscode.workspace.onDidCloseTextDocument((document) => {
            diagnostics.clear(document.uri);
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('guardrail')) {
                await client.updateConfiguration();
                await testConnection();
            }
        })
    );
}

async function testConnection(): Promise<void> {
    try {
        const connected = await client.testConnection();
        if (connected) {
            statusBar.setConnected();
        } else {
            statusBar.setDisconnected('Connection failed');
        }
    } catch (error) {
        statusBar.setDisconnected(error instanceof Error ? error.message : 'Unknown error');
    }
}
