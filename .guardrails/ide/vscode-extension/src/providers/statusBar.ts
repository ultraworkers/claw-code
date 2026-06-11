import * as vscode from 'vscode';
import { GuardrailClient } from '../utils/client';

export class GuardrailStatusBar {
    private client: GuardrailClient;
    private statusBarItem: vscode.StatusBarItem;
    private isConnected: boolean;

    constructor(client: GuardrailClient) {
        this.client = client;
        this.isConnected = false;
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'guardrail.configure';
        this.updateStatus();
        this.statusBarItem.show();
    }

    setConnected(): void {
        this.isConnected = true;
        this.updateStatus();
    }

    setDisconnected(reason?: string): void {
        this.isConnected = false;
        this.updateStatus(reason);
    }

    getItem(): vscode.StatusBarItem {
        return this.statusBarItem;
    }

    dispose(): void {
        this.statusBarItem.dispose();
    }

    private updateStatus(reason?: string): void {
        const enabled = this.client.isEnabled();

        if (!enabled) {
            this.statusBarItem.text = '$(circle-slash) Guardrail';
            this.statusBarItem.tooltip = 'Guardrail is disabled';
            this.statusBarItem.backgroundColor = undefined;
            return;
        }

        if (this.isConnected) {
            this.statusBarItem.text = '$(shield) Guardrail';
            this.statusBarItem.tooltip = 'Guardrail connected - Click to configure';
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.statusBarItem.text = '$(shield-x) Guardrail';
            this.statusBarItem.tooltip = reason 
                ? `Guardrail disconnected: ${reason} - Click to configure`
                : 'Guardrail disconnected - Click to configure';
            this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        }
    }
}
