# GeCL 桌面外壳

基于 [Tauri](https://tauri.app/) 的 Windows 桌面程序，负责：

- 创建窗口并加载 `apps/app-frontend` 构建出的界面；
- 通过 `src/api` 向界面提供 IPC 命令，实际业务由 `packages/app-lib` 实现；
- 生成 NSIS 安装包（`nsis/`）和本机命令行工具 `gecl-agent.exe`（`src/agent_cli.rs`）。

构建请在仓库根目录运行 `./scripts/gecl.ps1 -Action Build`，详见根目录 README。

本目录源自 Modrinth App，按 GPL-3.0-only 发布，见 `LICENSE` 与 `COPYING.md`。
