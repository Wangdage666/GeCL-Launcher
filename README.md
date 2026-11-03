# GeCL

[简体中文](#简体中文) · [English](#english)

## 简体中文

GeCL 是一款 Windows 上的 Minecraft Java 版启动器，基于 [Modrinth App](https://github.com/modrinth/code) 独立衍生，使用 Vue 3、Tauri 和 Rust。界面为简体中文，白底蓝色、操作紧凑。GeCL 与 Modrinth 官方没有隶属或背书关系。

### 主要功能

- **账户**：微软正版登录（设备代码）和离线账户分开管理；凭据经 Windows DPAPI 保护，只在后端使用。
- **游戏版本**：创建 Vanilla / Fabric / Forge / NeoForge / Quilt 版本，导入 `.mrpack` 整合包或已有的 `.minecraft` / 版本文件夹，复用本地文件前按 SHA-1 与大小校验。
- **内容**：Modrinth 与 CurseForge 双源模组搜索、版本选择与必需前置安装；本地文件拖入安装；显示本地模组声明的作者和图标。
- **中文名称**：通过 MC 百科显示模组中文名，支持中文搜索并直达已核验的百科条目。
- **下载**：可选 BMCLAPI 国内镜像（仅覆盖公开文件，失败或校验不符时回退）。
- **整合包与存档**：导出 `.mrpack`（可识别来源的文件写入下载清单，其余嵌入）；世界 ZIP 备份与恢复。
- **外观**：主题色、自定义颜色与本地图片背景。
- **本机接口**：正式版附带 `gecl-agent.exe` 命令行工具，可在本机调用应用功能，说明见 [docs/AGENT_COMMANDS.md](docs/AGENT_COMMANDS.md)。

### 安装与使用

在 Releases 下载 `gecl_*_x64-setup.exe` 安装（需要管理员权限，默认安装到 `C:\Program Files (x86)\GeCL`），或直接运行 `gecl.exe`。需要 Microsoft Edge WebView2。首次启动选择游戏数据目录并添加账户。程序目前未签名。

启动时会检查本仓库的最新 Release。有新版本时询问是否更新：确认后在应用内下载安装包并校验 SHA-256，然后以被动模式安装并自动重启（Windows 会请求一次管理员授权）。手动运行新版安装包时，若已安装旧版，安装页显示“更新”并保留设置与游戏数据。发布新版本时请同时上传安装包和 `build-gecl/SHA256SUMS.txt`：GitHub API 限流时，更新检查改用该文件中的校验值。

### 从源码构建

需要 Node.js、pnpm、Rust（见 `rust-toolchain.toml`）、Visual Studio C++ Build Tools 与 Windows SDK。在仓库根目录运行：

```powershell
pnpm install --filter '@modrinth/app...' --frozen-lockfile
./scripts/gecl.ps1 -Action Test
node scripts/gecl-check-translations.mjs
./scripts/gecl.ps1 -Action Build
```

`Build` 输出 release 程序、NSIS 安装包和命令行工具到 `build-gecl/`；`Check` 检查桌面 Rust，`Test` 运行核心库测试。

### 目录

- `apps/app-frontend`：桌面界面（Vue 3）。
- `apps/app`：Tauri 桌面壳、安装器、图标和 IPC。
- `packages/app-lib`：账户、导入、下载、校验和游戏管理（Rust）。
- `packages/ui`、`packages/assets`：共享组件与界面资源。
- `docs`：本机命令行工具说明。
- `scripts`：构建、翻译检查与命令清单生成脚本。

衍生来源与功能变更说明见 [GECL_FORK.md](GECL_FORK.md)。

### 许可证

源自 [modrinth/code](https://github.com/modrinth/code)，保留上游 LICENSE、COPYING 与作者归属。桌面应用、应用库和共享 UI 遵循各自的 GPL-3.0-only 许可证，详见 [COPYING.md](COPYING.md) 及各组件的 LICENSE。

## English

GeCL is a Minecraft: Java Edition launcher for Windows, an independent derivative of the [Modrinth App](https://github.com/modrinth/code) built with Vue 3, Tauri and Rust. The interface is in Simplified Chinese with a compact white-and-blue layout. GeCL is not affiliated with or endorsed by Modrinth.

### Features

- **Accounts**: Microsoft sign-in (device code) and offline accounts are kept separate; credentials are protected with Windows DPAPI and used only by the backend.
- **Game versions**: create Vanilla / Fabric / Forge / NeoForge / Quilt versions, import `.mrpack` modpacks or existing `.minecraft` / version folders; local files are reused only after SHA-1 and size checks.
- **Content**: combined Modrinth and CurseForge mod search, version selection and required-dependency installation; drag-and-drop local files; shows authors and icons declared inside local mods.
- **Chinese names**: shows Chinese mod names from MC百科 (mcmod.cn), supports Chinese search and opens verified encyclopedia entries directly.
- **Downloads**: optional BMCLAPI mirror for public files, with fallback on errors or hash mismatches.
- **Modpacks and worlds**: `.mrpack` export (files with a verified source go into the download manifest, others are embedded); ZIP world backup and restore.
- **Appearance**: theme colors, custom colors and a local image background.
- **Local interface**: the release ships `gecl-agent.exe`, a CLI for calling app features on the same machine; see [docs/AGENT_COMMANDS.md](docs/AGENT_COMMANDS.md).

### Install and use

Download `gecl_*_x64-setup.exe` from Releases (requires administrator rights; installs to `C:\Program Files (x86)\GeCL` by default), or run `gecl.exe` directly. Microsoft Edge WebView2 is required. On first launch, choose a game data folder and add an account. Binaries are currently unsigned.

On startup GeCL checks this repository's latest Release. When a newer version exists it asks whether to update; if confirmed, the installer is downloaded inside the app, its SHA-256 is verified, and it runs in passive mode and restarts GeCL (Windows asks for administrator approval once). Running a newer installer manually over an older install shows "更新" (Update) and keeps settings and game data. When publishing a release, upload the installer together with `build-gecl/SHA256SUMS.txt`; if the GitHub API is rate limited, the update check uses the checksum from that file.

### Building from source

Requires Node.js, pnpm, Rust (see `rust-toolchain.toml`), Visual Studio C++ Build Tools and the Windows SDK. From the repository root:

```powershell
pnpm install --filter '@modrinth/app...' --frozen-lockfile
./scripts/gecl.ps1 -Action Test
node scripts/gecl-check-translations.mjs
./scripts/gecl.ps1 -Action Build
```

`Build` writes the release executable, the NSIS installer and the CLI to `build-gecl/`; `Check` checks the desktop Rust crate and `Test` runs the core library tests.

### Layout

- `apps/app-frontend`: desktop UI (Vue 3).
- `apps/app`: Tauri shell, installer, icons and IPC.
- `packages/app-lib`: accounts, import, downloads, verification and game management (Rust).
- `packages/ui`, `packages/assets`: shared components and UI assets.
- `docs`: reference for the local CLI (in Chinese).
- `scripts`: build, translation check and command catalog scripts.

See [GECL_FORK.md](GECL_FORK.md) (in Chinese) for the derivation and the list of changes.

### License

Derived from [modrinth/code](https://github.com/modrinth/code); upstream LICENSE, COPYING and attribution are kept. The desktop app, app library and shared UI are licensed under GPL-3.0-only as stated in [COPYING.md](COPYING.md) and each component's LICENSE.
