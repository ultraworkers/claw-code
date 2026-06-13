<!-- Last synced with README.md: d229a9b -->

# Claw Code

<p align="center">
  <a href="https://github.com/code-yeongyu/lazycodex">
    <img src="https://img.shields.io/badge/LazyCodex-codex%20for%20no--brainers-111111?style=for-the-badge&logo=github&logoColor=white" alt="LazyCodex banner" />
  </a>
  <a href="https://github.com/Yeachan-Heo/gajae-code">
    <img src="https://img.shields.io/badge/Gajae--Code-red--claw%20agent%20harness-B22222?style=for-the-badge&logo=github&logoColor=white" alt="Gajae-Code banner" />
  </a>
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <strong>中文</strong>
</p>

<p align="center">
  <a href="https://github.com/code-yeongyu/lazycodex">
    <img src="https://opengraph.githubassets.com/lazycodex-card/code-yeongyu/lazycodex" alt="LazyCodex GitHub card" width="280" />
  </a>
  <a href="https://github.com/Yeachan-Heo/gajae-code">
    <img src="https://opengraph.githubassets.com/gajae-code-card/Yeachan-Heo/gajae-code" alt="Gajae-Code GitHub card" width="280" />
  </a>
</p>

<h3 align="center">start with the real crab-powered harnesses</h3>

<p align="center">
  <a href="https://github.com/code-yeongyu/lazycodex"><b>github.com/code-yeongyu/lazycodex</b></a>
  <br/>
  <a href="https://github.com/Yeachan-Heo/gajae-code"><b>github.com/Yeachan-Heo/gajae-code</b></a>
</p>

<p align="center">
  <a href="https://github.com/code-yeongyu/lazycodex">
    <img src="https://img.shields.io/badge/Open-LazyCodex-111111?style=flat-square&logo=github&logoColor=white" alt="Open LazyCodex on GitHub" />
  </a>
  <a href="https://github.com/Yeachan-Heo/gajae-code">
    <img src="https://img.shields.io/badge/Open-Gajae--Code-B22222?style=flat-square&logo=github&logoColor=white" alt="Open Gajae-Code on GitHub" />
  </a>
</p>

<p align="center">
  <a href="https://discord.gg/GtjhvgjnV">
    <img src="https://img.shields.io/badge/Discord-join%20the%20harness%20lab-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Join the harness lab on Discord" />
  </a>
  <a href="https://discord.gg/4Rt79F7dF">
    <img src="https://img.shields.io/badge/Discord-join%20the%20crab%20tank-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Join the crab tank on Discord" />
  </a>
</p>

<p align="center">
  加入 Discord：
  <a href="https://discord.gg/GtjhvgjnV"><b>ultraworkers discord</b></a>
  ·
  <a href="https://discord.gg/4Rt79F7dF"><b>gajae-code discord</b></a>
</p>

