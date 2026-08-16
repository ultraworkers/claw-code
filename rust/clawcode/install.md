# Claw Code Installation Guide

> One-click installation instructions for Windows users. The entire process is: download, extract, and double-click a single `.bat` file.

## 1. Download

Get the two files from [GitHub Releases](https://github.com/huagusam/clawcode/releases/latest):

| File | Download URL | Description |
|---|---|---|
| `Config_methods.7z` | [Download here](https://github.com/huagusam/clawcode/releases/download/v0.2.2.2/Config_methods.7z) | **Full installer package** — includes `claw.exe`, Git, fd, rg, config files, and the installation script |
| `claw.exe` | [Download here](https://github.com/huagusam/clawcode/releases/download/v0.2.2.1/claw.exe) | Standalone main binary (optional; already bundled in the installer package) |

> We recommend simply downloading **`Config_methods.7z`** — a single file completes the full installation.

## 2. Extract

1. Right-click `Config_methods.7z` → **Extract All** (built into Windows; install [7-Zip](https://www.7-zip.org/) if not available)
2. After extraction you get the `Install_Config_methods` folder containing:
   - `claw.exe` — main binary
   - `Git.7z` — offline Git Bash installer
   - `fd.exe` / `rg.exe` — search tools
   - `.claw/` — configuration directory
   - `install_claw.bat` — **one-click installation script**

> Note: the folder path must **not contain non-ASCII characters**, e.g. put it at `D:\claw\Install_Config_methods`.

## 3. One-Click Install

1. Enter the extracted `Install_Config_methods` folder
2. **Double-click `install_claw.bat`** and accept the administrator prompt (click "Yes" on the UAC dialog)
3. The script will automatically complete:

| Step | Action |
|---|---|
| 1/5 | Detect Git Bash: skip if installed, otherwise extract `Git.7z` to `C:\Program Files\Git` |
| 2/5 | Copy `fd.exe` and `rg.exe` to `C:\Program Files\Git\bin` |
| 3/5 | Copy `claw.exe` to `C:\Users\<your-username>\.local\bin` and create a `claw` shortcut on the desktop |
| 4/5 | Copy the `.claw` config folder to `C:\Users\<your-username>\.claw` (overwrites old config) |
| 5/5 | Add `C:\Program Files\Git\bin` and `.local\bin` to the system PATH |

You will see **"Installation finished"** once the installation succeeds.

## 4. Getting Started

1. **Reopen** a new terminal window (cmd / PowerShell / Windows Terminal) so the PATH takes effect
2. Double-click the **`claw`** shortcut on the desktop, or type `claw` and press Enter in a terminal
3. On first use, configure the API: edit `C:\Users\<your-username>\.claw\.env` and fill in your API Key and model:

```env
ANTHROPIC_BASE_URL=https://api.anthropic.com
ANTHROPIC_API_KEY=sk-ant-xxxxxxxx
ANTHROPIC_MODEL=claude-sonnet-4-20250514
```

> For local models (LM Studio / llama.cpp / Ollama): `ANTHROPIC_BASE_URL` only needs the server address (**do not** add `/v1` — claw automatically appends `/v1/messages`). The port varies by service: LM Studio `1234`, llama-server `8080`, Ollama `11434`.

## 5. FAQ

| Problem | Solution |
|---|---|
| The window flashes and closes after double-clicking the bat | Right-click `install_claw.bat` → Run as administrator |
| "7-Zip not found" error | Install [7-Zip](https://www.7-zip.org/) and rerun the script |
| `claw` command not found | Confirm the PATH has taken effect, or reopen the terminal and try again |
| No desktop shortcut | Check the installation log, or manually create a shortcut pointing to `C:\Users\<your-username>\.local\bin\claw.exe` |
| How to uninstall | Delete `C:\Users\<your-username>\.local\bin\claw.exe`, `C:\Users\<your-username>\.claw`, and the desktop shortcut |

## 6. Building from Source (Optional)

Requires a Rust + MSVC + Clang-CL environment; see the project [README](README.md).

## License

MIT
