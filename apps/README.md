# apps

GeCL 桌面程序由两个部分组成：

- `app`：Tauri 桌面外壳（Rust），负责窗口、IPC 命令、安装包和本机命令行工具。
- `app-frontend`：桌面界面（Vue 3 + Vite），构建后嵌入 `app` 一起发布。
