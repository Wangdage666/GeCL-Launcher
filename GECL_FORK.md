# gecl（Modrinth App 衍生版）

2026-11-02：统一正式版默认包含本机命令接口、CLI 与命令说明，不再单独交付调试版；沿用原软件名、版本标识及数据路径。CurseForge 的 MC 百科按钮等待项目匹配、补查规范化名称与别名，并核验关联链接；匹配成功直接打开条目页，未匹配或网络失败短提示并允许重试，不自动跳搜索页。旧生成文件保留回退，验证边界见 docs/DEVELOPMENT_PLAN.md。

2026-10-31：CurseForge 介绍保留原官方／镜像接口，失败或为空时从 [CFWidget](https://cfwidget.com/) 的公开项目数据补取作者 HTML，并核对项目 ID 与 Minecraft 类型。正文使用现有过滤和排版，失败可重试，不影响版本浏览与安装。元数据缓存可能晚于官网，具体交付与验证边界见 docs/DEVELOPMENT_PLAN.md。

2026-10-30：本地自制/非托管模组显示文件内声明的作者与图标；没有声明时不猜测身份。导出整合包可选图标并随包携带，文件导入可预览和替换。世界页增加指定位置 ZIP 备份与覆盖恢复，恢复前保留旧世界副本。另提供同名、同界面的 1.0.0-agent 构建，只在该构建启用本机命令接口；固定 CLI/JSON 调用与完整清单随包提供，不绑定模型厂商。验证与限制见 docs/DEVELOPMENT_PLAN.md。

2026-10-29 外观：新增八种主题色与自定义颜色、本地图片背景、模糊/遮罩/面板不透明度/对齐、实时预览和恢复默认。明暗主题自动适配操作文字对比度，背景不模糊正文，列表和弹窗保留清晰底色；尊重减少动画、减少透明度和增强对比度偏好。图片按比例缩放后保存应用副本，不修改原图。默认仍为浅色蓝色、无背景；详情与验证范围见 docs/DEVELOPMENT_PLAN.md。

2026-10-29：整合包导出按当前文件的哈希和大小识别下载来源；完全匹配的 MR/已知 CF 文件写入下载清单，被修改、自制或无法确认来源的文件直接嵌入。保留改名、禁用状态和所选配置，导出失败不覆盖旧包。仍使用标准 .mrpack，具体测试与边界见 docs/DEVELOPMENT_PLAN.md。

2026-10-25 补充：MR 普通内容下载接入右栏真实字节进度；CF 切换版本复用原弹窗，不再跳转详情页，按当前游戏兼容条件选择并保留依赖和校验。启动进度面板快速淡入淡出，仅透明度动画，遵循减少动画设置。验证状态见 docs/DEVELOPMENT_PLAN.md。

2026-10-25：模组双源结果合并为一个列表和分页，来源放在卡片标签区；两站使用同一个安装按钮和选择游戏弹窗，按目标 MC/加载器自动匹配并安装必需前置，保留来源各自的下载量和身份。CF 版本优先读取包内声明，与文件名分开；世界右键目录改用系统路径拼接。验证范围见 docs/DEVELOPMENT_PLAN.md。

2026-10-24：模组支持 Modrinth / CurseForge 双源浏览、独立来源与下载量、版本与必需依赖安装；CF 项目和文件元数据与 MR 缓存隔离，已安装列表保留作者、图标和版本。CF 公开信息默认来自 [MCIM](https://www.mcimirror.top/)，可能晚于官网；其未提供完整正文时使用摘要和官网入口，不嵌入来源脚本。下载保留 SHA-1/长度验证，并对缺少 API URL 的公开文件使用已验证的 ForgeCDN 回退。只参考 PCL 公开源码的处理机制，没有复制其代码、数据、素材或凭据。当前 CF 接入范围是模组，不代表支持 CF 整合包等所有类别。具体证据与未验证边界见 docs/DEVELOPMENT_PLAN.md。

2026-10-24 本地内容：在游戏版本的内容区域拖入模组、资源包、光影包和数据包文件，按包内结构自动分类复制并刷新列表；保留原文件。支持批量去重、状态和取消后续文件，未知文件与整合包会提示。目录外/忙碌/受限页面不接收拖放；不等同于自动验证模组版本兼容、补齐依赖或整批原子安装。

2026-10-23 下载修正：整合包封面使用可选的 4 秒/4 MiB 下载，不再重复获取现有本地图标，也不因图片错误回滚游戏；连接失败重试会刷新内置客户端的系统代理配置。模组并发遵循原设置（1–10），增加文件流缓冲和 HTTP/2 自适应窗口，仍保留哈希、取消与事务校验。实际线路受 CDN 和代理影响，未宣称跑满宽带。现有导出 .mrpack 可用下载清单替代已识别文件，未知/自制文件与配置随 overrides 携带；嵌入文件量决定包体积。

独立桌面项目，源自 [modrinth/code](https://github.com/modrinth/code) 提交 `b89f9f6c147b2c7d484d89e49190c67738b7f242`。保留上游 LICENSE、COPYING 与作者归属；应用、应用库和共享 UI 遵循各自的 GPL-3.0-only 许可证。gecl 与 Modrinth 官方没有隶属或背书关系。

## 使用与构建

2026-10-23：启动进度恢复与下载区域相同的 0.2 秒宽度过渡，面板加宽至 420 逻辑像素；阶段、计时、窗口判断和最小化时点不变。支持原生 versionType 的菜单使用 started by GeCL，通常显示为 Minecraft 版本号/started by GeCL；不修改游戏 JAR，不给缺少该参数的旧版强加参数。Forge/自定义主菜单可能覆盖该显示，不能保证所有整合包都显示；JVM launcher.brand 同步为 GeCL。

本轮补充：新目录保存与旧目录读取的 Windows 路径兼容、跨 Java 8/16/17/21/25 的日志配置文件 URI，以及蓝白启动进度条。进度按真实阶段推进并显示等待时长，窗口出现才到 100%；最小化延后到该时点。Java 选择和游戏主类/RPC 主体仍沿用上游；测试范围及旧 JRE 中文命令行限制见 docs/JAVA_LAUNCH_AUDIT.md。运行环境探针不等于真实整合包启动成功。

`build-gecl/gecl.exe` 可直接运行；同目录的 `*-setup.exe` 是 Windows 全机安装器，需要管理员授权，默认位置为 `C:\Program Files (x86)\GeCL`。首次使用须有 Microsoft Edge WebView2。文件未签名，不将成功构建等同于已在其他电脑通过安装验收。

```powershell
pnpm install --filter @modrinth/app... --frozen-lockfile
./scripts/gecl.ps1 Build
./scripts/gecl.ps1 Test
node scripts/gecl-check-translations.mjs
```

`Build` 生成经过优化的 release 可执行文件和中文 NSIS 安装器，输出至 `build-gecl/`。不携带调试符号、源码映射及旧可执行文件；旧程序备份在 `.build-backups/`。`Check` 检查桌面 Rust，`Test` 运行核心库测试，`Run` 为开发模式。原始产物在 `target/release/`。

工具版本以锁文件、package.json、rust-toolchain.toml 为准。本机通过根目录 `.tools/cargo`、`.tools/rustup` 使用 Rust，由脚本通过 vswhere 和 Windows SDK 查找 C++ x64 工具；不修改系统 PATH。换机需要 Node、pnpm、Java、Rust、C++ Build Tools 和 Windows SDK。Java 工程使用上游 Gradle wrapper。

## 界面与数据

- 固定简体中文。应用 1542 条、共享 UI 2469 条消息经过补齐、人工校订和 ICU 契约检查；未调用机器翻译服务。桌面语言目录只保留 zh-CN，移除语言设置。
- 使用“创建游戏版本”等直观名称。当前列表为空时，首页显示创建、导入 mrpack、导入已有游戏三个入口；原有其他启动器导入保留在下方，已有游戏列表的工具栏也能打开文件夹导入。读取失败单独显示重试。
- 首页右侧为游玩账户、最多三个最近游玩版本和进行中/需要处理的下载任务。无相关记录时不显示空下载模块。详情/搜索保留原有上下文侧栏。保留皮肤页面。
- 移除平台账户登录、新手任务、好友、云端共享、付费托管、广告、活动、新闻和 Plus 推广。公开资源搜索、详情和下载仍使用真实来源。不上报到 Modrinth 官方 Sentry，不使用其自动更新渠道。
- 新绘制蓝色切角方块图标，以白色 G 与浅蓝启动箭头组合；矢量源在 `apps/app/icons/gecl.svg`，同步导出 Windows ICO、各尺寸 PNG、ICNS 和应用内图标。 NSIS 安装器另通过 `apps/app/tauri.conf.json` 的 `installerIcon` 指向该 ICO。未复制官方品牌素材。设置中的平台导入、缓存和旧同步残留文案按本地功能重写。
- GUI 程序与 Java 子进程隐藏控制台，诊断日志仍保留。直接运行 gecl.exe 不经过命令脚本。

独立标识 `top.wangdage.gecl.modrinth`。全新使用先通过弹窗选择游戏数据目录，默认是当前 gecl.exe 所在文件夹本身，不再附加子文件夹；点击“下一步”添加离线或微软账户，点击“完成设置”直接进入首页。已有游戏从首页“导入已有游戏”进入，不再额外弹出导入步骤。未选目录前不初始化游戏内容仓库。游戏、Java、资源和下载缓存放所选目录；小型设置数据库、DPAPI 凭据和 WebView 配置仍位于 Windows 用户配置目录，通常为 `%APPDATA%/top.wangdage.gecl.modrinth`。已有数据库沿用原目录；首次设置中途退出或删除全部账户后，下次恢复添加账户弹窗，不创建 Player。

安装界面使用自有白蓝配色和图标，路径栏为单行高度。选择安装文件夹后补齐一层 `GeCL` 并显示完整路径；已以 GeCL 结尾则不重复添加。为允许普通权限的程序在安装位置存放游戏，给 profiles、meta、caches、store、icons、synced-options 六个数据子目录及预先创建的 store.lock 单个文件授予本机用户修改权限，不放宽程序文件所在根目录。首次选择目录同时验证锁文件可用，避免保存设置后才在初始化阶段失败。受保护根目录/可写数据子目录/可写锁文件的隔离原生流程已测试；真实管理员安装和旧版跨安装模式升级仍须单独验收。Tauri 安装模板来源及 MIT 许可证保留在 `apps/app/nsis/`。

卸载默认删除空的数据目录和锁文件；非空游戏目录保留。勾选“同时删除设置和安装目录内的游戏数据（含存档）”才删除安装目录下六个受管目录，外部数据位置及未知文件不会自动删除。递归清理不跟随目录链接/重解析点；更新安装不执行数据清理。清理函数与安装授权宏已使用隔离目录实际测试，不把它当作用户机器实装卸载验收。

可用 `THESEUS_CONFIG_DIR` 指定隔离配置目录。开发调试构建通过 `scripts/tauri.debug.conf.json` 使用 `top.wangdage.gecl.localtest`，不与正式版抢占单实例。不自动迁移原启动器或 WPF 工程的数据。不得把真实账户库、Token、WebView 用户目录或游戏存档打进源码包。

## 导入已有游戏

选择单个版本文件夹，或包含 `versions` 的 `.minecraft` 目录。根据真实版本 JSON 和加载器依赖识别 Minecraft、Fabric / Forge / NeoForge / Quilt 的版本，预览模组、光影包、资源包和存档；不能只凭文件夹名字推断版本，只有 mods 而没有版本描述的目录会提示无法识别。

已直接放在所选数据目录 `profiles` 下的版本会在后台发现并原地登记；外部文件夹先复制到临时目录，完成后登记，不移动或删除源文件。保留禁用模组和存档，跳过已知启动器账户文件、传统公共资源目录与 natives。导入前退出游戏，避免复制中存档变化。取消时清理本次临时副本；强制结束进程后的临时目录不会自动恢复导入，原文件仍保留。

导入版本初始标为尚未安装。“补全运行环境”先按可信版本清单的 SHA-1 和大小复用现有游戏本体、普通依赖库及资源，再下载缺少或校验不符的文件。外部导入保存原目录出处以只读查找公共 libraries/assets；原目录移动后仍可正常补下载，旧导入没有出处时只能复用导入目录中实际保留的文件。不会只凭文件名跳过校验，也不把识别成功当作已可启动。版本描述不支持的定制加载器、链接目录和损坏描述会拒绝导入。导入不继承其他启动器的 Java 参数和账户；首次登记禁用共享配置同步，避免覆盖已有设置。

资源索引会随官方内容更新，同一个索引 ID 不代表文件哈希不变。现在先取得并校验当前索引，再按当前索引逐个复用原目录 assets/objects 中哈希和大小一致的对象；旧索引不同不会阻止整批对象复用。下载总量在复用后计算；当前索引新增、变化或损坏的对象仍正常补下载。手动强制重装保留原有强制下载含义。只在导入实例执行本地来源复用，普通新建流程保持原行为。

Forge 安装处理程序的 Java 路径使用现有 dunce 转换为等价的兼容形式，包含尚未生成的库输出及 Minecraft JAR、游戏根、库根变量，避免普通 Windows 路径带 `\\?\` 前缀时旧 Java 文件接口无法创建目录。网络共享、保留名称、尾点和真正需要扩展格式的长路径不强制截去前缀，不改变 Rust 文件读写、目录权限或文件验证。已使用 Java 17 与 Forge installertools 1.4.1 在隔离目录复现错误，并验证兼容路径能完成 Minecraft 1.20.1 映射文件下载；不代表完整整合包或游戏启动已验收。

游戏启动的 Log4j 配置文件参数也使用相同的路径转换，保留为一个完整 JVM 参数，避免带扩展前缀的普通路径触发 Source.normalize 的 Bad pathname。已用 Java 17、实际缓存的 Log4j 2.19 和 client-1.12.xml 对照复现并验证日志初始化和写入；此项不是完整模组包进入主菜单的验收。

## 账户

微软公共应用 ID：`f4868fd9-77f6-454d-8e33-066c436d3f2c`。设备代码 → Xbox/XSTS → Minecraft Token → 拥有权与玩家资料校验；成功后才保存微软身份。应用仍需允许公共客户端流并取得 Minecraft 所需平台许可。未代替用户进行真实登录验收。

Token 通过 Windows 当前用户 DPAPI 保护，仅后端使用；前端只接收公开资料、类型和选择状态。离线账户是独立 SQLite 表，名字为 3–16 位英文、数字或下划线，UUID 遵循 Java `OfflinePlayer:<名字>`。不论新旧数据库，都不自动创建 Player；已有的 Player 账户也不擅自删除。删除最后一个账户后，下次启动要求重新添加；游戏目录及列表保留。离线身份不能用于需要正版验证的服务器或更改微软账户皮肤。离线换皮肤暂不实现。

## MC 百科与中文名称

`features/mcmod` 查询公开搜索页，再验证候选条目“相关链接”中的 Modrinth ID/slug，匹配后直达 `class/<编号>.html`。中文名显示为“中文名（原名）”，不改项目标识和下载信息。七天正缓存、一小时未匹配缓存、请求限速；网络错误保留原名，未匹配时提供明确搜索入口。不复制百科介绍正文。

动态作者介绍、外部网页、加载器名、路径和服务端技术错误可能保留原文，不能将其称为全部本地固定文案。

## 国内镜像

设置 → 下载：仅官方、优先官方慢速/失败切换、优先镜像失败回退、仅镜像（覆盖的文件）。默认优先官方，可设置慢速阈值和观察时间；仅影响新请求。

镜像由 [BMCLAPI](https://bmclapidoc.bangbang93.com/) 提供。严格按公开 Minecraft/资源/加载器主机及路径映射；认证请求、带认证头的请求、Modrinth API/CDN 和未知地址不转发。错误、慢速或哈希校验失败时从备用源重新下载，不拼接两源文件；保留已有哈希检查和临时文件清理。暂停时间不计入慢速窗口。

未接入 MCIM：其现行文档要求启动器登记与独立 User-Agent，本轮没有代用户向第三方登记。

## 维护入口与验证

- `apps/app-frontend/src/components/ui/WelcomeScreen.vue`、`pages/Index.vue`：当前空列表与三入口。
- `features/gecl/startup-gate.vue`、`first-run-setup.vue`、`folder-import.vue`：首次目录选择、账户向导和文件夹导入预览；`apps/app/src/api/gecl.rs` 提供桌面桥，`packages/app-lib/src/state/gecl/` 负责实际读取、复制、登记和发现。
- `app/shell/sidebar`：账户、最近游玩、下载任务。
- `features/settings/system/download-source-settings.vue`：下载设置；`packages/app-lib/src/util/download_sources.rs`：策略与映射；`util/fetch.rs`：请求切换与校验。
- `state/minecraft_auth/gecl.rs`、`offline_accounts.rs`、`token_protection.rs`：账户边界。
- `scripts/gecl-message-contract.json`：只有消息键和参数契约，不是英文翻译。

最新完成情况、测试证据及未验收边界统一见开发者文档 `docs/DEVELOPMENT_PLAN.md`。

### 中文搜索原理与参考

调查参考：[龙腾猫跃 PCL ResourceSearcher](https://github.com/Meloong-Git/PCL/blob/main/Plain%20Craft%20Launcher%202/Modules/Resource/ResourceSearcher.vb)、[WikiEntry](https://github.com/Meloong-Git/PCL/blob/main/PCLCS/Resource/WikiEntry.cs)。PCL 使用内置中文名/别名和平台 slug 数据库完成匹配、平台查询与筛选。此处只作机制调查，不引入其代码和 WikiEntries 数据。

gecl 通过公开百科的中文检索和条目相关链接，按需建立中文名到 Modrinth 标识的映射，再用原版本、加载器、排序及分页条件查询 v3 API。适配 v3 结果的 name 字段；详情仍使用 v2 title。映射查询最多核对前八个候选，成功缓存一天；宽泛词的覆盖范围有限，百科故障或未收录时可使用英文。真实“苹果皮 → AppleSkin”的 1.20.1/Fabric 查询及不兼容加载器排除已验证，最终原生窗口结果见开发计划。