> [!IMPORTANT]
> **Claw Code 在这里并不是一个严肃的生产级项目。**
> 这个代码库更像是一个博物馆的展品，而不是一个产品推介。它是一个由螃蟹（crab）驱动的工件，由带爪子的 gajae（小龙虾）维持生命，由智能体清扫和贴标签，并根据上述的工具（harnesses）自动维护。
>
> 正如项目理念中所描述的，这并不旨在像普通产品代码库那样由人工操作。它是一个**由智能体管理的展品 (agent-managed exhibit)**：当螃蟹保持水箱运转时，工具会规划、执行、验证、标记并保存该工件。
>
> 如果你想要实际运行工作任务，请从 **[LazyCodex](https://github.com/code-yeongyu/lazycodex)** 或 **[Gajae-Code](https://github.com/Yeachan-Heo/gajae-code)** 开始。如果你想研究 Claw Code 时刻这个奇怪的小化石，请继续往下看。
>
> 有关这一理念背后更详细的公开解释，请参阅[这里](https://x.com/realsigridjin/status/2039472968624185713)。

<p align="center">
  <a href="https://github.com/ultraworkers/claw-code">ultraworkers/claw-code</a>
  ·
  <a href="./USAGE.md">使用指南 (Usage)</a>
  ·
  <a href="./rust/README.md">Rust 工作区 (Rust workspace)</a>
  ·
  <a href="./PARITY.md">一致性 (Parity)</a>
  ·
  <a href="./ROADMAP.md">路线图 (Roadmap)</a>
  ·
  <a href="./CONTRIBUTING.md">贡献指南 (Contributing)</a>
  ·
  <a href="./SECURITY.md">安全 (Security)</a>
  ·
  <a href="https://discord.gg/5TUQKqFWd">UltraWorkers Discord</a>
</p>

<p align="center">
  <a href="https://star-history.com/#ultraworkers/claw-code&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date&theme=dark" />
      <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date" />
      <img alt="Star history for ultraworkers/claw-code" src="https://api.star-history.com/svg?repos=ultraworkers/claw-code&type=Date" width="600" />
    </picture>
  </a>
</p>

<p align="center">
  <img src="assets/claw-hero.jpeg" alt="Claw Code" width="300" />
</p>

Claw Code 是 `claw` CLI 智能体工具的公开 Rust 实现。
规范的实现位于 [`rust/`](./rust)，此代码库当前的事实来源是 **ultraworkers/claw-code**。

> [!IMPORTANT]
> 首先请阅读 [`USAGE.md`](./USAGE.md) 以了解构建、认证、CLI、会话和一致性工具（parity-harness）工作流。关于文件提交/导航问题，请参阅 [导航和文件上下文 (Navigation and file context)](./docs/navigation-file-context.md)。关于本地兼容 OpenAI 的模型和离线技能设置，请参阅 [本地兼容 OpenAI 提供商及技能设置 (Local OpenAI-compatible providers and skills setup)](./docs/local-openai-compatible-providers.md)。Windows 用户可以跳转至以 PowerShell 为主的 [Windows 安装和发布快速入门 (Windows install and release quickstart)](./docs/windows-install-release.md)。构建完成后，请首先使用 `claw doctor` 进行健康检查，查看 [`rust/README.md`](./rust/README.md) 获取 crate 级别的详细信息，阅读 [`PARITY.md`](./PARITY.md) 了解当前的 Rust 移植检查点，并查阅 [`docs/container.md`](./docs/container.md) 了解优先使用容器的工作流。
>
> **ACP / Zed 状态：** `claw-code` 尚未发布 ACP/Zed 守护进程或 JSON-RPC 入口点。请运行 `claw acp`（或 `claw --acp`）查看当前状态，而不是从源代码布局中猜测；`claw acp serve` 目前仅作为可发现性的别名，返回状态并以退出码 0 结束，真正的 ACP 支持仍在 `ROADMAP.md` 中单独追踪。有关公开的 JSON 契约，请参阅 [`docs/g011-acp-json-rpc-status-contract.md`](./docs/g011-acp-json-rpc-status-contract.md)。

## 当前代码库结构

- **`rust/`** — 规范的 Rust 工作区和 `claw` CLI 二进制文件
- **`USAGE.md`** — 针对当前产品表面的面向任务的使用指南
- **`PARITY.md`** — Rust 移植一致性状态和迁移说明
- **`ROADMAP.md`** — 活跃的路线图和清理待办事项
- **`PHILOSOPHY.md`** — 项目意图和系统设计框架
- **`src/` + `tests/`** — 配套的 Python/参考工作区和审计辅助工具；不是主要的运行时表面

## 快速入门

> [!NOTE]
> [!WARNING]
> **`cargo install claw-code` 会安装错误的东西。** crates.io 上的 `claw-code` crate 是一个已废弃的存根（stub），它会放置 `claw-code-deprecated.exe` — 而不是 `claw`。运行它只会打印 `"claw-code has been renamed to agent-code"`。**请勿使用 `cargo install claw-code`。** 请从源码构建（本代码库）或安装上游二进制文件：
> ```bash
> cargo install agent-code   # 上游二进制文件 — 安装为 'agent.exe' (Windows) / 'agent' (Unix)，而不是 'agent-code'
> ```
> 本代码库 (`ultraworkers/claw-code`) **仅支持从源码构建** — 请遵循以下步骤。

```bash
# 1. 克隆并构建
git clone https://github.com/ultraworkers/claw-code
cd claw-code/rust
cargo build --workspace

# 2. 设置您的 API 密钥（Anthropic API 密钥 — 不是 Claude 订阅）
export ANTHROPIC_API_KEY="sk-ant-..."

# 3. 验证所有配置是否正确
./target/debug/claw doctor

# 4. 运行提示词
./target/debug/claw prompt "say hello"

# 5. 启动交互式会话
./target/debug/claw
```

> [!NOTE]
> **Windows (PowerShell)：** 二进制文件是 `claw.exe`，而不是 `claw`。使用 `.\target\debug\claw.exe` 或运行 `cargo run -- prompt "say hello"` 以跳过路径查找。

### Windows 设置

**PowerShell 是一种支持的 Windows 路径。** 您可以使用任何适合您的终端 shell。Windows 上常见的入门问题是：

1. **首先安装 Rust** — 从 <https://rustup.rs/> 下载并运行安装程序。完成后关闭并重新打开终端。
2. **验证 Rust 是否在 PATH 中：**
   ```powershell
   cargo --version
   ```
   如果失败，请重新打开终端或运行 Rust 安装程序输出的 PATH 设置步骤，然后重试。
3. **克隆并构建**（在 PowerShell、Git Bash 或 WSL 中均可）：
   ```powershell
   git clone https://github.com/ultraworkers/claw-code
   cd claw-code/rust
   cargo build --workspace
   ```
4. **运行**（PowerShell — 注意 `.exe` 和反斜杠）：
   ```powershell
   $env:ANTHROPIC_API_KEY = "sk-ant-..."
   .\target\debug\claw.exe prompt "say hello"
   ```

有关发布版的 ZIP、PATH 设置、提供商切换以及通知的冒烟测试，请参阅 [`docs/windows-install-release.md`](./docs/windows-install-release.md)。

**Git Bash / WSL** 是可选的替代方案，而非必须。如果您更喜欢 bash 风格的路径（例如 `/c/Users/you/...` 而不是 `C:\Users\you\...`），Git Bash（随 Git for Windows 一起提供）效果很好。在 Git Bash 中，`MINGW64` 提示符是预期的正常现象 — 并不代表安装已损坏。

## 构建后操作：定位二进制文件并验证

在 `claw-code/rust/` 中运行 `cargo build --workspace` 后，`claw` 二进制文件已构建，但**不会**自动安装到您的系统中。以下是定位文件和验证构建是否成功的方法。

### 二进制文件位置

`cargo build --workspace` 后，在 `claw-code/rust/` 目录中：

**Debug 构建（默认，编译较快）：**
- **macOS/Linux：** `rust/target/debug/claw`
- **Windows：** `rust/target/debug/claw.exe`

**Release 构建（已优化，编译较慢）：**
- **macOS/Linux：** `rust/target/release/claw`
- **Windows：** `rust/target/release/claw.exe`

如果您运行 `cargo build` 时没有添加 `--release`，则二进制文件位于 `debug/` 文件夹中。

### 验证构建是否成功

直接使用其路径测试二进制文件：

```bash
# macOS/Linux (debug 构建)
./rust/target/debug/claw --help
./rust/target/debug/claw doctor

# Windows PowerShell (debug 构建)
.\rust\target\debug\claw.exe --help
.\rust\target\debug\claw.exe doctor
```

无需实时凭据的 PowerShell 冒烟测试命令：

```powershell
$env:CLAW_CONFIG_HOME = Join-Path $env:TEMP "claw config home"
New-Item -ItemType Directory -Force -Path $env:CLAW_CONFIG_HOME | Out-Null
Remove-Item Env:\ANTHROPIC_API_KEY, Env:\ANTHROPIC_AUTH_TOKEN, Env:\OPENAI_API_KEY -ErrorAction SilentlyContinue
.\rust\target\debug\claw.exe help
.\rust\target\debug\claw.exe status
.\rust\target\debug\claw.exe config env
.\rust\target\debug\claw.exe doctor
```

如果这些命令成功执行，说明构建正常工作。`claw doctor` 是您的首个健康检查工具 — 它会验证您的 API 密钥、模型访问权限和工具配置。

### 可选：添加到 PATH

如果您希望在任何目录下都不用输入完整路径即可运行 `claw`，请选择以下方法之一：

**选项 1：符号链接 (macOS/Linux)**
```bash
ln -s $(pwd)/rust/target/debug/claw /usr/local/bin/claw
```
然后重新加载 shell 并测试：
```bash
claw --help
```

**选项 2：使用 `cargo install` (所有平台)**

构建并安装到 Cargo 的默认位置（`~/.cargo/bin/`，通常已在 PATH 中）：
```bash
# 在 claw-code/rust/ 目录下
cargo install --path . --force

# 然后在任何地方都可以运行
claw --help
```

**选项 3：更新 shell 配置文件 (bash/zsh)**

将此行添加到 `~/.bashrc` 或 `~/.zshrc`：
```bash
export PATH="$(pwd)/rust/target/debug:$PATH"
```

重新加载 shell：
```bash
source ~/.bashrc  # 或 source ~/.zshrc
claw --help
```

### 故障排除

- **"command not found: claw"** — 二进制文件在 `rust/target/debug/claw`，但未在您的 PATH 中。请使用完整路径 `./rust/target/debug/claw` 或按照上述方法进行软链接/安装。
- **"permission denied"** — 在 macOS/Linux 上，如果缺少可执行权限（罕见情况），您可能需要运行 `chmod +x rust/target/debug/claw`。
- **Debug vs. release** — 如果构建运行缓慢，说明您处于 debug 模式（默认）。在 `cargo build` 中添加 `--release` 以获得更快的运行时速度，但这会使构建本身花费 5-10 分钟。

> [!NOTE]
> **认证：** claw 需要一个 **API 密钥** (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY` 等) — 暂不支持 Claude 订阅登录作为认证途径。

在验证二进制文件可运行后，执行工作区测试套件：

```bash
cd rust
cargo test --workspace
```

## 文档地图

- [`USAGE.md`](./USAGE.md) — 快捷命令、认证、会话、配置、一致性工具
- [`docs/navigation-file-context.md`](./docs/navigation-file-context.md) — 终端导航、历史滚动记录、`@path` 文件上下文、附件以及安全秘钥指南
- [`docs/local-openai-compatible-providers.md`](./docs/local-openai-compatible-providers.md) — Ollama/llama.cpp/vLLM 设置、Claw 多提供商定位和本地技能安装检查
- [`docs/windows-install-release.md`](./docs/windows-install-release.md) — PowerShell 为主的安装、发布工件、提供商切换和 Windows/WSL 通知冒烟测试路径
- [`rust/README.md`](./rust/README.md) — crate 地图、CLI 表面、功能特性、工作区布局
- [`PARITY.md`](./PARITY.md) — Rust 移植的一致性状态
- [`rust/MOCK_PARITY_HARNESS.md`](./rust/MOCK_PARITY_HARNESS.md) — 确定性模拟服务工具（mock-service harness）详细信息
- [`ROADMAP.md`](./ROADMAP.md) — 活跃的路线图和待清理的后台任务
- [`docs/g004-events-reports-contract.md`](./docs/g004-events-reports-contract.md) — 面向消费者的 Stream 2 lane 事件/报告契约指南
- [`PHILOSOPHY.md`](./PHILOSOPHY.md) — 该项目存在的原因以及它的运作方式
- [`CONTRIBUTING.md`](./CONTRIBUTING.md), [`SECURITY.md`](./SECURITY.md), [`SUPPORT.md`](./SUPPORT.md) 和 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) — 贡献、漏洞报告、支持和社区政策
- [`LICENSE`](./LICENSE) — 此代码库的 MIT 许可证

## 生态系统

Claw Code 是在开源环境下与更广泛的 UltraWorkers 工具链一起构建的：

- [clawhip](https://github.com/Yeachan-Heo/clawhip)
- [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent)
- [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode)
- [oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex)
- [gajae-code](https://github.com/Yeachan-Heo/gajae-code)
- [UltraWorkers Discord](https://discord.gg/5TUQKqFWd)

## 所有权 / 附属声明

- 本代码库**不**声明对原始 Claude Code 源代码拥有所有权。
- 本代码库**不隶属于 Anthropic，未获得 Anthropic 的认可，也不由 Anthropic 维护**。

---

> 中文翻译由 [JasonYeYuhe](https://github.com/JasonYeYuhe) 维护