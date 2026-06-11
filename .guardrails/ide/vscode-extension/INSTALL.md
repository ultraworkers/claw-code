# Guardrail VS Code Extension - Installation Guide

## Prerequisites

- VS Code 1.60.0 or higher
- Node.js 16.x or higher (for building from source)
- Running Guardrail MCP Server (default: http://localhost:8095)

## Installation Methods

### Method 1: From VS Code Marketplace (Recommended)

1. Open VS Code
2. Go to Extensions view (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "Guardrail"
4. Click **Install**

### Method 2: From VSIX File

1. Download the `.vsix` file from [Releases](https://github.com/TheArchitectit/agent-guardrails-template/releases)
2. In VS Code, go to Extensions view
3. Click `...` (More Actions) → **Install from VSIX...**
4. Select the downloaded `.vsix` file

### Method 3: Build from Source

```bash
cd ide/vscode-extension
npm install
npm run compile
npm run package
```

The `.vsix` file will be created in the current directory. Install it using Method 2.

## Development Mode

For development and testing:

```bash
cd ide/vscode-extension
npm install
npm run compile
```

Press `F5` in VS Code to launch Extension Development Host.

## Configuration

After installation, configure the extension:

1. Open Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Run **Guardrail: Configure Connection**
3. Set:
   - **Server URL**: Your MCP server URL (default: http://localhost:8095)
   - **API Key**: Your authentication key (if required)
   - **Project Slug**: Your project identifier

Or edit `settings.json` directly:

```json
{
  "guardrail.enabled": true,
  "guardrail.serverUrl": "http://localhost:8095",
  "guardrail.apiKey": "your-api-key",
  "guardrail.projectSlug": "your-project",
  "guardrail.validateOnSave": true
}
```

## Verify Installation

1. Open Command Palette
2. Run **Guardrail: Test Connection**
3. Check Output panel (View → Output → Guardrail) for status

## Keyboard Shortcuts

Default shortcuts:
- `Ctrl+Shift+G` (Mac: `Cmd+Shift+G`): Validate current file

Customize in Keyboard Shortcuts settings.

## Troubleshooting

### Extension not activating
- Check VS Code version (must be >= 1.60.0)
- Reload window: Command Palette → "Developer: Reload Window"

### Connection failed
- Verify MCP server is running
- Check server URL in settings
- Check Output panel for error details

### Validation not working
- Ensure `guardrail.enabled` is true
- Check `guardrail.projectSlug` is set correctly
- Verify file is within project scope

## Security Notes

> **API Key Storage:** VS Code extension uses the SecretStorage API to securely store your API key. The key is encrypted at rest and never stored in plain text settings.
>
> **HTTPS Recommended:** For production use, ensure your MCP server uses HTTPS.
