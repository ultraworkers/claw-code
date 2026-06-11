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
exports.deactivate = exports.activate = void 0;
const vscode = __importStar(require("vscode"));
const client_1 = require("./utils/client");
const diagnostics_1 = require("./providers/diagnostics");
const statusBar_1 = require("./providers/statusBar");
const commands_1 = require("./commands");
let client;
let diagnostics;
let statusBar;
let outputChannel;
async function activate(context) {
    outputChannel = vscode.window.createOutputChannel('Guardrail');
    outputChannel.appendLine('Guardrail extension activated');
    client = new client_1.GuardrailClient(outputChannel);
    diagnostics = new diagnostics_1.GuardrailDiagnostics(client, outputChannel);
    statusBar = new statusBar_1.GuardrailStatusBar(client);
    (0, commands_1.registerCommands)(context, client, diagnostics, statusBar, outputChannel);
    registerEventHandlers(context, client, diagnostics);
    context.subscriptions.push(diagnostics.getProvider(), statusBar.getItem());
    await testConnection();
    outputChannel.appendLine('Guardrail extension ready');
}
exports.activate = activate;
function deactivate() {
    outputChannel?.appendLine('Guardrail extension deactivated');
    client?.dispose();
    diagnostics?.dispose();
    statusBar?.dispose();
}
exports.deactivate = deactivate;
function registerEventHandlers(context, client, diagnostics) {
    const config = vscode.workspace.getConfiguration('guardrail');
    if (config.get('validateOnSave', true)) {
        context.subscriptions.push(vscode.workspace.onDidSaveTextDocument(async (document) => {
            await diagnostics.validateDocument(document);
        }));
    }
    if (config.get('validateOnType', false)) {
        let timeout;
        context.subscriptions.push(vscode.workspace.onDidChangeTextDocument((event) => {
            clearTimeout(timeout);
            timeout = setTimeout(async () => {
                await diagnostics.validateDocument(event.document);
            }, 500);
        }));
    }
    context.subscriptions.push(vscode.workspace.onDidCloseTextDocument((document) => {
        diagnostics.clear(document.uri);
    }));
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(async (e) => {
        if (e.affectsConfiguration('guardrail')) {
            await client.updateConfiguration();
            await testConnection();
        }
    }));
}
async function testConnection() {
    try {
        const connected = await client.testConnection();
        if (connected) {
            statusBar.setConnected();
        }
        else {
            statusBar.setDisconnected('Connection failed');
        }
    }
    catch (error) {
        statusBar.setDisconnected(error instanceof Error ? error.message : 'Unknown error');
    }
}
//# sourceMappingURL=extension.js.map