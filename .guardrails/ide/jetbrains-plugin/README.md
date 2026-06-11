# Guardrail JetBrains Plugin

Real-time guardrail validation for IntelliJ IDEA, PyCharm, and other JetBrains IDEs.

## Features

- **Inline Diagnostics**: Highlight violations as you code
- **Inspections**: IntelliJ-native code inspections with quick fixes
- **Status Bar**: Connection status to MCP server
- **File Actions**: Validate file or selection on demand

## Installation

### From JetBrains Marketplace (Coming Soon)

1. Open IDE → Settings/Preferences → Plugins
2. Go to Marketplace tab
3. Search for "Guardrail"
4. Click Install

### From Disk (Manual Install)

1. Build or download the plugin ZIP:
   ```bash
   ./gradlew buildPlugin
   ```
2. In IDE: Settings → Plugins → Gear Icon → Install from Disk
3. Select `build/distributions/guardrail-plugin-*.zip`
4. Restart IDE

### Build from Source

Requirements:
- JDK 17 or higher
- IntelliJ IDEA Community or Ultimate

```bash
cd ide/jetbrains-plugin
./gradlew buildPlugin
```

Plugin ZIP will be in `build/distributions/`.

## Development

1. Open `ide/jetbrains-plugin` in IntelliJ IDEA
2. Run: `./gradlew runIde`

This launches a sandbox IDE with the plugin loaded.

## Configuration

Settings → Tools → Guardrail:

| Setting | Default | Description |
|---------|---------|-------------|
| Server URL | http://localhost:8095 | MCP server endpoint |
| API Key | (empty) | Authentication key |
| Project Slug | (empty) | Project identifier |
| Validate on Save | true | Run validation on file save |

## Usage

- **Validate File**: Tools → Guardrail → Validate File
- **Validate Selection**: Editor → Right-click → Guardrail → Validate Selection
- **Test Connection**: Tools → Guardrail → Test Connection

## Supported IDEs

- IntelliJ IDEA (Community/Ultimate) 2022.3+
- PyCharm (Community/Professional) 2022.3+
- WebStorm 2022.3+
- GoLand 2022.3+
- PhpStorm 2022.3+
- RubyMine 2022.3+
- CLion 2022.3+

## Security Notes

> **API Key Storage:** The API key is masked in the settings UI using JPasswordField. For production use, consider using IDE password safe storage.
>
> **HTTPS Recommended:** Always use HTTPS when connecting to remote MCP servers.

## Troubleshooting

Plugin not loading? Check:
- IDE version compatibility
- JDK version (must be 17+)
- Plugin compatibility range in `build.gradle.kts`
