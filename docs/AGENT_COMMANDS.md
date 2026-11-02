# GeCL Agent 命令详细说明

文档修订：2026-11-02。共 **296 条实际注册的应用接口**，另有 CLI 入口和页面操作命令。每条接口都在下方写明中文用途、每个参数的意义、返回值、数据影响、可用条件与调用模板。

接口清单按当前源码的 296 条实际应用接口整理，已纠正旧版误列的模块名称及缓存查询缺失参数。统一正式版默认开放本机命令接口并同步嵌入修正后的清单；CurseForge 介绍读取在原接口失败或为空时增加 CFWidget 补取，命令名称与参数不变。

## 先理解命令怎么写

先启动 GeCL 正式版，再在安装目录中 `gecl-agent.exe` 所在文件夹打开 PowerShell。2026-11-02 起正式版默认开放本机命令接口，版本号保留 `1.0.0-local`，不再另行打包 `1.0.0-agent`。软件名、正常界面、账户与游戏目录沿用现有设置；同一份数据仍只运行一个启动器。

调用格式是：

```text
gecl-agent.exe invoke "接口名" "JSON参数或参数文件"
```

例如 `plugin:auth|add_offline_user` 的拆法是：`plugin:auth` 表示账户模块，`add_offline_user` 表示添加离线账户。整段必须作为一个参数引用，不能省掉中间的 `|`。这些是本地原生接口名称，**不是 `gecl://` 深链接**。

对于复杂参数，推荐把 JSON 保存为 UTF-8 的 `args.json`，再执行：

```powershell
.\gecl-agent.exe invoke 'plugin:auth|add_offline_user' '@args.json'
```

`args.json` 中的内容：

```json
{"username":"TestUser"}
```

也可以在 PowerShell 中生成这个文件：

```powershell
@{ username = 'TestUser' } | ConvertTo-Json -Depth 50 |
    Set-Content -LiteralPath .\args.json -Encoding utf8
.\gecl-agent.exe invoke 'plugin:auth|add_offline_user' '@args.json'
```

文档中的 `@args.json` 带引号，可以避免 PowerShell 把它解释为自己的语法。路径有空格时写成 `'@D:/我的文件/args.json'`。JSON 内路径使用 `D:/Games` 最省事；使用反斜杠时要写成 `D:\\Games`。Windows PowerShell 5.1 向外部程序传复杂 JSON 可能丢失双引号，优先使用文件；无参数的 `'{}'` 可以直接传。

## CLI 入口：每一条做什么

| 固定命令 | 参数 | 作用 | 返回／限制 |
|---|---|---|---|
| `gecl-agent.exe --help` | 无 | 显示命令入口用法 | 普通文本；不要求启动器已运行 |
| `gecl-agent.exe status` | 无 | 检查正式版是否就绪，并查询当前页面 | `result.ready` 与 `result.route`；不是游戏运行状态 |
| `gecl-agent.exe commands` | 无 | 读取当前程序内嵌的接口清单 | 接口数组；随本次正式版同步嵌入，与交付旁边的 JSON 清单对应 |
| `gecl-agent.exe invoke '接口名' '{}'` | 接口名、JSON | 调用一个无需参数的应用接口 | 返回统一 JSON；省略 JSON 时也按空对象处理 |
| `gecl-agent.exe invoke '接口名' '@args.json'` | 接口名、UTF-8 参数文件 | 读取文件中的 JSON 后调用接口 | 适合路径、嵌套配置、长参数；文件内容应是一个 JSON 对象 |
| `gecl-agent.exe invoke '接口名' -` | 接口名、标准输入 | 从标准输入读取全部 JSON | 适合其他程序或 AI 工具通过管道调用 |
| `gecl-agent.exe ui.navigate '/页面路径'` | 已存在的 Vue 路由 | 切换启动器当前页面 | 返回切换后的路由；跳转完成不代表页面网络请求已完成 |
| `gecl-agent.exe ui.snapshot` | 无 | 读取当前页文字和按钮、输入框等控件摘要 | 返回 `route`、`text`、`controls`；不读取输入框当前值，也不是截图 |
| `gecl-agent.exe ui.click 'CSS选择器'` | CSS 选择器 | 点击第一个匹配控件 | 找不到控件报错；true 只表示已发起点击，不保证后台操作完成 |
| `gecl-agent.exe ui.fill 'CSS选择器' '内容'` | CSS 选择器、文字 | 设置 input/textarea 的值，并触发 input/change | true 表示已填入；不会自动点保存，也不用于 select 或复选框 |

页面操作例子：

```powershell
.\gecl-agent.exe status
.\gecl-agent.exe ui.navigate '/'
.\gecl-agent.exe ui.snapshot
# 先从实际页面确认选择器；不要假定所有页面都有这个输入框
.\gecl-agent.exe ui.fill 'input[type="search"]' 'AppleSkin'
```

选择器匹配多项时只操作第一项，应使用实际控件的唯一 ID 或稳定属性缩小范围。`ui.click` 和 `ui.fill` 不会等待安装、下载或登录结束；完成情况应再查相应业务接口。能直接调用业务接口的操作，通常比依赖页面控件位置稳定。

## 怎样读返回结果

成功：

```json
{"ok":true,"result":{},"events":[]}
```

失败：

```json
{"ok":false,"error":"错误信息或错误对象"}
```

`ok` 只表示这次接口调用是否成功；`result` 的类型由具体接口决定。无返回值的接口成功时是 `null`，不是 `{}`。成功退出码为 0，失败为 1。失败的 `error` 可能是字符串或结构化对象，不要只按一种格式处理。

有些接口返回最终结果，例如本地文件读取；有些返回安装任务快照，例如 `install_create_instance`，此时还没有安装完。需要保存返回的 `job_id`，继续调用 `install_job_get`，检查 `status`、`phase`、`progress`、`error`。不要把“任务已创建”当作“游戏已安装”。

需要进度通道的顶层参数写成 `"onProgress":"$channel"`。CLI 收集最多 1000 条事件，放在命令结束时返回的 `events` 数组；**当前 CLI 不逐行实时输出这些事件**。长任务的实时状态可另开命令查询安装任务、启动进度或通用进度条；不同流程使用不同任务系统，见下表。

| 流程 | 开始 | 查询 | 取消 |
|---|---|---|---|
| 创建／修复／导入整合包等安装任务 | `plugin:install\|install_*` 对应接口 | `install_job_get` / `install_job_list` | `install_job_cancel`，UUID 任务 ID |
| CurseForge 模组及前置安装 | `plugin:gecl\|curseforge_install` | 命令返回事件；前端保持自己的下载任务状态 | `curseforge_cancel`，使用调用时的 UUID jobId |
| 原有 Modrinth 直接内容安装 | `instance_install_project_with_dependencies` 等 | `plugin:utils\|progress_bars_list`，完成后查询内容 | 并非所有旧直接调用都提供独立取消接口，不要冒用 install_job_cancel |
| 通用文件夹导入 | `plugin:gecl\|import_folder` | 进度通道 | `cancel_import`，同一个字符串 jobId |
| 指定位置的世界备份／恢复 | `plugin:gecl\|world_backup` | 进度通道 | `cancel_world_backup`，同一个字符串 jobId |
| Minecraft 启动 | `plugin:instance\|instance_run` | `plugin:gecl\|launch_progress`、进程和日志接口 | 结束游戏用 instance_kill / process_kill；dismiss_launch 只关闭进度显示 |
| 诊断 ZIP 导出 | `plugin:utils\|export_debug_info` | 进度通道 | `cancel_debug_info_export`，同一个 id |

默认最多等待 600 秒。可在运行命令前设置 `$env:GECL_AGENT_TIMEOUT_SECONDS='1800'`，允许值 1–3600 秒。超时不等于后台已停止，先查状态再决定取消或重试。重启程序可能来不及回传成功结果。单次请求总大小上限 1 MiB，字节数组接口只适合小文件，不应用来传大型整合包。

## ID、路径和参数规则

| 名称 | 从哪里取得 | 不要混用 |
|---|---|---|
| 游戏 ID `instanceId` | `instance_list`、导入结果或安装任务 | 不是游戏显示名称，也不是文件夹绝对路径 |
| 游玩账户 UUID `user` | `plugin:auth\|get_users` 中的账户／档案资料 | 不是 Modrinth 作者 ID |
| 安装任务 UUID `jobId` | 安装接口返回的 `job_id` | 不是游戏 ID |
| 进程记录 UUID `uuid` | `process_get_all` | 不是 Windows PID |
| Modrinth `projectId` / `versionId` | 项目搜索、项目详情和版本列表 | 都是平台字符串 ID；versionId 不是 `1.20.1` |
| CurseForge `projectId` / `fileId` | CF 搜索与文件列表 | 是数字；与 Modrinth ID 完全不同 |
| 世界 `world` | 游戏 `saves` 下的文件夹名 | 不是世界显示名称 |
| 已装资源 `projectPath` | 内容列表中的游戏内相对路径 | 管理已装文件用 `mods/a.jar`；安装本地文件用绝对源路径 |
| 截图 `key` | 截图列表的 instance_id 和 file_name | 不是截图标题、图片 URL 或 ID 字符串 |

顶层参数名按 camelCase，例如 `instanceId`、`onProgress`。**嵌套对象不统一转 camelCase**：`InstallCreateInstanceRequest` 和 CF 查询用 camelCase，Modrinth 依赖安装请求、分组成员条目、设置对象等仍可能用 snake_case。下方逐条模板按当前源码给出，不要自行统一大小写。

所有 `<游戏ID>`、`<任务UUID>`、`<实际文件名>` 等都是占位值。字节数组必须读取真实文件；`"<字节数组>"` 是说明占位，不能直接提交成字符串。复杂对象旁标有 Rust 类型，可在 [AGENT_TYPES.json](AGENT_TYPES.json) 查询完整声明和来源。同名类型可能来自不同模块，应按该接口源码导入选择；尤其 `EditInstance` 应看 `apps/app/src/api/instance.rs` 的外部请求类型，不要误用核心内部类型。

## 常用流程：按什么顺序调用

下表里的接口均通过 `gecl-agent.exe invoke '完整接口名' '@args.json'` 调用，具体 JSON 在后面每条接口里。

| 要做的事 | 推荐步骤 | 完成依据 |
|---|---|---|
| 添加离线账户 | `auth\|get_users` → `auth\|add_offline_user` → 必要时 `auth\|set_default_user` | 账户列表与默认账户查询 |
| 正版登录 | `auth\|begin_device_login` → 用户浏览器授权 → 按返回 interval 调 `auth\|poll_device_login` | status=complete；pending 时等待，cancelled 时结束 |
| 选择首次游戏位置 | `gecl\|setup_status` → `gecl\|choose_directory` → 添加账户 → `gecl\|finish_setup` | 重新查询 setup_status |
| 导入 PCL／本地游戏文件夹 | `gecl\|preview_folder` → 选择返回候选 → `gecl\|import_folder` → 查询内容 | 使用候选的 source、versionFile，不自己拼版本 JSON 路径 |
| 新建游戏 | `install\|install_create_instance` → `install\|install_job_get` | 任务成功，再查 instance_list |
| 安装本地 mrpack | `install\|install_get_modpack_preview` → `install\|install_create_modpack_instance` → 查任务 | 封面读取可另用 `gecl\|pack_artwork` |
| 搜索并安装 Modrinth 模组 | `cache\|get_search_results` → `cache\|get_project_versions` → `instance\|instance_install_project_with_dependencies` | 重查内容；目标游戏决定兼容条件 |
| 搜索并安装 CurseForge 模组 | `gecl\|curseforge_search` → `gecl\|curseforge_install` | fileId=null 自动选兼容文件并处理前置；返回后重查内容 |
| 添加自己下载的 jar／资源包 | `instance\|instance_add_project_from_path` → `instance\|instance_get_content_items` | 本地源路径是绝对路径，不修改原下载文件 |
| 导出轻量整合包 | `instance\|instance_get_pack_export_candidates` → `instance\|instance_export_mrpack` | 在导出路径检查 mrpack；一致的在线文件引用、修改及自制文件嵌入 |
| 设置游戏封面 | `instance\|instance_edit_icon` | 重查游戏配置；pack_artwork/cache_artwork 只缓存图片，不自动设置 |
| 备份／恢复世界 | `worlds\|get_instance_worlds` → `gecl\|world_backup` | 返回 path；恢复时另核对 recovery 的旧世界副本 |
| 启动及排错 | `instance\|instance_run` → `gecl\|launch_progress` → 日志接口 | 进程出现不等于已到主菜单；结合游戏输出判断 |
| 更新启动器设置 | `settings\|settings_get` → 修改结果中目标字段 → `settings\|settings_set` | 再读 settings_get；不要只传局部对象 |

表格为简写：例如 `auth|get_users` 的完整名称是 `plugin:auth|get_users`。

## 复杂参数的可用配方

### Modrinth 搜索与自动安装前置

`plugin:cache|get_search_results` 的参数文件：

```json
{"id":"?query=AppleSkin&limit=20&facets=[[\"project_type:mod\"],[\"versions:1.20.1\"],[\"categories:forge\"]]","cacheBehaviour":null}
```

从搜索结果取得真实项目 ID，给 `plugin:instance|instance_install_project_with_dependencies`：

```json
{"instanceId":"<游戏ID>","request":{"project_id":"<Modrinth项目ID>","version_id":null,"content_type":"mod"}}
```

`version_id=null` 自动按目标游戏选择；指定文件版本 ID 时仍受兼容检查。`content_type` 可为 `mod`、`plugin`、`datapack`、`resourcepack`、`shader`、`modpack`，不代表任意目标游戏均允许安装所有类型。`selected` 省略使用解析器默认依赖选择，不应凭空编造这个复杂对象。

### CurseForge 搜索与自动安装前置

`plugin:gecl|curseforge_search`：

```json
{"request":{"query":"AppleSkin","gameVersion":"1.20.1","loader":"forge","index":0,"pageSize":20}}
```

`query` 搜索词，`gameVersion` 游戏版本，`loader` 加载器；`index` 为记录起始下标，不是第几页；`pageSize` 为每页数量（1–50）。可选 `category` 是分类数字 ID，`slug` 为项目别名，`sort` 为当前提供方支持的排序值；不确定时省略，不自造枚举。

给 `plugin:gecl|curseforge_install` 的参数：

```json
{"jobId":"<调用前生成并记录的UUID>","instanceId":"<游戏ID>","projectId":238222,"fileId":null,"onProgress":"$channel"}
```

这里 238222 是示例项目数字 ID，实际使用搜索结果中的 ID。`fileId=null` 自动按游戏版本和加载器选文件及前置；指定 fileId 用于更换到所选兼容版本。该调用直接等待安装结果，不要把自定 jobId 交给 install_job_get；取消调用 `plugin:gecl|curseforge_cancel`。

### 本地游戏导入

先给 `plugin:gecl|preview_folder`：

```json
{"path":"D:/Plain Craft Launcher 2/.minecraft"}
```

选择返回数组中需要的候选，再给 `plugin:gecl|import_folder`：

```json
{"jobId":"import-1","source":"<候选source>","versionFile":"<候选versionFile>","onProgress":"$channel"}
```

候选返回字段是 `versionFile`（camelCase），不是 `version_file`。source 和 versionFile 都使用返回的真实路径；不要只用显示名称猜原目录。导入返回内部游戏 ID，可继续读取内容。`install_import_instance` 是另一套按特定启动器结构处理的入口，不应用 Unknown 类型代替通用目录识别。

### 整合包来源与封面

本地包 `location`：

```json
{"type":"fromFile","path":"D:/Packs/example.mrpack"}
```

在线 Modrinth 包 `location`：

```json
{"type":"fromVersionId","project_id":"<项目ID>","version_id":"<包版本ID>","title":"包名称","icon_url":null}
```

type 是 camelCase 枚举名，但在线来源内部字段仍是 `project_id`、`version_id`、`icon_url`。`postInstallEdit` 用 camelCase，例如 `{"name":"我的游戏","iconPath":"D:/Pictures/pack.png"}`。其中 iconPath 省略保留原封面，null 清除封面，字符串设置自己的图片。

导出 `instance_export_mrpack` 的 versionId 是自己给包写的版本标签，例如 `1.0.0`；安装下载时的 versionId 则是平台文件 ID，这两个意义不同。

### 外观与下载源

`appearance_set.preferences`：

```json
{"accentColor":"#176fb0","backgroundId":null,"backgroundEnabled":true,"blur":8,"overlay":18,"surfaceOpacity":88,"position":"center"}
```

| 字段 | 意义／范围 |
|---|---|
| accentColor | 六位十六进制主题色 |
| backgroundId | appearance_import_background 返回的背景副本 ID；null 无背景 |
| backgroundEnabled | 是否启用已选背景 |
| blur | 背景模糊，0–24 px |
| overlay | 背景遮罩，0–60 |
| surfaceOpacity | 内容面板不透明度，80–100 |
| position | center 居中、top 顶部、bottom 底部 |

原图导入支持 PNG/JPG/WebP，最大 32 MiB，应用保存规范化副本。先 appearance_get 读取当前偏好，再按需修改，避免把示例默认值覆盖成自己的设置。可选 `window` 为 `{"theme":"light","advancedRendering":true,"nativeDecorations":false}`；theme 可为 dark/light/oled/retro/system，advancedRendering 对应高级渲染，nativeDecorations 对应系统标题栏，后者按界面原逻辑可能需要重启生效。

`download_sources_set.settings`：

```json
{"policy":"official_preferred","slow_kib_per_second":128,"observation_seconds":8}
```

policy：official_only 只用官方、official_preferred 官方优先、mirror_preferred 镜像优先、mirror_only 只用镜像。慢速阈值为 16–8192 KiB/s，观察时长 3–60 秒。这是下载源策略，不是系统代理开关，也不保证所有模组平台都有镜像。

### 完整设置、Java 和皮肤对象

这些接口需要完整结构，不能把对象替换成一个名称、数字或空对象。

```powershell
# 保存完整设置的正确方式：先读取，再只修改一个真实字段
$response = .\gecl-agent.exe invoke 'plugin:settings|settings_get' '{}' | ConvertFrom-Json
if (-not $response.ok) { throw ($response.error | ConvertTo-Json -Depth 20) }
$current = $response.result
$current.show_play_time = $true
@{ settings = $current } | ConvertTo-Json -Depth 50 |
    Set-Content -LiteralPath .\args.json -Encoding utf8
.\gecl-agent.exe invoke 'plugin:settings|settings_set' '@args.json'
```

`set_java_version.javaVersion` 应使用 `jre_get_jre` 返回的完整 JavaVersion；`jre_auto_install_java.javaVersion` 则仅传主版本整数，例如 17。`equip_skin.skin` / `save_custom_skin.skin` 使用 get_available_skins 的实际完整对象；`cape` 使用 get_available_capes 的对象或 null。皮肤 `variant` 是大写 CLASSIC / SLIM，不是 classic/slim。`textureBlob` 等图片字节数组需要从真实图片读取，且请求不能超过 1 MiB。

### 游戏设置修订与冲突

`instance_get_local_game_options_config` 或 `instance_get_synced_game_options_config` 返回 `summary_revision`、`canonical_revision`、`catalog_revision` 和设置项数组。修改请求必须带对应修订号：

```json
{
  "expected_summary_revision":"<读取到的summary_revision>",
  "expected_canonical_revision":0,
  "expected_catalog_revision":0,
  "changes":[{
    "option_id":"<读取到的option_id>",
    "base_option_revision":0,
    "sync_enabled":true
  }]
}
```

上面三个 0 是**待替换的结构示意**，必须使用读取结果中的真实数值；不能照抄 0 来绕过冲突检测。`base_option_revision` 对应该设置的 option_revision。`canonical_value` 省略表示不改值，null 清除显式值；要赋新值，先核对 CanonicalValue 类型及该设置 editor 支持的范围。先 preview，检查冲突后才 save；冲突时重新读取，不强行覆盖。

保存结果中的 applied 是应用数量，deferred 是延后处理数量，unsupported 是不支持数量，failed 是失败数量，conflicts 为冲突信息。不要只看外层 ok=true 就认为全部改动已生效。

### 分组、截图与同步

- 游戏成员更新：`{"instance_id":"<游戏ID>","group_ids":["<分组ID>"]}`。
- 截图成员更新：`{"screenshot_id":"<截图ID>","group_id":"<分组ID>"}`；group_id=null 清除分组。
- 截图键：`{"instance_id":"<游戏ID>","file_name":"<图片文件名.png>"}`。
- 截图保存模式：create_copy 另存副本、replace_edit 覆盖当前图。
- 加入同步冲突决定：use_synced 使用共享数据、use_instance 使用当前游戏数据；先查询加入预览。
- 同步选项：game_options 游戏设置、command_history 命令历史、multiplayer_servers 多人服务器、creative_hotbars 创造快捷栏、screenshots 截图、resource_packs 资源包。data_packs 存在于兼容枚举，但当前不可开启。

全局同步资源不是在线共享整合包；`instance_share_*` 属于上游云共享，`instance_sync_pack` 等是本机同步资源。删除分组不删除游戏，删除截图分组不删除图片，delete_screenshots 才删除图片文件。

## 可用性与验证边界

接口分为本地读取、查询与缓存、本地写入／删除、原生对话框、内部流程、已禁用占位、上游服务查询／写入。每条都单独标注。`agent_mode` 是兼容保留的能力查询，正式版默认 true；`is_dev` 是 Rust 编译模式查询，优化正式构建为 false。`mr-auth` 已停用；自动更新接口在当前构建禁用；好友、共享发布、平台资料修改等需要上游账户，而 GeCL 没有平台账户登录入口，不能因为清单中有名字就承诺可调用成功。公开作者和项目信息查询仍可按平台规则读取。

这份清单覆盖应用自身注册 handler；不把 Tauri 的底层插件私有传输方法或模块名作为产品命令。`plugin:agent|ready` / `complete` 属于内部桥接，通用 invoke 会拒绝它们，不应手动调用。当前没有开放网络 HTTP 端口，也没有实现 AI 聊天窗口；任何能在本机执行进程并读 JSON 的 AI 工具都可使用这份 CLI，不依赖特定模型 SDK。

本次逐项说明来自注册表、Tauri 包装函数与相关核心源码，已经检查覆盖率、必填参数、示例 JSON 及枚举取值；没有为写文档逐一执行删除、发布、举报、真实登录等接口，也没有宣称 296 条都做过真实业务验收。复杂结构中标记 `x-schema-complete:false` 的部分是显式保留的结构限制，使用类型附录和真实查询返回构造参数。

## 隔离调试与文档维护

给启动器和 CLI 设置同一个目录可将测试配置与日常配置隔离：

```powershell
$env:THESEUS_CONFIG_DIR='D:/GeCL-Test/Settings'
Start-Process .\gecl.exe -WindowStyle Hidden
.\gecl-agent.exe status
```

先关闭另一个 GeCL，避免单实例限制导致测试程序退出。这个环境变量选择的是配置目录；游戏数据位置仍按首次设置选择。命令通过当前 Windows 用户的本地文件队列传给正在运行的界面，不监听网络端口。工具自动处理会话新旧，无需用户复制密码。不要把真实账户配置、调试请求结果或备份包混入源码归档。

机器可读说明：[AGENT_COMMANDS.json](AGENT_COMMANDS.json)。调用概念模型：[AGENT_API_SCHEMA.json](AGENT_API_SCHEMA.json)。复杂类型及序列化声明：[AGENT_TYPES.json](AGENT_TYPES.json)。逐条中文说明的维护源位于源码 `docs/AGENT_COMMAND_HELP.json`。其中 API_SCHEMA 是 CLI 操作的概念结构，不是可直接写入文件队列的底层协议包。

在源码目录运行 `python scripts/gecl-agent-catalog.py` 可重新生成清单；若新注册接口没有中文说明，生成器会报错，避免再次只增加名称却遗漏作用解释。实际程序内嵌清单只有重建正式版后才会更新。

<!-- BEGIN GENERATED COMMANDS -->

## 全部接口逐条说明

本表按当前源码注册表整理，共 **296 条应用接口**。接口注册与实际可用性分开说明；没有把上游平台功能当作已经开放的 GeCL 本地功能。

每条都列出用途、参数意义、结果、数据影响及调用模板。外层参数是 camelCase；嵌套对象必须遵守自己的字段格式。尖括号内容均为需要替换的占位值，不是能直接照抄执行的真实 ID。无参数接口传 `{}`。

- [应用与窗口（11 条）](#api-root)
- [游玩账户与微软设备登录（9 条）](#api-auth)
- [Modrinth 查询与缓存（18 条）](#api-cache)
- [游戏文件与系统保存对话框（12 条）](#api-files)
- [上游好友兼容接口（4 条）](#api-friends)
- [GeCL 专用功能与 CurseForge（26 条）](#api-gecl)
- [其他启动器目录识别（3 条）](#api-import)
- [安装任务与整合包（21 条）](#api-install)
- [游戏、内容、分组、截图与同步（97 条）](#api-instance)
- [Java 运行环境（7 条）](#api-jre)
- [游戏日志（9 条）](#api-logs)
- [安装元数据（2 条）](#api-metadata)
- [正版皮肤库（12 条）](#api-minecraft-skins)
- [已停用的平台账户接口（2 条）](#api-mr-auth)
- [引导状态（1 条）](#api-onboarding-checklist)
- [游戏进程（4 条）](#api-process)
- [上游举报接口（1 条）](#api-reports)
- [启动器设置与内容缓存（9 条）](#api-settings)
- [系统快捷方式（1 条）](#api-shortcuts)
- [搜索筛选标签（5 条）](#api-tags)
- [平台用户与作者（13 条）](#api-users)
- [系统工具与诊断（12 条）](#api-utils)
- [世界与多人服务器（17 条）](#api-worlds)

<a id="api-root"></a>

### 应用与窗口

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`agent_mode`](#cmd-root-agent_mode) | 检查本机命令接口是否启用 | 本地查询 |
| [`are_updates_enabled`](#cmd-root-are_updates_enabled) | 检查自动更新是否启用 | 本地查询 |
| [`enqueue_update_for_installation`](#cmd-root-enqueue_update_for_installation) | 预约安装启动器更新 | 当前禁用／兼容占位 |
| [`get_update_size`](#cmd-root-get_update_size) | 读取待更新程序大小 | 当前禁用／兼容占位 |
| [`initialize_state`](#cmd-root-initialize_state) | 初始化后端状态与事件通道 | 内部接口 |
| [`is_dev`](#cmd-root-is_dev) | 检查开发编译标志 | 本地查询 |
| [`remove_enqueued_update`](#cmd-root-remove_enqueued_update) | 取消已预约的启动器更新 | 当前禁用／兼容占位 |
| [`restart_app`](#cmd-root-restart_app) | 重新启动启动器 | 界面／系统打开 |
| [`set_restart_after_pending_update`](#cmd-root-set_restart_after_pending_update) | 设置更新后的重启标记 | 内部接口 |
| [`show_window`](#cmd-root-show_window) | 显示启动器主窗口 | 界面／系统打开 |
| [`toggle_decorations`](#cmd-root-toggle_decorations) | 切换系统窗口边框 | 界面／系统打开 |

<a id="cmd-root-agent_mode"></a>

#### `agent_mode` — 检查本机命令接口是否启用

返回本次构建是否启用本机命令接口。统一正式版默认返回 true；agent_mode 名称为兼容旧调用保留，不代表软件处于 Rust 调试编译模式。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`bool`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'agent_mode' '{}'
```

源码位置：`apps/app/src/main.rs:70`（相对源码根目录）。

<a id="cmd-root-are_updates_enabled"></a>

#### `are_updates_enabled` — 检查自动更新是否启用

检查当前构建和环境是否允许应用自身的更新机制；不是检查 Minecraft 更新。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`bool`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'are_updates_enabled' '{}'
```

源码位置：`apps/app/src/main.rs:78`（相对源码根目录）。

<a id="cmd-root-enqueue_update_for_installation"></a>

#### `enqueue_update_for_installation` — 预约安装启动器更新

用于上游自动更新流程。本次交付未启用 updater，调用会返回更新已禁用的错误，不会安装东西。

**影响与可用性：** 当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。

**参数：** 无，传 `{}`。

**返回：** 当前构建返回 updates are disabled in this build 错误。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'enqueue_update_for_installation' '{}'
```

源码位置：`apps/app/src/updater_impl_noop.rs:13`（相对源码根目录）。

<a id="cmd-root-get_update_size"></a>

#### `get_update_size` — 读取待更新程序大小

用于上游自动更新流程。本次交付未启用 updater，因此不返回大小而返回禁用错误。

**影响与可用性：** 当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。

**参数：** 无，传 `{}`。

**返回：** 当前构建返回 updates are disabled in this build 错误，不返回文件大小。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'get_update_size' '{}'
```

源码位置：`apps/app/src/updater_impl_noop.rs:8`（相对源码根目录）。

<a id="cmd-root-initialize_state"></a>

#### `initialize_state` — 初始化后端状态与事件通道

只供界面启动时建立数据库、目录和事件连接；应用已初始化后不要重复调用。不是首次设置目录的入口。

**影响与可用性：** 内部接口：供应用自身流程使用，一般业务自动化不应重复调用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `events` | 是 | tauri::ipc::Channel<tauri::ipc::InvokeResponseBody> | 启动初始化事件通道，传字面量 "$channel"；只供应用启动流程使用。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`api::Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "events": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'initialize_state' '@args.json'
```

源码位置：`apps/app/src/main.rs:27`（相对源码根目录）。

<a id="cmd-root-is_dev"></a>

#### `is_dev` — 检查开发编译标志

返回是否使用 Rust debug_assertions 编译；Agent 版也是优化的 release 构建，不能用此值判断 Agent 接口是否开放。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`bool`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'is_dev' '{}'
```

源码位置：`apps/app/src/main.rs:73`（相对源码根目录）。

<a id="cmd-root-remove_enqueued_update"></a>

#### `remove_enqueued_update` — 取消已预约的启动器更新

当前禁用更新的构建中为空操作；不取消任何游戏、模组下载任务。

**影响与可用性：** 当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。

**参数：** 无，传 `{}`。

**返回：** 当前构建执行兼容空操作，result 为 null。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'remove_enqueued_update' '{}'
```

源码位置：`apps/app/src/updater_impl_noop.rs:22`（相对源码根目录）。

<a id="cmd-root-restart_app"></a>

#### `restart_app` — 重新启动启动器

重启 GeCL 应用进程，不等于重启 Minecraft。重启会中断当前命令通道，可能拿不到本次回复。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'restart_app' '{}'
```

源码位置：`apps/app/src/main.rs:99`（相对源码根目录）。

<a id="cmd-root-set_restart_after_pending_update"></a>

#### `set_restart_after_pending_update` — 设置更新后的重启标记

只设置应用待更新流程的重启选项，不会立即重启；当前交付未开启自动更新。

**影响与可用性：** 内部接口：供应用自身流程使用，一般业务自动化不应重复调用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `shouldRestart` | 是 | bool | 是否在上游待安装更新处理后重启；不是立即重启命令。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`api::Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "shouldRestart": false
}
```

```powershell
.\gecl-agent.exe invoke 'set_restart_after_pending_update' '@args.json'
```

源码位置：`apps/app/src/main.rs:107`（相对源码根目录）。

<a id="cmd-root-show_window"></a>

#### `show_window` — 显示启动器主窗口

让隐藏的 GeCL 主窗口显示并取得焦点；不启动游戏。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'show_window' '{}'
```

源码位置：`apps/app/src/main.rs:53`（相对源码根目录）。

<a id="cmd-root-toggle_decorations"></a>

#### `toggle_decorations` — 切换系统窗口边框

调整当前 GeCL 窗口是否使用系统标题栏/边框；参数 b 为开关，不修改 Minecraft 窗口。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `b` | 是 | bool | 是否启用系统窗口装饰；true 开启，false 关闭。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`api::Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "b": false
}
```

```powershell
.\gecl-agent.exe invoke 'toggle_decorations' '@args.json'
```

源码位置：`apps/app/src/main.rs:91`（相对源码根目录）。

<a id="api-auth"></a>

### 游玩账户与微软设备登录

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`add_offline_user`](#cmd-auth-add_offline_user) | 添加离线游玩账户 | 写入操作 |
| [`begin_device_login`](#cmd-auth-begin_device_login) | 开始微软正版登录 | 联网写入 |
| [`cancel_device_login`](#cmd-auth-cancel_device_login) | 取消微软登录会话 | 写入操作 |
| [`check_reachable`](#cmd-auth-check_reachable) | 检查微软/Mojang 验证服务连接 | 查询／缓存 |
| [`get_default_user`](#cmd-auth-get_default_user) | 查询当前选中的游玩账户 | 本地查询 |
| [`get_users`](#cmd-auth-get_users) | 列出本机游玩账户 | 本地查询 |
| [`poll_device_login`](#cmd-auth-poll_device_login) | 查询微软登录进度 | 联网写入 |
| [`remove_user`](#cmd-auth-remove_user) | 移除本机游玩账户 | 删除／清理 |
| [`set_default_user`](#cmd-auth-set_default_user) | 切换游玩账户 | 写入操作 |

<a id="cmd-auth-add_offline_user"></a>

#### `plugin:auth|add_offline_user` — 添加离线游玩账户

校验 3–16 位英文字母、数字或下划线的用户名并创建离线身份；返回账户对象，其名称在 profile.name、UUID 在 profile.id。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `username` | 是 | String | 离线游戏名，使用 3–16 位英文字母、数字或下划线；不是微软邮箱。 |

**返回：** 游玩账户的安全序列化资料；用于识别账户及档案，不应作为可导出的登录令牌。 Rust 返回类型：`Result<Credentials>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "username": "TestUser"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:auth|add_offline_user' '@args.json'
```

源码位置：`apps/app/src/api/auth.rs:41`（相对源码根目录）。

<a id="cmd-auth-begin_device_login"></a>

#### `plugin:auth|begin_device_login` — 开始微软正版登录

创建微软设备代码登录会话；根据返回的验证网址和代码在浏览器授权，之后用 poll_device_login 查询。不会替用户自动完成网页登录。

**影响与可用性：** 联网写入：会提交网络请求并修改账户或本地状态，按本条用途及授权条件使用。

**参数：** 无，传 `{}`。

**返回：** 设备登录会话，包含会话 ID、用户验证码、验证地址和轮询信息；需要用户在浏览器完成微软授权。 Rust 返回类型：`Result<minecraft_auth::GeclDeviceLogin>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:auth|begin_device_login' '{}'
```

源码位置：`apps/app/src/api/auth.rs:23`（相对源码根目录）。

<a id="cmd-auth-cancel_device_login"></a>

#### `plugin:auth|cancel_device_login` — 取消微软登录会话

取消 id 对应的设备登录流程，不会删除已经保存的账户。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | uuid::Uuid | begin_device_login 返回的设备登录会话 UUID；不是账户 ID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:auth|cancel_device_login' '@args.json'
```

源码位置：`apps/app/src/api/auth.rs:35`（相对源码根目录）。

<a id="cmd-auth-check_reachable"></a>

#### `plugin:auth|check_reachable` — 检查微软/Mojang 验证服务连接

请求验证服务以检查连通性；成功返回空结果，失败返回网络错误，不会添加账户。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:auth|check_reachable' '{}'
```

源码位置：`apps/app/src/api/auth.rs:47`（相对源码根目录）。

<a id="cmd-auth-get_default_user"></a>

#### `plugin:auth|get_default_user` — 查询当前选中的游玩账户

返回当前默认账户的 UUID；没有选中账户时为 null，不返回完整账户资料。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 当前默认游玩账户的 UUID 字符串；没有默认账户时为 null。 Rust 返回类型：`Result<Option<uuid::Uuid>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:auth|get_default_user' '{}'
```

源码位置：`apps/app/src/api/auth.rs:58`（相对源码根目录）。

<a id="cmd-auth-get_users"></a>

#### `plugin:auth|get_users` — 列出本机游玩账户

返回微软与离线账户的公开资料列表；使用 profile.id 作为后续选择或删除的 user 参数。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：游玩账户的安全序列化资料；用于识别账户及档案，不应作为可导出的登录令牌。 Rust 返回类型：`Result<Vec<Credentials>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:auth|get_users' '{}'
```

源码位置：`apps/app/src/api/auth.rs:69`（相对源码根目录）。

<a id="cmd-auth-poll_device_login"></a>

#### `plugin:auth|poll_device_login` — 查询微软登录进度

使用 begin_device_login 返回的会话 id 检查授权状态；授权成功时按正常拥有权校验流程保存账户。

**影响与可用性：** 联网写入：会提交网络请求并修改账户或本地状态，按本条用途及授权条件使用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | uuid::Uuid | begin_device_login 返回的设备登录会话 UUID；不是账户 ID。 |

**返回：** 设备登录轮询状态；按返回状态判断等待、成功、到期或错误，不要把所有响应都视为登录成功。 Rust 返回类型：`Result<minecraft_auth::GeclDevicePoll>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:auth|poll_device_login' '@args.json'
```

源码位置：`apps/app/src/api/auth.rs:28`（相对源码根目录）。

<a id="cmd-auth-remove_user"></a>

#### `plugin:auth|remove_user` — 移除本机游玩账户

移除 user 指定的本地账户记录；不删除微软账号本身，也不删除游戏或世界。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `user` | 是 | uuid::Uuid | 本地 Minecraft/离线游玩账户 UUID，从 auth 的 get_users 返回结果取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "user": "<账户UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:auth|remove_user' '@args.json'
```

源码位置：`apps/app/src/api/auth.rs:53`（相对源码根目录）。

<a id="cmd-auth-set_default_user"></a>

#### `plugin:auth|set_default_user` — 切换游玩账户

将已有 user UUID 设为默认游玩账户；不是添加新账户，也不会改变已运行游戏的身份。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `user` | 是 | uuid::Uuid | 本地 Minecraft/离线游玩账户 UUID，从 auth 的 get_users 返回结果取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "user": "<账户UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:auth|set_default_user' '@args.json'
```

源码位置：`apps/app/src/api/auth.rs:63`（相对源码根目录）。

<a id="api-cache"></a>

### Modrinth 查询与缓存

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`get_organization`](#cmd-cache-get_organization) | 查询 Modrinth 组织 | 查询／缓存 |
| [`get_organization_many`](#cmd-cache-get_organization_many) | 批量查询 Modrinth 组织 | 查询／缓存 |
| [`get_project`](#cmd-cache-get_project) | 查询一个 Modrinth 项目 | 查询／缓存 |
| [`get_project_many`](#cmd-cache-get_project_many) | 批量查询 Modrinth 项目 | 查询／缓存 |
| [`get_project_v3`](#cmd-cache-get_project_v3) | 查询一个 Modrinth v3 项目 | 查询／缓存 |
| [`get_project_v3_many`](#cmd-cache-get_project_v3_many) | 批量查询 Modrinth v3 项目 | 查询／缓存 |
| [`get_project_versions`](#cmd-cache-get_project_versions) | 查询项目的版本清单 | 查询／缓存 |
| [`get_search_results`](#cmd-cache-get_search_results) | 搜索 Modrinth 内容 | 查询／缓存 |
| [`get_search_results_many`](#cmd-cache-get_search_results_many) | 批量执行 Modrinth 搜索 | 查询／缓存 |
| [`get_search_results_v3`](#cmd-cache-get_search_results_v3) | 使用 v3 搜索 Modrinth 内容 | 查询／缓存 |
| [`get_search_results_v3_many`](#cmd-cache-get_search_results_v3_many) | 批量执行 v3 搜索 | 查询／缓存 |
| [`get_team`](#cmd-cache-get_team) | 查询项目维护团队 | 查询／缓存 |
| [`get_team_many`](#cmd-cache-get_team_many) | 批量查询项目团队 | 查询／缓存 |
| [`get_user`](#cmd-cache-get_user) | 查询 Modrinth 用户公开资料 | 查询／缓存 |
| [`get_user_many`](#cmd-cache-get_user_many) | 批量查询 Modrinth 用户 | 查询／缓存 |
| [`get_version`](#cmd-cache-get_version) | 查询一个 Modrinth 文件版本 | 查询／缓存 |
| [`get_version_many`](#cmd-cache-get_version_many) | 批量查询 Modrinth 文件版本 | 查询／缓存 |
| [`purge_cache_types`](#cmd-cache-purge_cache_types) | 清除指定种类的元数据缓存 | 删除／清理 |

<a id="cmd-cache-get_organization"></a>

#### `plugin:cache|get_organization` — 查询 Modrinth 组织

按组织 ID 或该接口支持的别名取得组织资料。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 Organization 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Organization>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_organization' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_organization_many"></a>

#### `plugin:cache|get_organization_many` — 批量查询 Modrinth 组织

按 ids 取得多个组织资料，不修改组织或成员。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 Organization 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Organization>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_organization_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_project"></a>

#### `plugin:cache|get_project` — 查询一个 Modrinth 项目

按项目 ID 或该查询支持的别名读取项目资料，可命中本地缓存，必要时访问 Modrinth。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** Modrinth 项目资料，包括名称、简介、作者团队、图标、下载量、项目类型和版本关联等；具体字段按平台对象定义读取。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Project>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_project' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_project_many"></a>

#### `plugin:cache|get_project_many` — 批量查询 Modrinth 项目

按 ids 一次读取多个项目资料，减少逐个查询；返回找到的项目列表。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth 项目资料，包括名称、简介、作者团队、图标、下载量、项目类型和版本关联等；具体字段按平台对象定义读取。 Rust 返回类型：`Result<Vec<Project>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_project_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_project_v3"></a>

#### `plugin:cache|get_project_v3` — 查询一个 Modrinth v3 项目

读取 v3 格式的项目对象；字段结构与 get_project 的旧项目格式不同。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** Modrinth v3 项目资料；与旧版 Project 字段结构不同，不能直接混用。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<ProjectV3>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_project_v3' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_project_v3_many"></a>

#### `plugin:cache|get_project_v3_many` — 批量查询 Modrinth v3 项目

按 ids 读取多个 v3 项目对象；不要把结果字段当作旧版 v2 项目字段使用。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth v3 项目资料；与旧版 Project 字段结构不同，不能直接混用。 Rust 返回类型：`Result<Vec<ProjectV3>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_project_v3_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_project_versions"></a>

#### `plugin:cache|get_project_versions` — 查询项目的版本清单

按 projectId 读取该项目发布的文件版本列表；这是项目版本，不是 Minecraft 版本列表。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `projectId` | 是 | &str | 平台项目 ID；Modrinth 是字符串，CurseForge 是数字，不能互换。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth 文件版本资料，包括 id、version_number、game_versions、loaders、files、dependencies 等。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Vec<Version>>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "projectId": "<Modrinth项目ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_project_versions' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:73`（相对源码根目录）。

<a id="cmd-cache-get_search_results"></a>

#### `plugin:cache|get_search_results` — 搜索 Modrinth 内容

id 是搜索查询串，不是项目 ID，例如 ?query=AppleSkin&limit=20；返回搜索结果对象。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | Modrinth 搜索查询串，例如 ?query=AppleSkin&limit=20；不是只传搜索词。v3 接口使用其对应搜索参数。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** Modrinth 搜索结果对象；hits 为项目记录，total_hits 为该平台结果总数，另有 offset、limit 等分页信息。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<SearchResults>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "?query=AppleSkin&limit=20",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_search_results' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_search_results_many"></a>

#### `plugin:cache|get_search_results_many` — 批量执行 Modrinth 搜索

ids 是多个搜索查询串，一次返回多组查询结果；不是把多个关键词直接拼成项目 ID。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth 搜索结果对象；hits 为项目记录，total_hits 为该平台结果总数，另有 offset、limit 等分页信息。 Rust 返回类型：`Result<Vec<SearchResults>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "?query=AppleSkin&limit=20"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_search_results_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_search_results_v3"></a>

#### `plugin:cache|get_search_results_v3` — 使用 v3 搜索 Modrinth 内容

按搜索查询串取得 v3 结果，字段和筛选规则与旧搜索接口可能不同。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | Modrinth 搜索查询串，例如 ?query=AppleSkin&limit=20；不是只传搜索词。v3 接口使用其对应搜索参数。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** Modrinth v3 搜索结果对象；按 v3 的字段结构读取，不套用旧版字段。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<SearchResultsV3>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "?query=AppleSkin&limit=20",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_search_results_v3' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_search_results_v3_many"></a>

#### `plugin:cache|get_search_results_v3_many` — 批量执行 v3 搜索

ids 为多个 v3 搜索查询串，返回多组 v3 搜索结果。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth v3 搜索结果对象；按 v3 的字段结构读取，不套用旧版字段。 Rust 返回类型：`Result<Vec<SearchResultsV3>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "?query=AppleSkin&limit=20"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_search_results_v3_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_team"></a>

#### `plugin:cache|get_team` — 查询项目维护团队

id 为团队 ID，返回团队成员及角色资料，不是本机 Minecraft 账户。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 TeamMember 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Vec<TeamMember>>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_team' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_team_many"></a>

#### `plugin:cache|get_team_many` — 批量查询项目团队

ids 为团队 ID 数组；每项结果本身是一个团队成员列表。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：数组；每项为：序列化的 TeamMember 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Vec<TeamMember>>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_team_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_user"></a>

#### `plugin:cache|get_user` — 查询 Modrinth 用户公开资料

这里的 user 是 Modrinth 网站用户；本机微软/离线账户应使用 plugin:auth。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 User 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<User>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_user' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_user_many"></a>

#### `plugin:cache|get_user_many` — 批量查询 Modrinth 用户

按 ids 查询网站用户公开资料；不列出本机保存的游玩账户。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 User 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<User>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_user_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_version"></a>

#### `plugin:cache|get_version` — 查询一个 Modrinth 文件版本

id 是 Modrinth version ID，返回版本号、下载文件、依赖和兼容信息；不是 Minecraft 版本字符串。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | &str | 本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** Modrinth 文件版本资料，包括 id、version_number、game_versions、loaders、files、dependencies 等。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Version>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_version' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-get_version_many"></a>

#### `plugin:cache|get_version_many` — 批量查询 Modrinth 文件版本

按 ids 查询多个文件版本；可用于组装整合包依赖清单。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `ids` | 是 | Vec<String> | 待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：Modrinth 文件版本资料，包括 id、version_number、game_versions、loaders、files、dependencies 等。 Rust 返回类型：`Result<Vec<Version>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "ids": [
    "<对应ID或相对路径>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|get_version_many' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:8`（相对源码根目录）。

<a id="cmd-cache-purge_cache_types"></a>

#### `plugin:cache|purge_cache_types` — 清除指定种类的元数据缓存

按 cacheTypes 清除本地 API 元数据缓存，后续读取可能需要联网；不会清除游戏存档。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `cacheTypes` | 是 | Vec<CacheValueType>；数组项 project, project_v3, version, version_v3, user, team, organization, file, loader_manifest, minecraft_manifest, categories, report_types, loaders, game_versions, donation_platforms, file_hash, file_update, search_results, search_results_v3, modpack_files, embedded_content_metadata, project_versions | 需要清理的缓存类别数组；不会删除已安装游戏。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "cacheTypes": [
    "project"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:cache|purge_cache_types' '@args.json'
```

源码位置：`apps/app/src/api/cache.rs:68`（相对源码根目录）。

<a id="api-files"></a>

### 游戏文件与系统保存对话框

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`file_create_directory`](#cmd-files-file_create_directory) | 在游戏目录内创建文件夹 | 写入操作 |
| [`file_delete`](#cmd-files-file_delete) | 删除游戏目录内的文件或文件夹 | 删除／清理 |
| [`file_extract_zip`](#cmd-files-file_extract_zip) | 在游戏目录内解压 ZIP | 写入操作 |
| [`file_list`](#cmd-files-file_list) | 列出游戏目录内容 | 本地查询 |
| [`file_read`](#cmd-files-file_read) | 读取游戏内文件字节 | 本地查询 |
| [`file_read_dragged_file`](#cmd-files-file_read_dragged_file) | 读取指定本地文件字节 | 本地查询 |
| [`file_rename`](#cmd-files-file_rename) | 移动或重命名游戏内文件 | 写入操作 |
| [`file_save_as`](#cmd-files-file_save_as) | 把游戏内文件另存到外部 | 原生对话框 |
| [`file_write`](#cmd-files-file_write) | 写入游戏内文件 | 写入操作 |
| [`files_release_external`](#cmd-files-files_release_external) | 释放或取消外部保存会话 | 写入操作 |
| [`files_save_external`](#cmd-files-files_save_external) | 下载到已选外部文件 | 写入操作 |
| [`files_select_external`](#cmd-files-files_select_external) | 选择外部下载的保存位置 | 原生对话框 |

<a id="cmd-files-file_create_directory"></a>

#### `plugin:files|file_create_directory` — 在游戏目录内创建文件夹

path 是指定游戏目录下的相对路径；会通过现有路径边界检查。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `path` | 是 | &str | 游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "path": "config/<目标文件>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_create_directory' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:183`（相对源码根目录）。

<a id="cmd-files-file_delete"></a>

#### `plugin:files|file_delete` — 删除游戏目录内的文件或文件夹

recursive=true 才允许递归删除目录；这是实际删除操作，可能包含用户文件，不是移到回收站的承诺。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `path` | 是 | &str | 游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。 |
| `recursive` | 是 | bool | 是否允许递归删除目录及其内容；删除前确认路径确属目标。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "path": "config/<目标文件>",
  "recursive": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_delete' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:205`（相对源码根目录）。

<a id="cmd-files-file_extract_zip"></a>

#### `plugin:files|file_extract_zip` — 在游戏目录内解压 ZIP

filePath 指向游戏内 ZIP，解压到 ZIP 所在文件夹。dryRun=true 只列冲突；overrideConflicts 决定是否覆盖已有文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `filePath` | 是 | &str | 游戏内文件相对路径，例如 mods/example.jar 或临时存放的 ZIP。 |
| `overrideConflicts` | 是 | bool | 解压冲突时是否覆盖已有文件；配合 dryRun 先预检。 |
| `dryRun` | 是 | bool | true 只预检解压冲突；false 执行解压。 |

**返回：** 序列化的 ExtractDryRunResult 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<ExtractDryRunResult>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "filePath": "<游戏内的压缩包.zip>",
  "overrideConflicts": false,
  "dryRun": true
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_extract_zip' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:74`（相对源码根目录）。

<a id="cmd-files-file_list"></a>

#### `plugin:files|file_list` — 列出游戏目录内容

path 为游戏内相对目录，空字符串表示游戏根目录；返回文件与子目录条目。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `path` | 是 | &str | 游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。 |

**返回：** 数组；每项为：序列化的 theseus::instance::InstanceFileItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::InstanceFileItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "path": ""
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_list' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:154`（相对源码根目录）。

<a id="cmd-files-file_read"></a>

#### `plugin:files|file_read` — 读取游戏内文件字节

返回字节数组，不自动按 UTF-8 转文本；适合配置读取或二进制文件检查。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `path` | 是 | &str | 游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。 |

**返回：** 文件原始字节数组，每项为 0–255；若是文本需由调用者按正确编码解码。 Rust 返回类型：`Result<Vec<u8>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "path": "config/<目标文件>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_read' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:162`（相对源码根目录）。

<a id="cmd-files-file_read_dragged_file"></a>

#### `plugin:files|file_read_dragged_file` — 读取指定本地文件字节

读取一个绝对路径的普通文件并返回字节数组；文件会整体进入内存，不适合把大整合包当普通小文件读取。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | String | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |

**返回：** 文件原始字节数组，每项为 0–255；若是文本需由调用者按正确编码解码。 Rust 返回类型：`Result<Vec<u8>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_read_dragged_file' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:64`（相对源码根目录）。

<a id="cmd-files-file_rename"></a>

#### `plugin:files|file_rename` — 移动或重命名游戏内文件

source、destination 都是同一游戏目录下的相对路径；不用于任意磁盘路径搬迁。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `source` | 是 | &str | 重命名前的游戏内相对路径，例如 mods/old.jar。 |
| `destination` | 是 | &str | 重命名或移动后的游戏内相对路径，不能越出游戏目录。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "source": "mods/<原文件.jar>",
  "destination": "mods/<新文件名.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_rename' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:191`（相对源码根目录）。

<a id="cmd-files-file_save_as"></a>

#### `plugin:files|file_save_as` — 把游戏内文件另存到外部

弹出原生保存位置对话框，将 filePath 指定的游戏文件复制出去；需要有人选择目的位置。

**影响与可用性：** 原生对话框：需要桌面可交互；未完成选择前调用可能一直等待。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `filePath` | 是 | &str | 游戏内文件相对路径，例如 mods/example.jar 或临时存放的 ZIP。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "filePath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_save_as' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:217`（相对源码根目录）。

<a id="cmd-files-file_write"></a>

#### `plugin:files|file_write` — 写入游戏内文件

bytes 是 0–255 的字节数组。createOnly=true 表示只能新建；false 允许按后台规则覆盖已有内容。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `path` | 是 | &str | 游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。 |
| `bytes` | 是 | Vec<u8> | 字节数据或容量，具体取决于此接口的 Rust 类型；整数是字节数，数组是 0–255 文件字节。 |
| `createOnly` | 是 | bool | true 只允许创建新文件，已有文件会报错；false 允许按原接口覆盖。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "path": "config/<目标文件>",
  "bytes": "<从实际文件读取的0到255字节数组>",
  "createOnly": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|file_write' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:167`（相对源码根目录）。

<a id="cmd-files-files_release_external"></a>

#### `plugin:files|files_release_external` — 释放或取消外部保存会话

结束 saveId 对应的外部保存会话，并向运行中的下载发出取消信号；不删除已经完成的用户文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `saveId` | 是 | String | 原生保存对话框返回的保存会话 ID；不是文件路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`std::result::Result<(), ExternalSaveError>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "saveId": "<saveId>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|files_release_external' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:448`（相对源码根目录）。

<a id="cmd-files-files_save_external"></a>

#### `plugin:files|files_save_external` — 下载到已选外部文件

使用 files_select_external 的 saveId，将 request.url 下载到所选位置，支持进度通道；不是安装到某个游戏版本。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `saveId` | 是 | String | 原生保存对话框返回的保存会话 ID；不是文件路径。 |
| `request` | 是 | ExternalSaveRequest | 外部下载请求：url 文件 URL，headers 请求头对象；保存目标已由 saveId 对应的原生对话框决定。 |
| `onProgress` | 是 | tauri::ipc::Channel<ExternalSaveProgress> | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`std::result::Result<String, ExternalSaveError>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "saveId": "<saveId>",
  "request": {
    "url": "https://example.com/example.zip",
    "headers": {}
  },
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|files_save_external' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:369`（相对源码根目录）。

<a id="cmd-files-files_select_external"></a>

#### `plugin:files|files_select_external` — 选择外部下载的保存位置

弹出保存对话框并返回 saveId 会话标识；取消时返回 null。此时尚未开始下载。

**影响与可用性：** 原生对话框：需要桌面可交互；未完成选择前调用可能一直等待。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `filename` | 是 | String | 文件名称，不是可越出目标目录的任意绝对路径。 |
| `filter` | 否 | Option<ExternalSaveFilter> | 保存对话框过滤对象，例如 {"name":"ZIP 文件","extensions":["zip"]}；null 不指定过滤器。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 保存会话 ID 字符串；用户取消时为 null。后续 files_save_external 使用这个 ID，不传输出路径。 Rust 返回类型：`std::result::Result<Option<String>, ExternalSaveError>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "filename": "example.zip",
  "filter": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:files|files_select_external' '@args.json'
```

源码位置：`apps/app/src/api/files.rs:329`（相对源码根目录）。

<a id="api-friends"></a>

### 上游好友兼容接口

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`add_friend`](#cmd-friends-add_friend) | 添加上游平台好友 | 上游服务写入 |
| [`friend_statuses`](#cmd-friends-friend_statuses) | 查询平台好友状态 | 上游服务查询 |
| [`friends`](#cmd-friends-friends) | 列出上游平台好友 | 上游服务查询 |
| [`remove_friend`](#cmd-friends-remove_friend) | 移除上游平台好友 | 上游服务写入 |

<a id="cmd-friends-add_friend"></a>

#### `plugin:friends|add_friend` — 添加上游平台好友

向 Modrinth 社交服务发起好友操作，不是 Minecraft 服务器白名单；需要平台身份，当前无平台登录入口，通常无法使用。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`crate::api::Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:friends|add_friend' '@args.json'
```

源码位置：`apps/app/src/api/friends.rs:26`（相对源码根目录）。

<a id="cmd-friends-friend_statuses"></a>

#### `plugin:friends|friend_statuses` — 查询平台好友状态

读取上游好友的在线/游玩状态；依赖上游身份与服务，不是本机游戏进程检测。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 UserStatus 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`crate::api::Result<Vec<UserStatus>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:friends|friend_statuses' '{}'
```

源码位置：`apps/app/src/api/friends.rs:21`（相对源码根目录）。

<a id="cmd-friends-friends"></a>

#### `plugin:friends|friends` — 列出上游平台好友

返回 Modrinth 平台好友关系；不能用它查询微软 Xbox 好友。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 UserFriend 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`crate::api::Result<Vec<UserFriend>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:friends|friends' '{}'
```

源码位置：`apps/app/src/api/friends.rs:16`（相对源码根目录）。

<a id="cmd-friends-remove_friend"></a>

#### `plugin:friends|remove_friend` — 移除上游平台好友

修改平台好友关系；当前 GeCL 不提供平台登录，不能把它视为可直接使用的本机功能。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`crate::api::Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:friends|remove_friend' '@args.json'
```

源码位置：`apps/app/src/api/friends.rs:31`（相对源码根目录）。

<a id="api-gecl"></a>

### GeCL 专用功能与 CurseForge

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`appearance_discard_background`](#cmd-gecl-appearance_discard_background) | 清理未使用的背景副本 | 删除／清理 |
| [`appearance_get`](#cmd-gecl-appearance_get) | 读取主题色与背景设置 | 本地查询 |
| [`appearance_import_background`](#cmd-gecl-appearance_import_background) | 导入背景照片 | 写入操作 |
| [`appearance_set`](#cmd-gecl-appearance_set) | 保存外观配置 | 写入操作 |
| [`cache_artwork`](#cmd-gecl-cache_artwork) | 缓存一张自定义图标 | 写入操作 |
| [`cancel_import`](#cmd-gecl-cancel_import) | 取消本地游戏文件夹导入 | 写入操作 |
| [`cancel_world_backup`](#cmd-gecl-cancel_world_backup) | 取消存档备份或恢复任务 | 写入操作 |
| [`choose_directory`](#cmd-gecl-choose_directory) | 设置首次游戏数据目录 | 写入操作 |
| [`curseforge_cancel`](#cmd-gecl-curseforge_cancel) | 取消 CurseForge 安装任务 | 写入操作 |
| [`curseforge_changelog`](#cmd-gecl-curseforge_changelog) | 读取 CF 文件更新日志 | 查询／缓存 |
| [`curseforge_description`](#cmd-gecl-curseforge_description) | 读取 CF 项目介绍 | 查询／缓存 |
| [`curseforge_file`](#cmd-gecl-curseforge_file) | 读取一个 CF 文件版本 | 查询／缓存 |
| [`curseforge_files`](#cmd-gecl-curseforge_files) | 列出 CF 项目兼容文件 | 查询／缓存 |
| [`curseforge_install`](#cmd-gecl-curseforge_install) | 安装 CF 模组及必需依赖 | 写入操作 |
| [`curseforge_installed`](#cmd-gecl-curseforge_installed) | 查询已安装 CF 项目索引 | 本地查询 |
| [`curseforge_project`](#cmd-gecl-curseforge_project) | 读取 CF 项目资料 | 查询／缓存 |
| [`curseforge_search`](#cmd-gecl-curseforge_search) | 搜索 CF 模组 | 查询／缓存 |
| [`dismiss_launch`](#cmd-gecl-dismiss_launch) | 关闭一条已结束的启动提示 | 写入操作 |
| [`finish_setup`](#cmd-gecl-finish_setup) | 完成首次设置标记 | 写入操作 |
| [`import_folder`](#cmd-gecl-import_folder) | 导入已有游戏文件夹 | 写入操作 |
| [`launch_progress`](#cmd-gecl-launch_progress) | 读取游戏启动进度 | 本地查询 |
| [`pack_artwork`](#cmd-gecl-pack_artwork) | 预览本地整合包名称与图标 | 写入操作 |
| [`preview_folder`](#cmd-gecl-preview_folder) | 扫描已有游戏版本 | 本地查询 |
| [`setup_status`](#cmd-gecl-setup_status) | 查询首次设置缺项 | 本地查询 |
| [`world_backup`](#cmd-gecl-world_backup) | 备份或恢复一个世界 | 写入操作 |
| [`world_backup_directory`](#cmd-gecl-world_backup_directory) | 读取上次备份位置 | 本地查询 |

<a id="cmd-gecl-appearance_discard_background"></a>

#### `plugin:gecl|appearance_discard_background` — 清理未使用的背景副本

删除应用缓存的背景图片副本；后台会保留仍被当前外观设置引用的图片，不删除原照片。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 由外观接口返回的背景副本 ID；不是外部图片绝对路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|appearance_discard_background' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:125`（相对源码根目录）。

<a id="cmd-gecl-appearance_get"></a>

#### `plugin:gecl|appearance_get` — 读取主题色与背景设置

返回 preferences，以及已保存背景副本的 id/path；不打开设置窗口。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 当前外观偏好、背景副本与窗口配置等信息；保存后可再次查询核对。 Rust 返回类型：`Result<theseus::gecl::appearance::Appearance>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|appearance_get' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:95`（相对源码根目录）。

<a id="cmd-gecl-appearance_import_background"></a>

#### `plugin:gecl|appearance_import_background` — 导入背景照片

读取 source 图片，规范化并保存应用副本，返回 id/path；原图不变，之后用 appearance_set 启用。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `source` | 是 | PathBuf | 本机源文件绝对路径；文件重命名接口的 source 则是游戏内相对路径。 |

**返回：** 导入的背景副本信息，使用其 ID 写入 Preferences；导入本身不等于应用背景。 Rust 返回类型：`Result<theseus::gecl::appearance::Background>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "source": "D:/Example/source.png"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|appearance_import_background' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:115`（相对源码根目录）。

<a id="cmd-gecl-appearance_set"></a>

#### `plugin:gecl|appearance_set` — 保存外观配置

保存主题色、背景开关和效果参数；可同时传 window 保存明暗模式、阴影和系统标题栏选项。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `preferences` | 是 | theseus::gecl::appearance::Preferences | 外观偏好 Preferences（camelCase）：accentColor、backgroundId、backgroundEnabled、blur、overlay、surfaceOpacity、position；范围与示例见复杂参数配方。 |
| `window` | 否 | Option<theseus::gecl::appearance::WindowAppearance> | 窗口外观对象（主题、渲染与装饰配置）；不是运行时注入的 Window 参数。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 当前外观偏好、背景副本与窗口配置等信息；保存后可再次查询核对。 Rust 返回类型：`Result<theseus::gecl::appearance::Appearance>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "preferences": {
    "accentColor": "#176fb0",
    "backgroundId": null,
    "backgroundEnabled": true,
    "blur": 8,
    "overlay": 18,
    "surfaceOpacity": 88,
    "position": "center"
  },
  "window": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|appearance_set' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:104`（相对源码根目录）。

<a id="cmd-gecl-cache_artwork"></a>

#### `plugin:gecl|cache_artwork` — 缓存一张自定义图标

将 source 图片校验并规范化为应用 PNG 缓存，返回缓存路径；不会自动替换某个游戏图标。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `source` | 是 | PathBuf | 本机源文件绝对路径；文件重命名接口的 source 则是游戏内相对路径。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<PathBuf>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "source": "D:/Example/source.png"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|cache_artwork' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:54`（相对源码根目录）。

<a id="cmd-gecl-cancel_import"></a>

#### `plugin:gecl|cancel_import` — 取消本地游戏文件夹导入

按 import_folder 的 jobId 发出取消信号；要使用与导入时相同的编号。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | String | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<本次操作自定任务ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|cancel_import' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:215`（相对源码根目录）。

<a id="cmd-gecl-cancel_world_backup"></a>

#### `plugin:gecl|cancel_world_backup` — 取消存档备份或恢复任务

按 world_backup 的 jobId 取消准备/复制过程；最终目录提交阶段不做半途取消。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | String | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<本次操作自定任务ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|cancel_world_backup' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:81`（相对源码根目录）。

<a id="cmd-gecl-choose_directory"></a>

#### `plugin:gecl|choose_directory` — 设置首次游戏数据目录

设置或迁移应用的游戏数据目录，可能涉及文件操作；这不是仅改变下载对话框的默认文件夹。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Minecraft"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|choose_directory' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:147`（相对源码根目录）。

<a id="cmd-gecl-curseforge_cancel"></a>

#### `plugin:gecl|curseforge_cancel` — 取消 CurseForge 安装任务

按 curseforge_install 的 UUID jobId 请求取消；不会取消 Modrinth 的安装任务。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | uuid::Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_cancel' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:286`（相对源码根目录）。

<a id="cmd-gecl-curseforge_changelog"></a>

#### `plugin:gecl|curseforge_changelog` — 读取 CF 文件更新日志

id 为 CF 项目数字 ID，fileId 为该项目的文件数字 ID；返回更新日志文本。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | u64 | CurseForge 项目的数字 ID，例如 238222；不是文件 ID。 |
| `fileId` | 是 | u64 | CurseForge 文件数字 ID；自动安装时 null 表示按游戏兼容条件选择。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": 238222,
  "fileId": "<实际CurseForge文件数字ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_changelog' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:253`（相对源码根目录）。

<a id="cmd-gecl-curseforge_description"></a>

#### `plugin:gecl|curseforge_description` — 读取 CF 项目介绍

按 CF 项目数字 ID 获取原作者完整 HTML 介绍；首选现有官方或镜像接口，失败或为空时通过 CFWidget 的公开项目数据补取，并校验项目 ID 与游戏类型。不下载模组。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | u64 | CurseForge 项目的数字 ID，例如 238222；不是文件 ID。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": 238222
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_description' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:242`（相对源码根目录）。

<a id="cmd-gecl-curseforge_file"></a>

#### `plugin:gecl|curseforge_file` — 读取一个 CF 文件版本

返回指定项目与文件的版本资料、下载信息及依赖信息，不开始安装。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | u64 | CurseForge 项目的数字 ID，例如 238222；不是文件 ID。 |
| `fileId` | 是 | u64 | CurseForge 文件数字 ID；自动安装时 null 表示按游戏兼容条件选择。 |

**返回：** 序列化的 theseus::curseforge::File 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::curseforge::File>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": 238222,
  "fileId": "<实际CurseForge文件数字ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_file' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:246`（相对源码根目录）。

<a id="cmd-gecl-curseforge_files"></a>

#### `plugin:gecl|curseforge_files` — 列出 CF 项目兼容文件

按 request 中的 Minecraft 版本、加载器、分页条件查询该项目可用文件。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | u64 | CurseForge 项目的数字 ID，例如 238222；不是文件 ID。 |
| `request` | 是 | theseus::curseforge::Search | CurseForge 查询对象（camelCase）：query 搜索词、gameVersion 游戏版本、loader 加载器、index 起始下标、pageSize 每页数量，另有 sort、category、slug。 |

**返回：** 序列化的 theseus::curseforge::Page<theseus::curseforge::File> 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::curseforge::Page<theseus::curseforge::File>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": 238222,
  "request": {
    "query": "AppleSkin",
    "gameVersion": "1.20.1",
    "loader": "forge",
    "index": 0,
    "pageSize": 20
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_files' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:235`（相对源码根目录）。

<a id="cmd-gecl-curseforge_install"></a>

#### `plugin:gecl|curseforge_install` — 安装 CF 模组及必需依赖

向 instanceId 指定游戏安装 projectId；省略 fileId 时按目标游戏兼容条件选择文件。支持 jobId 取消和进度通道。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | uuid::Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectId` | 是 | u64 | CurseForge 项目数字 ID，可从 curseforge_search 或 curseforge_project 取得。 |
| `fileId` | 否 | Option<u64> | CurseForge 文件数字 ID；自动安装时 null 表示按游戏兼容条件选择。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `onProgress` | 是 | tauri::ipc::Channel<theseus::curseforge::InstallProgress> | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 已安装文件的相对路径数组；这是直接等待安装的调用，不返回 install_job 的任务快照。进度在 events 中。 Rust 返回类型：`Result<Vec<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>",
  "instanceId": "<游戏ID>",
  "projectId": 238222,
  "fileId": null,
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_install' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:257`（相对源码根目录）。

<a id="cmd-gecl-curseforge_installed"></a>

#### `plugin:gecl|curseforge_installed` — 查询已安装 CF 项目索引

返回游戏 ID 到 CF 项目数字 ID 列表的映射，供判断某版本是否装过某项目。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 对象映射：键为内部游戏 ID，值为该游戏已安装的 CurseForge 项目数字 ID 数组。 Rust 返回类型：`Result<std::collections::HashMap<String, Vec<u64>>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_installed' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:291`（相对源码根目录）。

<a id="cmd-gecl-curseforge_project"></a>

#### `plugin:gecl|curseforge_project` — 读取 CF 项目资料

获取项目名称、作者、图标、下载量与链接；不会下载模组文件。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | u64 | CurseForge 项目的数字 ID，例如 238222；不是文件 ID。 |

**返回：** Modrinth 项目资料，包括名称、简介、作者团队、图标、下载量、项目类型和版本关联等；具体字段按平台对象定义读取。 Rust 返回类型：`Result<theseus::curseforge::Project>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": 238222
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_project' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:231`（相对源码根目录）。

<a id="cmd-gecl-curseforge_search"></a>

#### `plugin:gecl|curseforge_search` — 搜索 CF 模组

request 支持关键词、MC 版本、加载器、分类和分页；返回项目列表及分页信息，数字 ID 与 Modrinth ID 不通用。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `request` | 是 | theseus::curseforge::Search | CurseForge 查询对象（camelCase）：query 搜索词、gameVersion 游戏版本、loader 加载器、index 起始下标、pageSize 每页数量，另有 sort、category、slug。 |

**返回：** 序列化的 theseus::curseforge::Page<theseus::curseforge::Project> 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::curseforge::Page<theseus::curseforge::Project>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "request": {
    "query": "AppleSkin",
    "gameVersion": "1.20.1",
    "loader": "forge",
    "index": 0,
    "pageSize": 20
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|curseforge_search' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:225`（相对源码根目录）。

<a id="cmd-gecl-dismiss_launch"></a>

#### `plugin:gecl|dismiss_launch` — 关闭一条已结束的启动提示

只隐藏对应启动进度记录，不会终止 Minecraft 进程。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | uuid::Uuid | launch_progress 返回的启动进度记录 UUID；不会因此结束游戏。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|dismiss_launch' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:135`（相对源码根目录）。

<a id="cmd-gecl-finish_setup"></a>

#### `plugin:gecl|finish_setup` — 完成首次设置标记

确认首次目录/账户设置流程已完成；不自动添加 Player，也不负责安装 Minecraft。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|finish_setup' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:155`（相对源码根目录）。

<a id="cmd-gecl-import_folder"></a>

#### `plugin:gecl|import_folder` — 导入已有游戏文件夹

source 是原游戏来源，versionFile 是 preview_folder 返回的版本描述路径；复制、识别并创建本机游戏，返回新游戏 ID。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | String | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |
| `source` | 是 | PathBuf | 本机源文件绝对路径；文件重命名接口的 source 则是游戏内相对路径。 |
| `versionFile` | 是 | PathBuf | preview_folder 返回的具体版本 JSON 的绝对路径；不要自己猜文件名。 |
| `onProgress` | 是 | tauri::ipc::Channel<(u64, u64)> | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 导入成功后的内部游戏 ID 字符串，可用来读取内容、设置图标或启动游戏。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<本次操作自定任务ID>",
  "source": "D:/Example/source.png",
  "versionFile": "<预览返回的版本JSON路径>",
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|import_folder' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:167`（相对源码根目录）。

<a id="cmd-gecl-launch_progress"></a>

#### `plugin:gecl|launch_progress` — 读取游戏启动进度

返回各游戏的启动阶段、进度和错误状态；它不是文件下载进度表。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::launch_progress::LaunchProgress 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Vec<theseus::launch_progress::LaunchProgress>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|launch_progress' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:130`（相对源码根目录）。

<a id="cmd-gecl-pack_artwork"></a>

#### `plugin:gecl|pack_artwork` — 预览本地整合包名称与图标

只检查 source 的 .mrpack 清单与图标并缓存图片；不会安装包内 Minecraft 或 Mod。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `source` | 是 | PathBuf | 本机源文件绝对路径；文件重命名接口的 source 则是游戏内相对路径。 |

**返回：** 本地整合包的封面预览信息和应用缓存图片位置；不是安装完成结果。 Rust 返回类型：`Result<theseus::gecl::pack_artwork::PackArtwork>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "source": "D:/Example/source.png"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|pack_artwork' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:46`（相对源码根目录）。

<a id="cmd-gecl-preview_folder"></a>

#### `plugin:gecl|preview_folder` — 扫描已有游戏版本

识别 path 中可导入的版本，返回候选和 version_file；只做检测，不创建新游戏。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 已安装游戏目录或 .minecraft 根目录的绝对路径；预览返回可导入的版本 JSON。 |

**返回：** 数组；每项为：识别到的游戏版本信息，包含后续导入所需的源目录及版本 JSON 路径。 Rust 返回类型：`Result<Vec<theseus::gecl::LocalGame>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Minecraft"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|preview_folder' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:160`（相对源码根目录）。

<a id="cmd-gecl-setup_status"></a>

#### `plugin:gecl|setup_status` — 查询首次设置缺项

返回 needsDirectory、needsAccount、dataDirectory，供判断是否需要选择目录和添加账户。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 首次设置状态和默认路径信息；finish_setup 仍需满足目录和账户条件。 Rust 返回类型：`Result<theseus::gecl::SetupStatus>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|setup_status' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:140`（相对源码根目录）。

<a id="cmd-gecl-world_backup"></a>

#### `plugin:gecl|world_backup` — 备份或恢复一个世界

restore=false 将世界写成 archive ZIP；true 从 ZIP 恢复并覆盖现存目标世界，先保留旧世界副本。世界需未被游戏占用。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | String | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `world` | 是 | String | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |
| `archive` | 是 | PathBuf | 备份 ZIP 的本机绝对路径；备份时为输出文件，恢复时为已有输入文件。 |
| `restore` | 是 | bool | false 创建备份，true 从 archive 恢复覆盖；恢复流程先保留当前世界副本。 |
| `onProgress` | 是 | tauri::ipc::Channel<theseus::gecl::world_backups::Progress> | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 备份或恢复结果，包含输出位置与恢复前副本等信息；按实际返回路径检查文件。 Rust 返回类型：`Result<theseus::gecl::world_backups::BackupResult>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<本次操作自定任务ID>",
  "instanceId": "<游戏ID>",
  "world": "<世界文件夹名>",
  "archive": "D:/Backups/world.zip",
  "restore": false,
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|world_backup' '@args.json'
```

源码位置：`apps/app/src/api/gecl.rs:67`（相对源码根目录）。

<a id="cmd-gecl-world_backup_directory"></a>

#### `plugin:gecl|world_backup_directory` — 读取上次备份位置

返回记住的世界备份文件夹；尚未选择过时为 null。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:gecl|world_backup_directory' '{}'
```

源码位置：`apps/app/src/api/gecl.rs:64`（相对源码根目录）。

<a id="api-import"></a>

### 其他启动器目录识别

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`get_default_launcher_path`](#cmd-import-get_default_launcher_path) | 查找其他启动器默认目录 | 本地查询 |
| [`get_importable_instances`](#cmd-import-get_importable_instances) | 扫描其他启动器的游戏列表 | 本地查询 |
| [`is_valid_importable_instance`](#cmd-import-is_valid_importable_instance) | 检查一个外部游戏是否可导入 | 本地查询 |

<a id="cmd-import-get_default_launcher_path"></a>

#### `plugin:import|get_default_launcher_path` — 查找其他启动器默认目录

按 launcherType 检测已知启动器的常用数据目录，未找到时返回 null；不扫描所有硬盘。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `launcherType` | 是 | ImportLauncherType；可选值 MultiMC, PrismLauncher, ATLauncher, GDLauncher, Curseforge, Unknown | 其他启动器类型枚举，区分大小写；PCL 本地目录请用 gecl 文件夹导入。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<PathBuf>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "launcherType": "MultiMC"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:import|get_default_launcher_path' '@args.json'
```

源码位置：`apps/app/src/api/import.rs:45`（相对源码根目录）。

<a id="cmd-import-get_importable_instances"></a>

#### `plugin:import|get_importable_instances` — 扫描其他启动器的游戏列表

在 basePath 指定的启动器根目录中列出可导入的游戏名称。PCL 目录优先使用 gecl 的通用文件夹扫描。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `launcherType` | 是 | ImportLauncherType；可选值 MultiMC, PrismLauncher, ATLauncher, GDLauncher, Curseforge, Unknown | 其他启动器类型枚举，区分大小写；PCL 本地目录请用 gecl 文件夹导入。 |
| `basePath` | 是 | PathBuf | 待导入启动器的根目录，必须符合 launcherType 对应目录结构。 |

**返回：** 数组；每项为：字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<Vec<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "launcherType": "MultiMC",
  "basePath": "D:/Launcher"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:import|get_importable_instances' '@args.json'
```

源码位置：`apps/app/src/api/import.rs:22`（相对源码根目录）。

<a id="cmd-import-is_valid_importable_instance"></a>

#### `plugin:import|is_valid_importable_instance` — 检查一个外部游戏是否可导入

检查 instanceFolder 是否符合指定启动器的数据结构，只返回 true/false，不复制文件。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceFolder` | 是 | PathBuf | 待验证游戏目录的绝对路径。 |
| `launcherType` | 是 | ImportLauncherType；可选值 MultiMC, PrismLauncher, ATLauncher, GDLauncher, Curseforge, Unknown | 其他启动器类型枚举，区分大小写；PCL 本地目录请用 gecl 文件夹导入。 |

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceFolder": "<查询到的版本文件夹>",
  "launcherType": "MultiMC"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:import|is_valid_importable_instance' '@args.json'
```

源码位置：`apps/app/src/api/import.rs:32`（相对源码根目录）。

<a id="api-install"></a>

### 安装任务与整合包

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`install_accept_shared_instance_invite`](#cmd-install-install_accept_shared_instance_invite) | 接受共享游戏邀请 | 上游服务写入 |
| [`install_bulk_update_content`](#cmd-install-install_bulk_update_content) | 批量更新已装内容 | 写入操作 |
| [`install_create_instance`](#cmd-install-install_create_instance) | 创建游戏版本 | 写入操作 |
| [`install_create_modpack_instance`](#cmd-install-install_create_modpack_instance) | 安装整合包为新游戏 | 写入操作 |
| [`install_duplicate_instance`](#cmd-install-install_duplicate_instance) | 复制游戏版本 | 写入操作 |
| [`install_existing_instance`](#cmd-install-install_existing_instance) | 补全或修复游戏 | 写入操作 |
| [`install_get_modpack_preview`](#cmd-install-install_get_modpack_preview) | 预览整合包 | 查询／缓存 |
| [`install_get_shared_instance_preview`](#cmd-install-install_get_shared_instance_preview) | 预览共享游戏安装 | 上游服务查询 |
| [`install_get_shared_instance_update_preview`](#cmd-install-install_get_shared_instance_update_preview) | 预览共享游戏更新 | 上游服务查询 |
| [`install_import_instance`](#cmd-install-install_import_instance) | 从其他启动器导入 | 写入操作 |
| [`install_job_cancel`](#cmd-install-install_job_cancel) | 取消安装任务 | 写入操作 |
| [`install_job_dismiss`](#cmd-install-install_job_dismiss) | 移除任务显示记录 | 写入操作 |
| [`install_job_get`](#cmd-install-install_job_get) | 查询一个安装任务 | 本地查询 |
| [`install_job_list`](#cmd-install-install_job_list) | 列出安装任务 | 本地查询 |
| [`install_job_pause`](#cmd-install-install_job_pause) | 暂停安装任务 | 写入操作 |
| [`install_job_resume`](#cmd-install-install_job_resume) | 继续安装任务 | 写入操作 |
| [`install_job_retry`](#cmd-install-install_job_retry) | 重试失败安装 | 写入操作 |
| [`install_job_support_details`](#cmd-install-install_job_support_details) | 生成安装报告 | 本地查询 |
| [`install_pack_to_existing_instance`](#cmd-install-install_pack_to_existing_instance) | 将整合包装入已有游戏 | 写入操作 |
| [`install_shared_instance`](#cmd-install-install_shared_instance) | 安装共享游戏 | 上游服务写入 |
| [`install_update_shared_instance`](#cmd-install-install_update_shared_instance) | 更新共享游戏 | 上游服务写入 |

<a id="cmd-install-install_accept_shared_instance_invite"></a>

#### `plugin:install|install_accept_shared_instance_invite` — 接受共享游戏邀请

向上游共享服务接受邀请并取得待安装预览；不是本地文件夹导入。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `inviteId` | 是 | String | 上游共享邀请的 ID，从邀请查询结果取得。 |

**返回：** 序列化的 SharedInstanceInviteInstallPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<SharedInstanceInviteInstallPreview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "inviteId": "<inviteId>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_accept_shared_instance_invite' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:131`（相对源码根目录）。

<a id="cmd-install-install_bulk_update_content"></a>

#### `plugin:install|install_bulk_update_content` — 批量更新已装内容

为指定游戏提交所选模组或资源的更新任务，任务中执行下载与替换；返回任务快照。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `updates` | 是 | Vec<theseus::install::ContentUpdateSelection> | 内容更新数组，每项 {"project_path":"mods/a.jar","version_id":"Modrinth版本ID"}；字段为 snake_case。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "updates": [
    {
      "project_path": "mods/<原文件名.jar>",
      "version_id": "<Modrinth版本ID>"
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_bulk_update_content' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:180`（相对源码根目录）。

<a id="cmd-install-install_create_instance"></a>

#### `plugin:install|install_create_instance` — 创建游戏版本

按名称、Minecraft 版本、加载器及加载器版本创建游戏并下载所需组件；返回异步安装任务。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `request` | 是 | InstallCreateInstanceRequest | 新建游戏对象：name 名称、gameVersion Minecraft 版本、loader 加载器、loaderVersion 加载器版本（如 stable）、iconPath 可选本地图标。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "request": {
    "name": "测试游戏",
    "gameVersion": "1.20.1",
    "loader": "fabric",
    "loaderVersion": "stable",
    "iconPath": null
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_create_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:88`（相对源码根目录）。

<a id="cmd-install-install_create_modpack_instance"></a>

#### `plugin:install|install_create_modpack_instance` — 安装整合包为新游戏

读取本地 mrpack 或指定在线整合包版本，创建新的游戏目录并安排下载、解包和安装。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `location` | 是 | CreatePackLocation | 整合包来源对象；本地 fromFile 与在线 fromVersionId 的字段示例见下方。 |
| `postInstallEdit` | 否 | Option<InstallPostInstallEditRequest> | 整合包安装后调整对象（camelCase）；可选 name、iconPath，省略表示不覆盖原信息。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "location": {
    "type": "fromFile",
    "path": "D:/Packs/example.mrpack"
  },
  "postInstallEdit": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_create_modpack_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:107`（相对源码根目录）。

<a id="cmd-install-install_duplicate_instance"></a>

#### `plugin:install|install_duplicate_instance` — 复制游戏版本

复制已有游戏的文件与配置为新游戏，返回复制任务；不只是建立快捷方式。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `sourceInstanceId` | 是 | String | 复制来源的内部游戏 ID；不会按名称模糊查找。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "sourceInstanceId": "<来源游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_duplicate_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:202`（相对源码根目录）。

<a id="cmd-install-install_existing_instance"></a>

#### `plugin:install|install_existing_instance` — 补全或修复游戏

为已有游戏检查并补全运行文件；force 为 true 会要求强制修复，可能重新下载文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `force` | 是 | bool | 是否强制执行完整修复；true 可能重新校验或下载运行文件。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "force": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_existing_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:209`（相对源码根目录）。

<a id="cmd-install-install_get_modpack_preview"></a>

#### `plugin:install|install_get_modpack_preview` — 预览整合包

读取整合包的名称、游戏版本、加载器等安装信息；在线来源可能下载包文件和缓存，但不创建游戏。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `location` | 是 | CreatePackLocation | 整合包来源对象；本地 fromFile 与在线 fromVersionId 的字段示例见下方。 |

**返回：** 序列化的 InstallModpackPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<InstallModpackPreview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "location": {
    "type": "fromFile",
    "path": "D:/Packs/example.mrpack"
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_get_modpack_preview' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:81`（相对源码根目录）。

<a id="cmd-install-install_get_shared_instance_preview"></a>

#### `plugin:install|install_get_shared_instance_preview` — 预览共享游戏安装

向共享服务查询指定共享游戏的安装预览；依赖上游共享服务和权限。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `sharedInstanceId` | 是 | String | 上游在线共享游戏的内部 ID；不是本地游戏 ID。 |
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |

**返回：** 序列化的 SharedInstanceInstallPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<SharedInstanceInstallPreview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "sharedInstanceId": "<sharedInstanceId>",
  "name": "测试名称"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_get_shared_instance_preview' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:119`（相对源码根目录）。

<a id="cmd-install-install_get_shared_instance_update_preview"></a>

#### `plugin:install|install_get_shared_instance_update_preview` — 预览共享游戏更新

比较本地共享游戏与服务端版本，返回更新预览；没有更新时可为空。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 SharedInstanceUpdatePreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<SharedInstanceUpdatePreview>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_get_shared_instance_update_preview' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:143`（相对源码根目录）。

<a id="cmd-install-install_import_instance"></a>

#### `plugin:install|install_import_instance` — 从其他启动器导入

按指定启动器的目录结构导入一个游戏；PCL 或任意 .minecraft 目录优先使用 gecl 的文件夹预览及导入接口。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `launcherType` | 是 | ImportLauncherType；可选值 MultiMC, PrismLauncher, ATLauncher, GDLauncher, Curseforge, Unknown | 其他启动器类型枚举，区分大小写；PCL 本地目录请用 gecl 文件夹导入。 |
| `basePath` | 是 | PathBuf | 待导入启动器的根目录，必须符合 launcherType 对应目录结构。 |
| `instanceFolder` | 是 | String | get_importable_instances 返回的版本文件夹名称，配合 basePath 使用。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "launcherType": "MultiMC",
  "basePath": "D:/Launcher",
  "instanceFolder": "<查询到的版本文件夹>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_import_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:188`（相对源码根目录）。

<a id="cmd-install-install_job_cancel"></a>

#### `plugin:install|install_job_cancel` — 取消安装任务

请求取消任务；已修改文件的任务按原安装流程执行回滚，不代表调用后立即停止。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_cancel' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:258`（相对源码根目录）。

<a id="cmd-install-install_job_dismiss"></a>

#### `plugin:install|install_job_dismiss` — 移除任务显示记录

移除允许被关闭的安装任务记录；不删除成功安装的游戏，也不等于取消运行中任务。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_dismiss' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:263`（相对源码根目录）。

<a id="cmd-install-install_job_get"></a>

#### `plugin:install|install_job_get` — 查询一个安装任务

按任务 UUID 读取当前状态、阶段、进度、错误与关联游戏 ID。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_get' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:238`（相对源码根目录）。

<a id="cmd-install-install_job_list"></a>

#### `plugin:install|install_job_list` — 列出安装任务

读取安装任务快照；includeFinished 决定是否包含已完成、失败或取消的记录。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `includeFinished` | 是 | bool | 是否包含已经结束的安装任务；应显式传 true 或 false。 |

**返回：** 数组；每项为：安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<Vec<InstallJobSnapshot>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "includeFinished": true
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_list' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:231`（相对源码根目录）。

<a id="cmd-install-install_job_pause"></a>

#### `plugin:install|install_job_pause` — 暂停安装任务

请求在安装流程支持的检查点暂停；能否暂停取决于任务当前阶段。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_pause' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:248`（相对源码根目录）。

<a id="cmd-install-install_job_resume"></a>

#### `plugin:install|install_job_resume` — 继续安装任务

恢复已经暂停且可继续的安装任务。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_resume' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:253`（相对源码根目录）。

<a id="cmd-install-install_job_retry"></a>

#### `plugin:install|install_job_retry` — 重试失败安装

按原任务信息重试允许重试的安装任务，可能再次下载缺失文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_retry' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:243`（相对源码根目录）。

<a id="cmd-install-install_job_support_details"></a>

#### `plugin:install|install_job_support_details` — 生成安装报告

返回任务的环境、时间线、错误上下文和日志摘录文本；不会主动上传报告。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `jobId` | 是 | Uuid | 任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "jobId": "<任务UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_job_support_details' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:268`（相对源码根目录）。

<a id="cmd-install-install_pack_to_existing_instance"></a>

#### `plugin:install|install_pack_to_existing_instance` — 将整合包装入已有游戏

在指定游戏上应用整合包内容及安装配置；会改动目标游戏，调用前应备份。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `location` | 是 | CreatePackLocation | 整合包来源对象；本地 fromFile 与在线 fromVersionId 的字段示例见下方。 |
| `postInstallEdit` | 否 | Option<InstallPostInstallEditRequest> | 整合包安装后调整对象（camelCase）；可选 name、iconPath，省略表示不覆盖原信息。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "location": {
    "type": "fromFile",
    "path": "D:/Packs/example.mrpack"
  },
  "postInstallEdit": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_pack_to_existing_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:217`（相对源码根目录）。

<a id="cmd-install-install_shared_instance"></a>

#### `plugin:install|install_shared_instance` — 安装共享游戏

向上游服务取得共享游戏内容并创建安装任务；依赖服务端授权，不是普通 mrpack 安装。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `sharedInstanceId` | 是 | String | 上游在线共享游戏的内部 ID；不是本地游戏 ID。 |
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |
| `managerId` | 否 | Option<String> | 上游共享管理者 ID；普通本地导入不使用。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `serverManagerName` | 否 | Option<String> | 上游共享服务器管理者名称；省略不指定。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `serverManagerIconUrl` | 否 | Option<String> | 上游共享服务器管理者的图标 URL；省略不指定。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `instanceIconUrl` | 否 | Option<String> | 共享游戏的封面 URL；省略时不提供替代图片。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "sharedInstanceId": "<sharedInstanceId>",
  "name": "测试名称",
  "managerId": null,
  "serverManagerName": null,
  "serverManagerIconUrl": null,
  "instanceIconUrl": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_shared_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:153`（相对源码根目录）。

<a id="cmd-install-install_update_shared_instance"></a>

#### `plugin:install|install_update_shared_instance` — 更新共享游戏

根据上游共享配置更新已关联的游戏，可能下载和替换本地内容。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:install|install_update_shared_instance' '@args.json'
```

源码位置：`apps/app/src/api/install.rs:173`（相对源码根目录）。

<a id="api-instance"></a>

### 游戏、内容、分组、截图与同步

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`instance_add_project_from_path`](#cmd-instance-instance_add_project_from_path) | 安装本地资源文件 | 写入操作 |
| [`instance_add_project_from_version`](#cmd-instance-instance_add_project_from_version) | 安装指定 Modrinth 文件 | 写入操作 |
| [`instance_cache_generated_icon`](#cmd-instance-instance_cache_generated_icon) | 生成并缓存游戏图标 | 写入操作 |
| [`instance_check_installed`](#cmd-instance-instance_check_installed) | 检查项目是否已安装 | 本地查询 |
| [`instance_content`](#cmd-instance-instance_content) | 读取游戏内容列表 | 查询／缓存 |
| [`instance_create_group`](#cmd-instance-instance_create_group) | 创建游戏分组 | 写入操作 |
| [`instance_create_screenshot_group`](#cmd-instance-instance_create_screenshot_group) | 创建截图分组 | 写入操作 |
| [`instance_delete_group`](#cmd-instance-instance_delete_group) | 删除游戏分组 | 删除／清理 |
| [`instance_delete_screenshot_group`](#cmd-instance-instance_delete_screenshot_group) | 删除截图分组 | 删除／清理 |
| [`instance_delete_screenshots`](#cmd-instance-instance_delete_screenshots) | 删除截图文件 | 删除／清理 |
| [`instance_desync_pack`](#cmd-instance-instance_desync_pack) | 取消资源同步 | 写入操作 |
| [`instance_edit`](#cmd-instance-instance_edit) | 修改游戏配置 | 写入操作 |
| [`instance_edit_generated_icon`](#cmd-instance-instance_edit_generated_icon) | 生成并应用游戏图标 | 写入操作 |
| [`instance_edit_icon`](#cmd-instance-instance_edit_icon) | 设置或清除游戏图标 | 写入操作 |
| [`instance_export_mrpack`](#cmd-instance-instance_export_mrpack) | 导出整合包 | 写入操作 |
| [`instance_export_screenshots`](#cmd-instance-instance_export_screenshots) | 导出截图 | 写入操作 |
| [`instance_get`](#cmd-instance-instance_get) | 读取一个游戏 | 本地查询 |
| [`instance_get_command_history`](#cmd-instance-instance_get_command_history) | 读取游戏命令历史 | 本地查询 |
| [`instance_get_content_items`](#cmd-instance-instance_get_content_items) | 读取已装内容详情 | 查询／缓存 |
| [`instance_get_dependencies_as_content_items`](#cmd-instance-instance_get_dependencies_as_content_items) | 把依赖转为内容详情 | 查询／缓存 |
| [`instance_get_full_path`](#cmd-instance-instance_get_full_path) | 取得游戏目录路径 | 本地查询 |
| [`instance_get_game_setting_locale_labels`](#cmd-instance-instance_get_game_setting_locale_labels) | 读取游戏设置译名 | 查询／缓存 |
| [`instance_get_global_synced_options`](#cmd-instance-instance_get_global_synced_options) | 读取全局同步开关 | 本地查询 |
| [`instance_get_initialized_synced_options`](#cmd-instance-instance_get_initialized_synced_options) | 初始化并读取同步开关 | 写入操作 |
| [`instance_get_install_candidates`](#cmd-instance-instance_get_install_candidates) | 查询可安装目标 | 查询／缓存 |
| [`instance_get_installed_project_ids`](#cmd-instance-instance_get_installed_project_ids) | 读取已装项目 ID | 本地查询 |
| [`instance_get_linked_modpack_content`](#cmd-instance-instance_get_linked_modpack_content) | 读取关联整合包内容 | 查询／缓存 |
| [`instance_get_linked_modpack_info`](#cmd-instance-instance_get_linked_modpack_info) | 读取关联整合包信息 | 查询／缓存 |
| [`instance_get_local_game_options_config`](#cmd-instance-instance_get_local_game_options_config) | 读取本地游戏设置编辑状态 | 本地查询 |
| [`instance_get_many`](#cmd-instance-instance_get_many) | 批量读取游戏 | 本地查询 |
| [`instance_get_mod_full_path`](#cmd-instance-instance_get_mod_full_path) | 取得资源文件绝对路径 | 本地查询 |
| [`instance_get_optimal_jre_key`](#cmd-instance-instance_get_optimal_jre_key) | 查询合适的 Java | 本地查询 |
| [`instance_get_pack_export_candidates`](#cmd-instance-instance_get_pack_export_candidates) | 列出可导出文件 | 本地查询 |
| [`instance_get_pack_sync_preview`](#cmd-instance-instance_get_pack_sync_preview) | 预览资源同步 | 本地查询 |
| [`instance_get_projects`](#cmd-instance-instance_get_projects) | 读取内容文件映射 | 查询／缓存 |
| [`instance_get_recent_icon_configs`](#cmd-instance-instance_get_recent_icon_configs) | 读取最近图标配置 | 本地查询 |
| [`instance_get_synced_game_options_config`](#cmd-instance-instance_get_synced_game_options_config) | 读取共享游戏设置编辑状态 | 本地查询 |
| [`instance_get_synced_option_join_preview`](#cmd-instance-instance_get_synced_option_join_preview) | 预览加入同步 | 本地查询 |
| [`instance_get_synced_options_overview`](#cmd-instance-instance_get_synced_options_overview) | 读取游戏同步概况 | 本地查询 |
| [`instance_import_screenshot_groups`](#cmd-instance-instance_import_screenshot_groups) | 导入截图分组关系 | 写入操作 |
| [`instance_install_project_with_dependencies`](#cmd-instance-instance_install_project_with_dependencies) | 安装模组及前置 | 写入操作 |
| [`instance_is_file_on_modrinth`](#cmd-instance-instance_is_file_on_modrinth) | 按文件内容查询来源 | 查询／缓存 |
| [`instance_kill`](#cmd-instance-instance_kill) | 停止一个游戏 | 写入操作 |
| [`instance_list`](#cmd-instance-instance_list) | 列出全部游戏 | 本地查询 |
| [`instance_list_all_screenshots`](#cmd-instance-instance_list_all_screenshots) | 列出全部截图 | 本地查询 |
| [`instance_list_game_options_sync_sources`](#cmd-instance-instance_list_game_options_sync_sources) | 列出可选设置同步源 | 本地查询 |
| [`instance_list_groups`](#cmd-instance-instance_list_groups) | 列出游戏分组 | 本地查询 |
| [`instance_list_screenshot_groups`](#cmd-instance-instance_list_screenshot_groups) | 列出截图分组 | 本地查询 |
| [`instance_list_screenshots`](#cmd-instance-instance_list_screenshots) | 列出一个游戏的截图 | 本地查询 |
| [`instance_list_synced_packs`](#cmd-instance-instance_list_synced_packs) | 列出全局同步资源 | 本地查询 |
| [`instance_list_synced_screenshots`](#cmd-instance-instance_list_synced_screenshots) | 列出共享截图 | 本地查询 |
| [`instance_list_synced_servers`](#cmd-instance-instance_list_synced_servers) | 列出同步服务器 | 本地查询 |
| [`instance_move_screenshots`](#cmd-instance-instance_move_screenshots) | 移动截图到其他游戏 | 写入操作 |
| [`instance_open_screenshot`](#cmd-instance-instance_open_screenshot) | 用系统程序打开截图 | 界面／系统打开 |
| [`instance_open_synced_options_folder`](#cmd-instance-instance_open_synced_options_folder) | 打开同步设置目录 | 界面／系统打开 |
| [`instance_preview_local_game_option_changes`](#cmd-instance-instance_preview_local_game_option_changes) | 预览本地设置变更 | 本地查询 |
| [`instance_preview_synced_game_option_changes`](#cmd-instance-instance_preview_synced_game_option_changes) | 预览共享设置变更 | 本地查询 |
| [`instance_rebuild_synced_options`](#cmd-instance-instance_rebuild_synced_options) | 重建同步关系 | 写入操作 |
| [`instance_refresh_content_updates`](#cmd-instance-instance_refresh_content_updates) | 刷新资源更新检查 | 查询／缓存 |
| [`instance_remove`](#cmd-instance-instance_remove) | 删除游戏版本 | 删除／清理 |
| [`instance_remove_project`](#cmd-instance-instance_remove_project) | 删除已装资源 | 删除／清理 |
| [`instance_remove_synced_pack`](#cmd-instance-instance_remove_synced_pack) | 删除全局同步资源 | 删除／清理 |
| [`instance_remove_synced_server`](#cmd-instance-instance_remove_synced_server) | 删除全局同步服务器 | 删除／清理 |
| [`instance_rename_group`](#cmd-instance-instance_rename_group) | 重命名游戏分组 | 写入操作 |
| [`instance_rename_screenshot_group`](#cmd-instance-instance_rename_screenshot_group) | 重命名截图分组 | 写入操作 |
| [`instance_repair_managed_modrinth`](#cmd-instance-instance_repair_managed_modrinth) | 修复关联整合包 | 写入操作 |
| [`instance_run`](#cmd-instance-instance_run) | 启动游戏 | 写入操作 |
| [`instance_save_edited_screenshot`](#cmd-instance-instance_save_edited_screenshot) | 保存编辑后的截图 | 写入操作 |
| [`instance_save_local_game_option_changes`](#cmd-instance-instance_save_local_game_option_changes) | 保存本地游戏设置 | 写入操作 |
| [`instance_save_synced_game_option_changes`](#cmd-instance-instance_save_synced_game_option_changes) | 保存共享游戏设置 | 写入操作 |
| [`instance_set_command_history`](#cmd-instance-instance_set_command_history) | 保存游戏命令历史 | 写入操作 |
| [`instance_set_global_synced_option`](#cmd-instance-instance_set_global_synced_option) | 设置全局同步开关 | 写入操作 |
| [`instance_set_group_memberships`](#cmd-instance-instance_set_group_memberships) | 修改游戏分组成员 | 写入操作 |
| [`instance_set_group_order`](#cmd-instance-instance_set_group_order) | 调整游戏分组顺序 | 写入操作 |
| [`instance_set_project_locked`](#cmd-instance-instance_set_project_locked) | 锁定或解锁资源更新 | 写入操作 |
| [`instance_set_screenshot_group_memberships`](#cmd-instance-instance_set_screenshot_group_memberships) | 修改截图分组成员 | 写入操作 |
| [`instance_set_synced_option`](#cmd-instance-instance_set_synced_option) | 设置单个游戏同步 | 写入操作 |
| [`instance_set_synced_pack_enabled`](#cmd-instance-instance_set_synced_pack_enabled) | 启用或禁用全局同步资源 | 写入操作 |
| [`instance_share_can_current_user_use`](#cmd-instance-instance_share_can_current_user_use) | 检查共享服务权限 | 上游服务查询 |
| [`instance_share_create_invite_link`](#cmd-instance-instance_share_create_invite_link) | 创建共享游戏邀请 | 上游服务写入 |
| [`instance_share_get_invites`](#cmd-instance-instance_share_get_invites) | 列出共享游戏邀请 | 上游服务查询 |
| [`instance_share_get_publish_preview`](#cmd-instance-instance_share_get_publish_preview) | 预览共享游戏发布 | 上游服务查询 |
| [`instance_share_get_users`](#cmd-instance-instance_share_get_users) | 列出共享游戏用户 | 上游服务查询 |
| [`instance_share_invite_users`](#cmd-instance-instance_share_invite_users) | 邀请用户加入共享游戏 | 上游服务写入 |
| [`instance_share_publish`](#cmd-instance-instance_share_publish) | 发布共享游戏 | 上游服务写入 |
| [`instance_share_remove_users`](#cmd-instance-instance_share_remove_users) | 移除共享游戏成员 | 上游服务写入 |
| [`instance_share_revoke_invite`](#cmd-instance-instance_share_revoke_invite) | 撤销共享游戏邀请 | 上游服务写入 |
| [`instance_share_unlink`](#cmd-instance-instance_share_unlink) | 解除共享游戏关联 | 写入操作 |
| [`instance_share_unpublish`](#cmd-instance-instance_share_unpublish) | 取消共享游戏发布 | 上游服务写入 |
| [`instance_switch_project_version_with_dependencies`](#cmd-instance-instance_switch_project_version_with_dependencies) | 更换资源版本及前置 | 写入操作 |
| [`instance_sync_content_files`](#cmd-instance-instance_sync_content_files) | 重新扫描本地资源 | 写入操作 |
| [`instance_sync_pack`](#cmd-instance-instance_sync_pack) | 加入全局资源同步 | 写入操作 |
| [`instance_toggle_disable_project`](#cmd-instance-instance_toggle_disable_project) | 启用或禁用一个资源 | 写入操作 |
| [`instance_update_managed_modrinth_version`](#cmd-instance-instance_update_managed_modrinth_version) | 更新关联整合包版本 | 写入操作 |
| [`instance_update_project`](#cmd-instance-instance_update_project) | 更新一个已装资源 | 写入操作 |
| [`instance_update_synced_server`](#cmd-instance-instance_update_synced_server) | 修改同步服务器条目 | 写入操作 |
| [`instance_upload_synced_pack`](#cmd-instance-instance_upload_synced_pack) | 添加全局同步资源 | 写入操作 |

<a id="cmd-instance-instance_add_project_from_path"></a>

#### `plugin:instance|instance_add_project_from_path` — 安装本地资源文件

从本机绝对路径添加模组、资源包等到指定游戏；返回安装后的相对文件路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &Path | 本机源模组或资源文件的绝对路径，例如 D:/Downloads/example.jar。 |
| `projectType` | 否 | Option<ProjectType>；可选值 mod, datapack, resourcepack, shaderpack | 资源类型：mod、datapack、resourcepack 或 shaderpack；shader 也是后端接收的别名。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "D:/Downloads/example.jar",
  "projectType": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_add_project_from_path' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1207`（相对源码根目录）。

<a id="cmd-instance-instance_add_project_from_version"></a>

#### `plugin:instance|instance_add_project_from_version` — 安装指定 Modrinth 文件

按 Modrinth 文件版本 ID 下载单个项目文件并记录来源；需要自动处理前置时优先调用 install_project_with_dependencies。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `versionId` | 是 | &str | Modrinth 文件版本 ID；导出包时则是自定义包版本标签，见本条说明。 |
| `reason` | 是 | DownloadReason；可选值 standalone, dependency, modpack, update | 下载用途：standalone 单独安装、dependency 前置、modpack 整合包、update 更新。 |
| `dependentOnVersionId` | 否 | Option<String> | 被依赖方所属的 Modrinth 版本 ID，用于记录依赖下载来源；无则 null。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "versionId": "<Modrinth版本ID>",
  "reason": "standalone",
  "dependentOnVersionId": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_add_project_from_version' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1165`（相对源码根目录）。

<a id="cmd-instance-instance_cache_generated_icon"></a>

#### `plugin:instance|instance_cache_generated_icon` — 生成并缓存游戏图标

把图标配置和图案字节渲染为图标，可写入最近使用列表；不会自动给某个游戏设置图标。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `config` | 是 | theseus::data::InstanceIconConfig | 生成图标配置 InstanceIconConfig；字段按类型附录填写。 |
| `symbolBytes` | 是 | Vec<u8> | 生成图标使用的图案资源字节数组，配合 InstanceIconConfig 使用。 |
| `addToRecents` | 是 | bool | 是否把生成图标的配置加入最近使用列表。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "config": "<按 theseus::data::InstanceIconConfig 声明填写的完整值>",
  "symbolBytes": "<从实际文件读取的0到255字节数组>",
  "addToRecents": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_cache_generated_icon' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1378`（相对源码根目录）。

<a id="cmd-instance-instance_check_installed"></a>

#### `plugin:instance|instance_check_installed` — 检查项目是否已安装

按 Modrinth 项目 ID 检查目标游戏是否已安装对应内容；不用于任意文件名或 CurseForge 数字 ID。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectId` | 是 | &str | 平台项目 ID；Modrinth 是字符串，CurseForge 是数字，不能互换。 |

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectId": "<Modrinth项目ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_check_installed' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1133`（相对源码根目录）。

<a id="cmd-instance-instance_content"></a>

#### `plugin:instance|instance_content` — 读取游戏内容列表

读取指定游戏中可展示的内容及其平台信息；可按缓存策略补充在线元数据。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 ContentItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<ContentItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_content' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:617`（相对源码根目录）。

<a id="cmd-instance-instance_create_group"></a>

#### `plugin:instance|instance_create_group` — 创建游戏分组

新增游戏列表中的自定义分组；不创建游戏目录。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |

**返回：** 序列化的 theseus::instance::InstanceGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::InstanceGroup>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "name": "测试名称"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_create_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:556`（相对源码根目录）。

<a id="cmd-instance-instance_create_screenshot_group"></a>

#### `plugin:instance|instance_create_screenshot_group` — 创建截图分组

新增截图分组并可把指定截图加入其中；截图 ID 应从截图列表取得。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |
| `screenshotIds` | 是 | Vec<String> | 截图记录的内部 ID 数组，从截图列表取得。 |

**返回：** 序列化的 theseus::instance::ScreenshotGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::ScreenshotGroup>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "name": "测试名称",
  "screenshotIds": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_create_screenshot_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:741`（相对源码根目录）。

<a id="cmd-instance-instance_delete_group"></a>

#### `plugin:instance|instance_delete_group` — 删除游戏分组

删除分组及相关分组关系；不删除分组里的游戏文件。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 游戏分组或截图分组的内部 ID，从对应分组列表取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_delete_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:571`（相对源码根目录）。

<a id="cmd-instance-instance_delete_screenshot_group"></a>

#### `plugin:instance|instance_delete_screenshot_group` — 删除截图分组

删除截图分组关系；不等同于删除截图图片。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 游戏分组或截图分组的内部 ID，从对应分组列表取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_delete_screenshot_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:760`（相对源码根目录）。

<a id="cmd-instance-instance_delete_screenshots"></a>

#### `plugin:instance|instance_delete_screenshots` — 删除截图文件

按截图键批量删除实际截图；不是只从分组中移除。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `keys` | 是 | Vec<theseus::instance::ScreenshotKey> | 截图键数组，每项包含 instance_id、file_name；不是截图显示标题数组。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "keys": [
    {
      "instance_id": "<游戏ID>",
      "file_name": "<列表中的图片.png>"
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_delete_screenshots' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:779`（相对源码根目录）。

<a id="cmd-instance-instance_desync_pack"></a>

#### `plugin:instance|instance_desync_pack` — 取消资源同步

让指定游戏不再同步某个全局资源；mode 决定保留本地副本还是移除同步文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `packId` | 是 | &str | 全局同步资源的内部 ID；不是在线整合包项目 ID。 |
| `mode` | 是 | theseus::instance::DesyncServerMode；可选值 keep_in_other_instances, remove_from_other_instances | keep_in_other_instances 让其他游戏保留自己的副本；remove_from_other_instances 从其他游戏移除对应同步项。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "packId": "<packId>",
  "mode": "keep_in_other_instances"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_desync_pack' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1008`（相对源码根目录）。

<a id="cmd-instance-instance_edit"></a>

#### `plugin:instance|instance_edit` — 修改游戏配置

修改指定游戏的名称、版本设置等允许编辑的字段；只发送确实要改的字段，结构见 EditInstance。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `editInstance` | 是 | EditInstance | EditInstance 修改对象，只填要改的字段；具体字段见复杂类型附录。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "editInstance": {
    "name": "新的游戏名称"
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_edit' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1334`（相对源码根目录）。

<a id="cmd-instance-instance_edit_generated_icon"></a>

#### `plugin:instance|instance_edit_generated_icon` — 生成并应用游戏图标

用配置及图案字节生成图标并应用到游戏；onlyIfEmpty 可限制仅在没有图标时设置。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `config` | 是 | theseus::data::InstanceIconConfig | 生成图标配置 InstanceIconConfig；字段按类型附录填写。 |
| `symbolBytes` | 是 | Vec<u8> | 生成图标使用的图案资源字节数组，配合 InstanceIconConfig 使用。 |
| `onlyIfEmpty` | 否 | Option<bool> | true 仅在游戏还没有图标时设置；省略按接口默认处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "config": "<按 theseus::data::InstanceIconConfig 声明填写的完整值>",
  "symbolBytes": "<从实际文件读取的0到255字节数组>",
  "onlyIfEmpty": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_edit_generated_icon' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1352`（相对源码根目录）。

<a id="cmd-instance-instance_edit_icon"></a>

#### `plugin:instance|instance_edit_icon` — 设置或清除游戏图标

从本地图片设置游戏图标；iconPath 为 null 时清除自定义图标。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `iconPath` | 否 | Option<&Path> | 本机图片绝对路径；可空语义见对应接口，导出未指定时按游戏现有图标处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "iconPath": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_edit_icon' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1343`（相对源码根目录）。

<a id="cmd-instance-instance_export_mrpack"></a>

#### `plugin:instance|instance_export_mrpack` — 导出整合包

导出选中的文件；与在线文件哈希完全一致的资源写下载引用，修改过的或本地自制文件放入覆盖文件，支持自定义封面。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `exportLocation` | 是 | PathBuf | 导出 .mrpack 的绝对文件路径，例如 D:/Packs/export.mrpack。 |
| `includedOverrides` | 是 | Vec<String> | 导出时包含的游戏内相对路径数组，可包含 mods、config 等目录。 |
| `excludedOverrides` | 是 | Vec<String> | 导出时排除的游戏内相对路径数组。 |
| `versionId` | 否 | Option<String> | 导出包的自定义版本标签，例如 1.0.0；不是在线文件版本 ID。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `description` | 否 | Option<String> | 整合包描述文本；省略时按原导出流程决定默认值。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `name` | 否 | Option<String> | 新建对象或条目的显示名称，不能代替内部 ID。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `iconPath` | 否 | Option<PathBuf> | 本机图片绝对路径；可空语义见对应接口，导出未指定时按游戏现有图标处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 成功为 null；在 exportLocation 指定位置检查导出的 .mrpack 文件。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "exportLocation": "D:/Packs/export.mrpack",
  "includedOverrides": [
    "mods",
    "config",
    "resourcepacks",
    "shaderpacks"
  ],
  "excludedOverrides": [],
  "versionId": "1.0.0",
  "description": null,
  "name": "我的整合包",
  "iconPath": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_export_mrpack' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1279`（相对源码根目录）。

<a id="cmd-instance-instance_export_screenshots"></a>

#### `plugin:instance|instance_export_screenshots` — 导出截图

把所选截图导出到指定本地位置；原截图仍保留。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `keys` | 是 | Vec<theseus::instance::ScreenshotKey> | 截图键数组，每项包含 instance_id、file_name；不是截图显示标题数组。 |
| `exportPath` | 是 | PathBuf | 截图导出的本机目标位置；根据接口处理规则提供有效路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "keys": [
    {
      "instance_id": "<游戏ID>",
      "file_name": "<列表中的图片.png>"
    }
  ],
  "exportPath": "D:/Pictures/Export"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_export_screenshots' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:786`（相对源码根目录）。

<a id="cmd-instance-instance_get"></a>

#### `plugin:instance|instance_get` — 读取一个游戏

按内部游戏 ID 返回游戏配置；游戏不存在时返回 null。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 Instance 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<Instance>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:522`（相对源码根目录）。

<a id="cmd-instance-instance_get_command_history"></a>

#### `plugin:instance|instance_get_command_history` — 读取游戏命令历史

读取 Minecraft 的共享聊天命令历史文本；不是 Agent 命令记录，也不会执行游戏命令。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_command_history' '{}'
```

源码位置：`apps/app/src/api/instance.rs:949`（相对源码根目录）。

<a id="cmd-instance-instance_get_content_items"></a>

#### `plugin:instance|instance_get_content_items` — 读取已装内容详情

读取模组、资源包、光影等统一列表，包含识别出的名称、作者、图标、来源与版本信息。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 ContentItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<ContentItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_content_items' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:625`（相对源码根目录）。

<a id="cmd-instance-instance_get_dependencies_as_content_items"></a>

#### `plugin:instance|instance_get_dependencies_as_content_items` — 把依赖转为内容详情

按给定的 Modrinth 依赖对象查询可展示的内容信息；只查询，不安装依赖。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `dependencies` | 是 | Vec<Dependency> | 从 Modrinth 版本元数据取得的 Dependency 数组。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 ContentItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<ContentItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "dependencies": [
    "<按 Dependency 声明填写的完整值>"
  ],
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_dependencies_as_content_items' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:647`（相对源码根目录）。

<a id="cmd-instance-instance_get_full_path"></a>

#### `plugin:instance|instance_get_full_path` — 取得游戏目录路径

将内部游戏 ID 解析为该游戏的本机绝对目录路径。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<PathBuf>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_full_path' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:685`（相对源码根目录）。

<a id="cmd-instance-instance_get_game_setting_locale_labels"></a>

#### `plugin:instance|instance_get_game_setting_locale_labels` — 读取游戏设置译名

取得指定语言的游戏设置名称，可按要求刷新翻译来源缓存；不修改游戏设置值。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 否 | Option<String> | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `locale` | 是 | String | 翻译语言代码，例如 zh_cn；不是任意中文语言名称。 |
| `optionIds` | 是 | Vec<String> | 需要读取译名的游戏设置 ID 数组；取自设置编辑状态。 |
| `refreshSources` | 是 | bool | 是否刷新译名来源缓存；true 可能读取额外文件或网络。 |

**返回：** 序列化的 theseus::instance::GameSettingLocaleLabels 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::GameSettingLocaleLabels>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": null,
  "locale": "zh_cn",
  "optionIds": [
    "<对应ID或相对路径>"
  ],
  "refreshSources": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_game_setting_locale_labels' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:889`（相对源码根目录）。

<a id="cmd-instance-instance_get_global_synced_options"></a>

#### `plugin:instance|instance_get_global_synced_options` — 读取全局同步开关

读取全局游戏选项同步策略；不是启动器完整设置。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 theseus::instance::GlobalSyncedOptions 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::GlobalSyncedOptions>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_global_synced_options' '{}'
```

源码位置：`apps/app/src/api/instance.rs:851`（相对源码根目录）。

<a id="cmd-instance-instance_get_initialized_synced_options"></a>

#### `plugin:instance|instance_get_initialized_synced_options` — 初始化并读取同步开关

确保首次同步设置已初始化，再返回全局同步策略；可能写入初始化状态。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

**参数：** 无，传 `{}`。

**返回：** 序列化的 theseus::instance::GlobalSyncedOptions 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::GlobalSyncedOptions>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_initialized_synced_options' '{}'
```

源码位置：`apps/app/src/api/instance.rs:857`（相对源码根目录）。

<a id="cmd-instance-instance_get_install_candidates"></a>

#### `plugin:instance|instance_get_install_candidates` — 查询可安装目标

根据项目和候选游戏检查兼容性及安装条件，返回每个目标的可安装版本或限制。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `projectId` | 是 | &str | 平台项目 ID；Modrinth 是字符串，CurseForge 是数字，不能互换。 |
| `projectType` | 是 | ProjectType；可选值 mod, datapack, resourcepack, shaderpack | 资源类型：mod、datapack、resourcepack 或 shaderpack；shader 也是后端接收的别名。 |
| `targets` | 是 | Vec<InstanceInstallTarget> | 候选目标配置数组，每项包含 game_version 与 loader 字符串。 |

**返回：** 数组；每项为：序列化的 InstanceInstallCandidate 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<InstanceInstallCandidate>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "projectId": "<Modrinth项目ID>",
  "projectType": "mod",
  "targets": [
    {
      "game_version": "1.20.1",
      "loader": "forge"
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_install_candidates' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:603`（相对源码根目录）。

<a id="cmd-instance-instance_get_installed_project_ids"></a>

#### `plugin:instance|instance_get_installed_project_ids` — 读取已装项目 ID

返回游戏内识别到的 Modrinth 项目 ID 列表；不是所有本地文件的清单。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 数组；每项为：字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<Vec<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_installed_project_ids' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:596`（相对源码根目录）。

<a id="cmd-instance-instance_get_linked_modpack_content"></a>

#### `plugin:instance|instance_get_linked_modpack_content` — 读取关联整合包内容

返回当前游戏所关联的 Modrinth 整合包内容信息；不自动更新游戏。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 ContentItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<ContentItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_linked_modpack_content' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:673`（相对源码根目录）。

<a id="cmd-instance-instance_get_linked_modpack_info"></a>

#### `plugin:instance|instance_get_linked_modpack_info` — 读取关联整合包信息

返回游戏关联的整合包项目信息；没有关联时为 null。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 LinkedModpackInfo 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<LinkedModpackInfo>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_linked_modpack_info' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:659`（相对源码根目录）。

<a id="cmd-instance-instance_get_local_game_options_config"></a>

#### `plugin:instance|instance_get_local_game_options_config` — 读取本地游戏设置编辑状态

读取该游戏可编辑的 options 设置及修订号；修改前应先读取并保留修订信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 游戏设置编辑状态，包括设置项、当前值和并发修改修订信息，供预览及保存请求使用。 Rust 返回类型：`Result<theseus::instance::GameSettingsEditorState>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_local_game_options_config' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:919`（相对源码根目录）。

<a id="cmd-instance-instance_get_many"></a>

#### `plugin:instance|instance_get_many` — 批量读取游戏

根据内部游戏 ID 数组读取多个游戏的配置。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceIds` | 是 | Vec<String> | 多个内部游戏 ID 的数组。 |

**返回：** 数组；每项为：序列化的 Instance 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Instance>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceIds": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_many' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:529`（相对源码根目录）。

<a id="cmd-instance-instance_get_mod_full_path"></a>

#### `plugin:instance|instance_get_mod_full_path` — 取得资源文件绝对路径

把游戏 ID 和游戏内相对资源路径解析为绝对路径，仍受目录边界检查。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<PathBuf>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_mod_full_path' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:690`（相对源码根目录）。

<a id="cmd-instance-instance_get_optimal_jre_key"></a>

#### `plugin:instance|instance_get_optimal_jre_key` — 查询合适的 Java

根据游戏版本与加载器查询合适的已登记 Java 配置；没有合适 Java 时可为空，不会自动安装。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 检测得到的 Java 完整配置（路径、版本和架构等），可原样供 set_java_version 使用。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<JavaVersion>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_optimal_jre_key' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1126`（相对源码根目录）。

<a id="cmd-instance-instance_get_pack_export_candidates"></a>

#### `plugin:instance|instance_get_pack_export_candidates` — 列出可导出文件

列出游戏中可选的整合包覆盖文件，可指定相对子目录；不会生成整合包。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `parent` | 否 | Option<SafeRelativeUtf8UnixPathBuf> | 导出候选文件的相对子目录；省略或 null 从游戏根目录开始。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 theseus::instance::PackExportCandidate 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::PackExportCandidate>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "parent": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_pack_export_candidates' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1304`（相对源码根目录）。

<a id="cmd-instance-instance_get_pack_sync_preview"></a>

#### `plugin:instance|instance_get_pack_sync_preview` — 预览资源同步

检查游戏中的一个资源加入全局同步后的影响及冲突。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |

**返回：** 序列化的 theseus::instance::PackSyncPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::PackSyncPreview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_pack_sync_preview' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:989`（相对源码根目录）。

<a id="cmd-instance-instance_get_projects"></a>

#### `plugin:instance|instance_get_projects` — 读取内容文件映射

返回游戏内资源的相对路径到文件元数据映射，可补充在线来源信息。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cacheBehaviour` | 否 | Option<CacheBehaviour>；可选值 stale_while_revalidate_skip_offline, stale_while_revalidate, must_revalidate, bypass | 缓存策略；省略或 null 使用默认。四种取值见参数类型表。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 DashMap<String, ContentFile> 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<DashMap<String, ContentFile>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cacheBehaviour": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_projects' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:588`（相对源码根目录）。

<a id="cmd-instance-instance_get_recent_icon_configs"></a>

#### `plugin:instance|instance_get_recent_icon_configs` — 读取最近图标配置

返回最近使用过的生成图标配置。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::data::InstanceIconConfig 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::data::InstanceIconConfig>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_recent_icon_configs' '{}'
```

源码位置：`apps/app/src/api/instance.rs:1392`（相对源码根目录）。

<a id="cmd-instance-instance_get_synced_game_options_config"></a>

#### `plugin:instance|instance_get_synced_game_options_config` — 读取共享游戏设置编辑状态

读取全局同步的游戏设置及修订号，供预览和提交修改使用。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 游戏设置编辑状态，包括设置项、当前值和并发修改修订信息，供预览及保存请求使用。 Rust 返回类型：`Result<theseus::instance::GameSettingsEditorState>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_synced_game_options_config' '{}'
```

源码位置：`apps/app/src/api/instance.rs:883`（相对源码根目录）。

<a id="cmd-instance-instance_get_synced_option_join_preview"></a>

#### `plugin:instance|instance_get_synced_option_join_preview` — 预览加入同步

检查一个游戏开启某类同步时与现有共享数据的冲突和可选处理方式。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `option` | 是 | InstanceSyncedOption；可选值 game_options, command_history, multiplayer_servers, creative_hotbars, screenshots, resource_packs, data_packs | 游戏同步选项枚举；data_packs 虽在枚举中但当前不支持开启。 |

**返回：** 序列化的 theseus::instance::SyncedOptionJoinPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SyncedOptionJoinPreview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "option": "game_options"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_synced_option_join_preview' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:833`（相对源码根目录）。

<a id="cmd-instance-instance_get_synced_options_overview"></a>

#### `plugin:instance|instance_get_synced_options_overview` — 读取游戏同步概况

返回该游戏各类同步项目是否开启、状态和相关信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 theseus::instance::SyncedOptionsOverview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SyncedOptionsOverview>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_get_synced_options_overview' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:844`（相对源码根目录）。

<a id="cmd-instance-instance_import_screenshot_groups"></a>

#### `plugin:instance|instance_import_screenshot_groups` — 导入截图分组关系

按给定分组数据导入或合并截图分组；不是导入截图图片本身。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `groups` | 是 | Vec<theseus::instance::ScreenshotGroupImport> | 待导入的 ScreenshotGroupImport 数组，字段 id、name、screenshot_ids。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "groups": [
    {
      "id": "<截图分组ID>",
      "name": "截图分组",
      "screenshot_ids": [
        "<截图ID>"
      ]
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_import_screenshot_groups' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:772`（相对源码根目录）。

<a id="cmd-instance-instance_install_project_with_dependencies"></a>

#### `plugin:instance|instance_install_project_with_dependencies` — 安装模组及前置

按项目、指定或自动选择的兼容版本解析 Modrinth 依赖计划，安装项目及所需前置并返回处理计划。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `request` | 是 | InstallProjectWithDependenciesRequest | Modrinth 安装请求（snake_case）：project_id 项目ID、version_id 可选文件版本ID、content_type 资源类型、selected 可选依赖选择。 |

**返回：** 解析并执行的内容安装计划，包含依赖和文件处理信息；后续可重新查询游戏内容确认。 Rust 返回类型：`Result<ResolveContentPlan>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "request": {
    "project_id": "<Modrinth项目ID>",
    "version_id": null,
    "content_type": "mod"
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_install_project_with_dependencies' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1181`（相对源码根目录）。

<a id="cmd-instance-instance_is_file_on_modrinth"></a>

#### `plugin:instance|instance_is_file_on_modrinth` — 按文件内容查询来源

计算本地文件标识并查询 Modrinth 是否存在完全匹配的文件；不是只比较文件名。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `projectPath` | 是 | &Path | 本机源模组或资源文件的绝对路径，例如 D:/Downloads/example.jar。 |

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "projectPath": "D:/Downloads/example.jar"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_is_file_on_modrinth' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1221`（相对源码根目录）。

<a id="cmd-instance-instance_kill"></a>

#### `plugin:instance|instance_kill` — 停止一个游戏

终止该游戏关联的 Minecraft 进程；未保存的游戏进度可能丢失。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_kill' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1328`（相对源码根目录）。

<a id="cmd-instance-instance_list"></a>

#### `plugin:instance|instance_list` — 列出全部游戏

返回启动器登记的所有游戏配置及内部 ID。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 Instance 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Instance>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list' '{}'
```

源码位置：`apps/app/src/api/instance.rs:541`（相对源码根目录）。

<a id="cmd-instance-instance_list_all_screenshots"></a>

#### `plugin:instance|instance_list_all_screenshots` — 列出全部截图

读取所有游戏中的截图及其所属游戏信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 InstanceScreenshot 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<InstanceScreenshot>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_all_screenshots' '{}'
```

源码位置：`apps/app/src/api/instance.rs:707`（相对源码根目录）。

<a id="cmd-instance-instance_list_game_options_sync_sources"></a>

#### `plugin:instance|instance_list_game_options_sync_sources` — 列出可选设置同步源

返回可用作游戏设置同步基础的游戏候选项。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::instance::GameOptionsSourceCandidate 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::GameOptionsSourceCandidate>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_game_options_sync_sources' '{}'
```

源码位置：`apps/app/src/api/instance.rs:877`（相对源码根目录）。

<a id="cmd-instance-instance_list_groups"></a>

#### `plugin:instance|instance_list_groups` — 列出游戏分组

读取自定义游戏分组及排序信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::instance::InstanceGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::InstanceGroup>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_groups' '{}'
```

源码位置：`apps/app/src/api/instance.rs:550`（相对源码根目录）。

<a id="cmd-instance-instance_list_screenshot_groups"></a>

#### `plugin:instance|instance_list_screenshot_groups` — 列出截图分组

读取已登记的截图分组。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::instance::ScreenshotGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::ScreenshotGroup>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_screenshot_groups' '{}'
```

源码位置：`apps/app/src/api/instance.rs:735`（相对源码根目录）。

<a id="cmd-instance-instance_list_screenshots"></a>

#### `plugin:instance|instance_list_screenshots` — 列出一个游戏的截图

读取指定游戏的截图及后续打开、移动和删除所需的截图键。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 数组；每项为：序列化的 InstanceScreenshot 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<InstanceScreenshot>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_screenshots' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:698`（相对源码根目录）。

<a id="cmd-instance-instance_list_synced_packs"></a>

#### `plugin:instance|instance_list_synced_packs` — 列出全局同步资源

按资源类型列出全局同步的模组、资源包等；不是在线整合包搜索。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `projectType` | 是 | ProjectType；可选值 mod, datapack, resourcepack, shaderpack | 资源类型：mod、datapack、resourcepack 或 shaderpack；shader 也是后端接收的别名。 |

**返回：** 数组；每项为：序列化的 ContentItem 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<ContentItem>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "projectType": "mod"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_synced_packs' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1017`（相对源码根目录）。

<a id="cmd-instance-instance_list_synced_screenshots"></a>

#### `plugin:instance|instance_list_synced_screenshots` — 列出共享截图

读取全局截图同步目录中的截图信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 InstanceScreenshot 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<InstanceScreenshot>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_synced_screenshots' '{}'
```

源码位置：`apps/app/src/api/instance.rs:715`（相对源码根目录）。

<a id="cmd-instance-instance_list_synced_servers"></a>

#### `plugin:instance|instance_list_synced_servers` — 列出同步服务器

读取全局同步的多人服务器条目；不会连接服务器。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 theseus::instance::SyncedServer 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::SyncedServer>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_list_synced_servers' '{}'
```

源码位置：`apps/app/src/api/instance.rs:971`（相对源码根目录）。

<a id="cmd-instance-instance_move_screenshots"></a>

#### `plugin:instance|instance_move_screenshots` — 移动截图到其他游戏

将所选截图文件移动到目标游戏截图目录，返回更新后的截图键。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `keys` | 是 | Vec<theseus::instance::ScreenshotKey> | 截图键数组，每项包含 instance_id、file_name；不是截图显示标题数组。 |
| `targetInstanceId` | 是 | &str | 截图要移动到的目标游戏 ID。 |

**返回：** 数组；每项为：序列化的 theseus::instance::ScreenshotKey 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::ScreenshotKey>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "keys": [
    {
      "instance_id": "<游戏ID>",
      "file_name": "<列表中的图片.png>"
    }
  ],
  "targetInstanceId": "<目标游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_move_screenshots' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:794`（相对源码根目录）。

<a id="cmd-instance-instance_open_screenshot"></a>

#### `plugin:instance|instance_open_screenshot` — 用系统程序打开截图

按截图键解析实际图片路径并交给系统打开。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `key` | 是 | theseus::instance::ScreenshotKey | 截图键对象 {"instance_id":"游戏ID","file_name":"图片.png"}，应由列表返回数据构造。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "key": {
    "instance_id": "<游戏ID>",
    "file_name": "<列表中的图片.png>"
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_open_screenshot' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:802`（相对源码根目录）。

<a id="cmd-instance-instance_open_synced_options_folder"></a>

#### `plugin:instance|instance_open_synced_options_folder` — 打开同步设置目录

在文件管理器中打开共享游戏选项目录。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_open_synced_options_folder' '{}'
```

源码位置：`apps/app/src/api/instance.rs:959`（相对源码根目录）。

<a id="cmd-instance-instance_preview_local_game_option_changes"></a>

#### `plugin:instance|instance_preview_local_game_option_changes` — 预览本地设置变更

校验游戏设置变更请求及修订冲突，返回预览状态；不提交设置修改。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `request` | 是 | theseus::instance::UpdateGameSettingsRequest | 带修订号的游戏设置变更；先读取编辑状态，保留 expected_*_revision 与 option 修订号，预览后再保存，不能编造为 0。 |

**返回：** 游戏设置编辑状态，包括设置项、当前值和并发修改修订信息，供预览及保存请求使用。 Rust 返回类型：`Result<theseus::instance::GameSettingsEditorState>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "request": "<按 theseus::instance::UpdateGameSettingsRequest 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_preview_local_game_option_changes' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:926`（相对源码根目录）。

<a id="cmd-instance-instance_preview_synced_game_option_changes"></a>

#### `plugin:instance|instance_preview_synced_game_option_changes` — 预览共享设置变更

校验共享游戏设置的变更及冲突，返回预览；不保存新值。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `request` | 是 | theseus::instance::UpdateGameSettingsRequest | 带修订号的游戏设置变更；先读取编辑状态，保留 expected_*_revision 与 option 修订号，预览后再保存，不能编造为 0。 |

**返回：** 游戏设置编辑状态，包括设置项、当前值和并发修改修订信息，供预览及保存请求使用。 Rust 返回类型：`Result<theseus::instance::GameSettingsEditorState>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "request": "<按 theseus::instance::UpdateGameSettingsRequest 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_preview_synced_game_option_changes' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:905`（相对源码根目录）。

<a id="cmd-instance-instance_rebuild_synced_options"></a>

#### `plugin:instance|instance_rebuild_synced_options` — 重建同步关系

重新建立指定游戏或全部游戏的设置同步关系；会更新相关文件链接或同步内容。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 否 | Option<&str> | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_rebuild_synced_options' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1053`（相对源码根目录）。

<a id="cmd-instance-instance_refresh_content_updates"></a>

#### `plugin:instance|instance_refresh_content_updates` — 刷新资源更新检查

重新查询目标游戏资源的可用更新并更新检查缓存；不会直接安装更新。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_refresh_content_updates' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:642`（相对源码根目录）。

<a id="cmd-instance-instance_remove"></a>

#### `plugin:instance|instance_remove` — 删除游戏版本

删除目标游戏及其目录中的内容；包括其中的模组、配置和存档，应先备份。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_remove' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:516`（相对源码根目录）。

<a id="cmd-instance-instance_remove_project"></a>

#### `plugin:instance|instance_remove_project` — 删除已装资源

按游戏内相对路径移除模组、资源包或光影文件及管理记录。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_remove_project' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1251`（相对源码根目录）。

<a id="cmd-instance-instance_remove_synced_pack"></a>

#### `plugin:instance|instance_remove_synced_pack` — 删除全局同步资源

移除全局同步资源及关联的同步关系；会影响使用该共享资源的游戏。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `packId` | 是 | &str | 全局同步资源的内部 ID；不是在线整合包项目 ID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "packId": "<packId>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_remove_synced_pack' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1048`（相对源码根目录）。

<a id="cmd-instance-instance_remove_synced_server"></a>

#### `plugin:instance|instance_remove_synced_server` — 删除全局同步服务器

从全局同步服务器列表移除指定条目，并处理关联游戏的同步项。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `serverId` | 是 | &str | 全局同步服务器的内部 ID，不是服务器 IP 地址。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "serverId": "<serverId>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_remove_synced_server' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:984`（相对源码根目录）。

<a id="cmd-instance-instance_rename_group"></a>

#### `plugin:instance|instance_rename_group` — 重命名游戏分组

修改一个自定义游戏分组的显示名称。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 游戏分组或截图分组的内部 ID，从对应分组列表取得。 |
| `newName` | 是 | String | 新的显示名称；世界重命名只修改世界显示名称。 |

**返回：** 序列化的 theseus::instance::InstanceGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::InstanceGroup>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "newName": "新名称"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_rename_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:563`（相对源码根目录）。

<a id="cmd-instance-instance_rename_screenshot_group"></a>

#### `plugin:instance|instance_rename_screenshot_group` — 重命名截图分组

修改截图分组的显示名称。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 游戏分组或截图分组的内部 ID，从对应分组列表取得。 |
| `newName` | 是 | String | 新的显示名称；世界重命名只修改世界显示名称。 |

**返回：** 序列化的 theseus::instance::ScreenshotGroup 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::ScreenshotGroup>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "newName": "新名称"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_rename_screenshot_group' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:752`（相对源码根目录）。

<a id="cmd-instance-instance_repair_managed_modrinth"></a>

#### `plugin:instance|instance_repair_managed_modrinth` — 修复关联整合包

为受管理的 Modrinth 整合包游戏创建修复任务，依据其关联版本恢复所需文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<theseus::install::InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_repair_managed_modrinth' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1272`（相对源码根目录）。

<a id="cmd-instance-instance_run"></a>

#### `plugin:instance|instance_run` — 启动游戏

按目标游戏配置准备运行环境并启动 Minecraft，可指定自动加入的服务器地址。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `serverAddress` | 否 | Option<String> | 启动时要自动加入的服务器地址；null 正常进入游戏。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。 Rust 返回类型：`Result<ProcessMetadata>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "serverAddress": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_run' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1316`（相对源码根目录）。

<a id="cmd-instance-instance_save_edited_screenshot"></a>

#### `plugin:instance|instance_save_edited_screenshot` — 保存编辑后的截图

把 PNG 字节按指定模式覆盖原截图或另存为副本；返回保存后的截图信息。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `key` | 是 | theseus::instance::ScreenshotKey | 截图键对象 {"instance_id":"游戏ID","file_name":"图片.png"}，应由列表返回数据构造。 |
| `pngBytes` | 是 | Vec<u8> | 编辑后 PNG 图片的实际字节数组；不是路径或 Base64 文本。 |
| `mode` | 是 | theseus::instance::ScreenshotEditSaveMode；可选值 create_copy, replace_edit | create_copy 另存副本；replace_edit 覆盖当前截图。 |

**返回：** 序列化的 InstanceScreenshot 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<InstanceScreenshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "key": {
    "instance_id": "<游戏ID>",
    "file_name": "<列表中的图片.png>"
  },
  "pngBytes": "<从实际文件读取的0到255字节数组>",
  "mode": "create_copy"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_save_edited_screenshot' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:723`（相对源码根目录）。

<a id="cmd-instance-instance_save_local_game_option_changes"></a>

#### `plugin:instance|instance_save_local_game_option_changes` — 保存本地游戏设置

按编辑请求和修订号提交 options 设置；返回成功、延后、失败及冲突等结果。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `request` | 是 | theseus::instance::UpdateGameSettingsRequest | 带修订号的游戏设置变更；先读取编辑状态，保留 expected_*_revision 与 option 修订号，预览后再保存，不能编造为 0。 |

**返回：** 设置保存结果，包含 state、applied、migrated、deferred、unsupported、failed、conflicts；调用成功不代表每一项都已应用。 Rust 返回类型：`Result<theseus::instance::SaveGameSettingsResult>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "request": "<按 theseus::instance::UpdateGameSettingsRequest 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_save_local_game_option_changes' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:938`（相对源码根目录）。

<a id="cmd-instance-instance_save_synced_game_option_changes"></a>

#### `plugin:instance|instance_save_synced_game_option_changes` — 保存共享游戏设置

提交全局共享的游戏设置并处理同步；可能影响启用了对应同步的多个游戏。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `request` | 是 | theseus::instance::UpdateGameSettingsRequest | 带修订号的游戏设置变更；先读取编辑状态，保留 expected_*_revision 与 option 修订号，预览后再保存，不能编造为 0。 |

**返回：** 设置保存结果，包含 state、applied、migrated、deferred、unsupported、failed、conflicts；调用成功不代表每一项都已应用。 Rust 返回类型：`Result<theseus::instance::SaveGameSettingsResult>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "request": "<按 theseus::instance::UpdateGameSettingsRequest 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_save_synced_game_option_changes' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:912`（相对源码根目录）。

<a id="cmd-instance-instance_set_command_history"></a>

#### `plugin:instance|instance_set_command_history` — 保存游戏命令历史

覆盖共享 Minecraft 命令历史文本并按上限规范化内容；不会在游戏里执行这些命令。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `contents` | 是 | &str | Minecraft 命令历史的完整文本，按换行分隔；不是待执行命令。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "contents": "/help\n"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_command_history' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:954`（相对源码根目录）。

<a id="cmd-instance-instance_set_global_synced_option"></a>

#### `plugin:instance|instance_set_global_synced_option` — 设置全局同步开关

修改一类全局同步功能，可选择一个游戏作为初始数据源；可能改变多个游戏的同步关系。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `option` | 是 | InstanceSyncedOption；可选值 game_options, command_history, multiplayer_servers, creative_hotbars, screenshots, resource_packs, data_packs | 游戏同步选项枚举；data_packs 虽在枚举中但当前不支持开启。 |
| `enabled` | 是 | bool | 是否启用此项功能或同步资源。 |
| `baseInstanceId` | 否 | Option<String> | 初始化共享数据时采用的游戏 ID；省略表示不指定基础游戏。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 theseus::instance::GlobalSyncedOptions 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::GlobalSyncedOptions>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "option": "game_options",
  "enabled": false,
  "baseInstanceId": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_global_synced_option' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:863`（相对源码根目录）。

<a id="cmd-instance-instance_set_group_memberships"></a>

#### `plugin:instance|instance_set_group_memberships` — 修改游戏分组成员

批量更新游戏所属的自定义分组；不移动游戏目录。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `updates` | 是 | Vec<theseus::instance::InstanceGroupMembershipUpdate> | 游戏分组更新数组，每项 {"instance_id":"游戏ID","group_ids":["分组ID"]}；空数组清除所属分组。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "updates": [
    {
      "instance_id": "<游戏ID>",
      "group_ids": [
        "<分组ID>"
      ]
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_group_memberships' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:581`（相对源码根目录）。

<a id="cmd-instance-instance_set_group_order"></a>

#### `plugin:instance|instance_set_group_order` — 调整游戏分组顺序

按分组 ID 数组保存自定义分组的显示顺序。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `groupIds` | 是 | Vec<String> | 游戏分组内部 ID 数组；用于排序或成员关系，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "groupIds": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_group_order' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:576`（相对源码根目录）。

<a id="cmd-instance-instance_set_project_locked"></a>

#### `plugin:instance|instance_set_project_locked` — 锁定或解锁资源更新

设置一个已装资源的更新锁定状态；不是操作系统文件锁，也不会禁用模组。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |
| `locked` | 是 | bool | true 锁定资源更新，false 解锁；不改变资源的启用状态。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>",
  "locked": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_project_locked' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1240`（相对源码根目录）。

<a id="cmd-instance-instance_set_screenshot_group_memberships"></a>

#### `plugin:instance|instance_set_screenshot_group_memberships` — 修改截图分组成员

批量更新截图属于哪些分组；不移动截图图片文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `updates` | 是 | Vec<theseus::instance::ScreenshotGroupMembershipUpdate> | 截图分组更新数组，每项 {"screenshot_id":"截图ID","group_id":"分组ID或null"}；null 清除分组。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "updates": [
    {
      "screenshot_id": "<截图ID>",
      "group_id": "<分组ID>"
    }
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_screenshot_group_memberships' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:765`（相对源码根目录）。

<a id="cmd-instance-instance_set_synced_option"></a>

#### `plugin:instance|instance_set_synced_option` — 设置单个游戏同步

开启或关闭该游戏的一类同步；有冲突时需要传入预览所对应的处理决定。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `option` | 是 | InstanceSyncedOption；可选值 game_options, command_history, multiplayer_servers, creative_hotbars, screenshots, resource_packs, data_packs | 游戏同步选项枚举；data_packs 虽在枚举中但当前不支持开启。 |
| `enabled` | 是 | bool | 是否启用此项功能或同步资源。 |
| `resolution` | 否 | Option<theseus::instance::SyncedOptionJoinResolution>；可选值 use_synced, use_instance | 发生同步冲突时选 use_synced 使用共享数据，或 use_instance 使用当前游戏数据；先查询加入预览。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 Instance 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Instance>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "option": "game_options",
  "enabled": false,
  "resolution": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_synced_option' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:815`（相对源码根目录）。

<a id="cmd-instance-instance_set_synced_pack_enabled"></a>

#### `plugin:instance|instance_set_synced_pack_enabled` — 启用或禁用全局同步资源

修改一个全局共享资源的启用状态，会影响同步使用它的游戏。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `packId` | 是 | &str | 全局同步资源的内部 ID；不是在线整合包项目 ID。 |
| `enabled` | 是 | bool | 是否启用此项功能或同步资源。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "packId": "<packId>",
  "enabled": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_set_synced_pack_enabled' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1040`（相对源码根目录）。

<a id="cmd-instance-instance_share_can_current_user_use"></a>

#### `plugin:instance|instance_share_can_current_user_use` — 检查共享服务权限

查询当前上游账户能否使用在线游戏共享；本地离线账户不提供上游登录身份。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_can_current_user_use' '{}'
```

源码位置：`apps/app/src/api/instance.rs:1398`（相对源码根目录）。

<a id="cmd-instance-instance_share_create_invite_link"></a>

#### `plugin:instance|instance_share_create_invite_link` — 创建共享游戏邀请

在上游服务创建邀请链接，可限制有效期和使用次数；会改变服务端邀请状态。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `maxAgeSeconds` | 否 | Option<i32> | 共享邀请有效期秒数；省略时由上游默认规则处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `maxUses` | 否 | Option<i32> | 共享邀请最多使用次数；省略时由上游默认规则处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `replaceInviteId` | 否 | Option<String> | 用新链接替换的旧共享邀请 ID；省略不指定旧邀请。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 theseus::instance::SharedInstanceInviteLink 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SharedInstanceInviteLink>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "maxAgeSeconds": null,
  "maxUses": null,
  "replaceInviteId": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_create_invite_link' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1421`（相对源码根目录）。

<a id="cmd-instance-instance_share_get_invites"></a>

#### `plugin:instance|instance_share_get_invites` — 列出共享游戏邀请

读取指定共享游戏的邀请记录，依赖上游权限。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 数组；每项为：序列化的 theseus::instance::SharedInstanceInvite 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<theseus::instance::SharedInstanceInvite>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_get_invites' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1437`（相对源码根目录）。

<a id="cmd-instance-instance_share_get_publish_preview"></a>

#### `plugin:instance|instance_share_get_publish_preview` — 预览共享游戏发布

取得共享游戏发布所需的差异和发布信息；不执行发布。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 theseus::instance::SharedInstancePublishPreview 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<theseus::instance::SharedInstancePublishPreview>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_get_publish_preview' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1472`（相对源码根目录）。

<a id="cmd-instance-instance_share_get_users"></a>

#### `plugin:instance|instance_share_get_users` — 列出共享游戏用户

读取共享游戏的成员及邀请状态。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 theseus::instance::SharedInstanceUsers 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SharedInstanceUsers>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_get_users' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1403`（相对源码根目录）。

<a id="cmd-instance-instance_share_invite_users"></a>

#### `plugin:instance|instance_share_invite_users` — 邀请用户加入共享游戏

向上游共享游戏添加指定用户的邀请；这是对其他用户和服务端状态的操作。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `userIds` | 是 | Vec<String> | 多个 Modrinth 平台用户 ID 数组。 |

**返回：** 序列化的 theseus::instance::SharedInstanceUsers 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SharedInstanceUsers>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "userIds": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_invite_users' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1410`（相对源码根目录）。

<a id="cmd-instance-instance_share_publish"></a>

#### `plugin:instance|instance_share_publish` — 发布共享游戏

把游戏及选中的配置路径发布到上游共享服务；不是导出本地 mrpack。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `configPaths` | 是 | Vec<String> | 发布共享游戏时包含的配置相对路径数组；可能上传这些文件。 |

**返回：** 序列化的 SharedInstanceAttachment 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<SharedInstanceAttachment>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "configPaths": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_publish' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1482`（相对源码根目录）。

<a id="cmd-instance-instance_share_remove_users"></a>

#### `plugin:instance|instance_share_remove_users` — 移除共享游戏成员

从共享服务中移除指定成员或待接受邀请者，可能撤销其访问权限。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `userIds` | 是 | Vec<String> | 多个 Modrinth 平台用户 ID 数组。 |
| `hasPendingRecipients` | 是 | bool | 成员移除流程是否还包含待接受邀请者，按当前共享成员状态填写。 |

**返回：** 序列化的 theseus::instance::SharedInstanceUsers 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::instance::SharedInstanceUsers>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "userIds": [
    "<对应ID或相对路径>"
  ],
  "hasPendingRecipients": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_remove_users' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1458`（相对源码根目录）。

<a id="cmd-instance-instance_share_revoke_invite"></a>

#### `plugin:instance|instance_share_revoke_invite` — 撤销共享游戏邀请

在上游服务撤销指定邀请链接。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `inviteId` | 是 | String | 上游共享邀请的 ID，从邀请查询结果取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "inviteId": "<inviteId>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_revoke_invite' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1444`（相对源码根目录）。

<a id="cmd-instance-instance_share_unlink"></a>

#### `plugin:instance|instance_share_unlink` — 解除共享游戏关联

解除本地游戏与共享实例的管理关联；不是导出整合包。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_unlink' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1494`（相对源码根目录）。

<a id="cmd-instance-instance_share_unpublish"></a>

#### `plugin:instance|instance_share_unpublish` — 取消共享游戏发布

在上游服务取消游戏共享发布，会影响其他人的共享访问。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_share_unpublish' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1500`（相对源码根目录）。

<a id="cmd-instance-instance_switch_project_version_with_dependencies"></a>

#### `plugin:instance|instance_switch_project_version_with_dependencies` — 更换资源版本及前置

将一个已装 Modrinth 资源切换到指定文件版本，同时重新处理依赖并返回新文件相对路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |
| `versionId` | 是 | &str | Modrinth 文件版本 ID；导出包时则是自定义包版本标签，见本条说明。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>",
  "versionId": "<Modrinth版本ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_switch_project_version_with_dependencies' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1193`（相对源码根目录）。

<a id="cmd-instance-instance_sync_content_files"></a>

#### `plugin:instance|instance_sync_content_files` — 重新扫描本地资源

扫描实际模组、资源包和光影文件，更新内容索引及外部改动状态；不会把扫描当成下载。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_sync_content_files' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:636`（相对源码根目录）。

<a id="cmd-instance-instance_sync_pack"></a>

#### `plugin:instance|instance_sync_pack` — 加入全局资源同步

把游戏中的指定模组或资源包纳入全局资源同步；会改变该文件的管理和共享关系。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_sync_pack' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1000`（相对源码根目录）。

<a id="cmd-instance-instance_toggle_disable_project"></a>

#### `plugin:instance|instance_toggle_disable_project` — 启用或禁用一个资源

设置资源启用状态；desiredEnabled 省略时切换，true 启用、false 禁用，返回调整后的路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |
| `desiredEnabled` | 否 | Option<bool> | true 启用，false 禁用；省略或 null 时切换当前状态。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>",
  "desiredEnabled": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_toggle_disable_project' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1226`（相对源码根目录）。

<a id="cmd-instance-instance_update_managed_modrinth_version"></a>

#### `plugin:instance|instance_update_managed_modrinth_version` — 更新关联整合包版本

为关联的 Modrinth 整合包切换到指定包版本并创建安装任务；可能替换模组与配置。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `versionId` | 是 | String | Modrinth 文件版本 ID；导出包时则是自定义包版本标签，见本条说明。 |

**返回：** 安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。 Rust 返回类型：`Result<theseus::install::InstallJobSnapshot>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "versionId": "<Modrinth版本ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_update_managed_modrinth_version' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1260`（相对源码根目录）。

<a id="cmd-instance-instance_update_project"></a>

#### `plugin:instance|instance_update_project` — 更新一个已装资源

根据目标游戏的兼容条件更新一个 Modrinth 资源到可用版本，返回新文件路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `projectPath` | 是 | &str | 目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "projectPath": "mods/<模组文件.jar>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_update_project' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1154`（相对源码根目录）。

<a id="cmd-instance-instance_update_synced_server"></a>

#### `plugin:instance|instance_update_synced_server` — 修改同步服务器条目

保存完整的全局同步服务器条目，并更新同步使用该条目的游戏。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `server` | 是 | theseus::instance::SyncedServer | 快捷方式的服务器地址；null 不设置自动加入服务器。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "server": {
    "id": "<同步服务器ID>",
    "name": "测试服务器",
    "address": "play.example.com:25565",
    "accept_textures": null
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_update_synced_server' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:977`（相对源码根目录）。

<a id="cmd-instance-instance_upload_synced_pack"></a>

#### `plugin:instance|instance_upload_synced_pack` — 添加全局同步资源

从本地文件把模组、资源包等加入全局同步库，并记录适用的游戏版本；这里的 upload 是本地同步库导入。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | &Path | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |
| `projectType` | 是 | ProjectType；可选值 mod, datapack, resourcepack, shaderpack | 资源类型：mod、datapack、resourcepack 或 shaderpack；shader 也是后端接收的别名。 |
| `gameVersions` | 是 | Vec<String> | 该资源适用的 Minecraft 版本字符串数组，例如 ["1.20.1"]。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file",
  "projectType": "mod",
  "gameVersions": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:instance|instance_upload_synced_pack' '@args.json'
```

源码位置：`apps/app/src/api/instance.rs:1024`（相对源码根目录）。

<a id="api-jre"></a>

### Java 运行环境

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`get_java_versions`](#cmd-jre-get_java_versions) | 读取已登记 Java | 本地查询 |
| [`jre_auto_install_java`](#cmd-jre-jre_auto_install_java) | 下载并安装 Java | 写入操作 |
| [`jre_find_filtered_jres`](#cmd-jre-jre_find_filtered_jres) | 扫描可用 Java | 本地查询 |
| [`jre_get_jre`](#cmd-jre-jre_get_jre) | 检测一个 Java 程序 | 本地查询 |
| [`jre_get_max_memory`](#cmd-jre-jre_get_max_memory) | 读取可分配内存上限 | 本地查询 |
| [`jre_test_jre`](#cmd-jre-jre_test_jre) | 验证 Java 主版本 | 本地查询 |
| [`set_java_version`](#cmd-jre-set_java_version) | 登记所选 Java | 写入操作 |

<a id="cmd-jre-get_java_versions"></a>

#### `plugin:jre|get_java_versions` — 读取已登记 Java

返回 Java 主版本号与所选 Java 配置的映射；不是扫描电脑上的全部 Java。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 DashMap<u32, JavaVersion> 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<DashMap<u32, JavaVersion>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:jre|get_java_versions' '{}'
```

源码位置：`apps/app/src/api/jre.rs:23`（相对源码根目录）。

<a id="cmd-jre-jre_auto_install_java"></a>

#### `plugin:jre|jre_auto_install_java` — 下载并安装 Java

按主版本号自动取得当前平台所需的 Java 运行环境，返回本地安装路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `javaVersion` | 是 | u32 | 要安装的 Java 主版本整数，例如 8、17 或 21。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<PathBuf>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "javaVersion": 17
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:jre|jre_auto_install_java' '@args.json'
```

源码位置：`apps/app/src/api/jre.rs:56`（相对源码根目录）。

<a id="cmd-jre-jre_find_filtered_jres"></a>

#### `plugin:jre|jre_find_filtered_jres` — 扫描可用 Java

查找系统中的 Java，并可按主版本号筛选；返回检测到的配置数组。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `version` | 否 | Option<u32> | 筛选的 Java 主版本数字；省略或 null 不按主版本筛选。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：检测得到的 Java 完整配置（路径、版本和架构等），可原样供 set_java_version 使用。 Rust 返回类型：`Result<Vec<JavaVersion>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "version": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:jre|jre_find_filtered_jres' '@args.json'
```

源码位置：`apps/app/src/api/jre.rs:35`（相对源码根目录）。

<a id="cmd-jre-jre_get_jre"></a>

#### `plugin:jre|jre_get_jre` — 检测一个 Java 程序

运行或检测给定路径的 Java，取得主版本、架构和路径等配置。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 待检测的本机 Java 可执行文件绝对路径，例如 D:/Java/bin/java.exe。 |

**返回：** 检测得到的 Java 完整配置（路径、版本和架构等），可原样供 set_java_version 使用。 Rust 返回类型：`Result<JavaVersion>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:jre|jre_get_jre' '@args.json'
```

源码位置：`apps/app/src/api/jre.rs:44`（相对源码根目录）。

<a id="cmd-jre-jre_get_max_memory"></a>

#### `plugin:jre|jre_get_max_memory` — 读取可分配内存上限

返回系统检测到的内存总量，单位 KiB（字节数除以 1024）；不是 Minecraft 当前已分配内存。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 非负整数，单位 KiB（系统内存字节数 / 1024）；要显示 MiB 再除以 1024。 Rust 返回类型：`Result<u64>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:jre|jre_get_max_memory' '{}'
```

源码位置：`apps/app/src/api/jre.rs:62`（相对源码根目录）。

<a id="cmd-jre-jre_test_jre"></a>

#### `plugin:jre|jre_test_jre` — 验证 Java 主版本

测试指定路径的 Java 是否适合要求的主版本，返回布尔值。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 待检测的本机 Java 可执行文件绝对路径，例如 D:/Java/bin/java.exe。 |
| `majorVersion` | 是 | u32 | 要求的 Java 主版本整数，例如 8、17、21；不是 Minecraft 版本号。 |

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file",
  "majorVersion": 17
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:jre|jre_test_jre' '@args.json'
```

源码位置：`apps/app/src/api/jre.rs:50`（相对源码根目录）。

<a id="cmd-jre-set_java_version"></a>

#### `plugin:jre|set_java_version` — 登记所选 Java

保存一个检测得到的完整 JavaVersion 配置，作为对应主版本的 Java；不能仅传数字 17。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `javaVersion` | 是 | JavaVersion | 完整 JavaVersion 对象；先调用 jre_get_jre 或 get_java_versions 获取，不要仅传整数。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "javaVersion": "<复制jre_get_jre返回的完整对象>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:jre|set_java_version' '@args.json'
```

源码位置：`apps/app/src/api/jre.rs:28`（相对源码根目录）。

<a id="api-logs"></a>

### 游戏日志

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`logs_clear_live_log_buffer`](#cmd-logs-logs_clear_live_log_buffer) | 清空实时日志缓冲 | 写入操作 |
| [`logs_delete_logs`](#cmd-logs-logs_delete_logs) | 删除游戏日志 | 删除／清理 |
| [`logs_delete_logs_by_filename`](#cmd-logs-logs_delete_logs_by_filename) | 删除指定日志 | 删除／清理 |
| [`logs_get_latest_log_cursor`](#cmd-logs-logs_get_latest_log_cursor) | 增量读取最新日志 | 本地查询 |
| [`logs_get_live_log_buffer`](#cmd-logs-logs_get_live_log_buffer) | 读取实时游戏日志 | 本地查询 |
| [`logs_get_logs`](#cmd-logs-logs_get_logs) | 列出游戏日志 | 本地查询 |
| [`logs_get_logs_by_filename`](#cmd-logs-logs_get_logs_by_filename) | 读取指定日志详情 | 本地查询 |
| [`logs_get_output_by_filename`](#cmd-logs-logs_get_output_by_filename) | 读取指定日志正文 | 本地查询 |
| [`logs_log_webview`](#cmd-logs-logs_log_webview) | 写入前端诊断日志 | 内部接口 |

<a id="cmd-logs-logs_clear_live_log_buffer"></a>

#### `plugin:logs|logs_clear_live_log_buffer` — 清空实时日志缓冲

清空目标游戏在启动器内的实时日志缓冲；不等同于删除磁盘日志文件。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_clear_live_log_buffer' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:122`（相对源码根目录）。

<a id="cmd-logs-logs_delete_logs"></a>

#### `plugin:logs|logs_delete_logs` — 删除游戏日志

删除指定游戏的日志文件；需要保留排错证据时应先导出。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_delete_logs' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:89`（相对源码根目录）。

<a id="cmd-logs-logs_delete_logs_by_filename"></a>

#### `plugin:logs|logs_delete_logs_by_filename` — 删除指定日志

按日志类型和文件名删除一个游戏日志或崩溃报告。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `logType` | 是 | LogType；可选值 InfoLog, CrashReport | 日志类型，严格使用 "InfoLog" 或 "CrashReport"。 |
| `filename` | 是 | String | 文件名称，不是可越出目标目录的任意绝对路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "logType": "InfoLog",
  "filename": "example.zip"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_delete_logs_by_filename' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:95`（相对源码根目录）。

<a id="cmd-logs-logs_get_latest_log_cursor"></a>

#### `plugin:logs|logs_get_latest_log_cursor` — 增量读取最新日志

从指定字节游标读取最新日志并返回新游标；首次用 0，后续使用上次返回值。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `cursor` | 是 | u64 | 日志字节游标；0 从开头读取，后续使用上次返回的新游标。 |

**返回：** 增量日志对象：output 为本次文本，cursor 为下一次读取游标，new_file 表示日志文件是否切换。 Rust 返回类型：`Result<LatestLogCursor>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "cursor": 0
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_get_latest_log_cursor' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:105`（相对源码根目录）。

<a id="cmd-logs-logs_get_live_log_buffer"></a>

#### `plugin:logs|logs_get_live_log_buffer` — 读取实时游戏日志

读取启动器内保存的当前游戏输出缓冲，并经过已有敏感内容遮蔽处理。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 经过日志遮蔽规则处理的文本字符串；仍应在分享前检查具体内容。 Rust 返回类型：`Result<CensoredString>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_get_live_log_buffer' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:114`（相对源码根目录）。

<a id="cmd-logs-logs_get_logs"></a>

#### `plugin:logs|logs_get_logs` — 列出游戏日志

读取游戏的日志记录列表；clearContents 控制返回内容是否清空，不是删除磁盘文件。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `clearContents` | 否 | Option<bool> | 是否省略返回日志记录中的正文，减少传输；不删除磁盘日志。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 Logs 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Logs>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "clearContents": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_get_logs' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:58`（相对源码根目录）。

<a id="cmd-logs-logs_get_logs_by_filename"></a>

#### `plugin:logs|logs_get_logs_by_filename` — 读取指定日志详情

根据类型和文件名读取一个日志记录及其信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `logType` | 是 | LogType；可选值 InfoLog, CrashReport | 日志类型，严格使用 "InfoLog" 或 "CrashReport"。 |
| `filename` | 是 | String | 文件名称，不是可越出目标目录的任意绝对路径。 |

**返回：** 序列化的 Logs 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Logs>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "logType": "InfoLog",
  "filename": "example.zip"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_get_logs_by_filename' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:69`（相对源码根目录）。

<a id="cmd-logs-logs_get_output_by_filename"></a>

#### `plugin:logs|logs_get_output_by_filename` — 读取指定日志正文

根据类型和文件名返回经过遮蔽处理的日志文本。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `logType` | 是 | LogType；可选值 InfoLog, CrashReport | 日志类型，严格使用 "InfoLog" 或 "CrashReport"。 |
| `filename` | 是 | String | 文件名称，不是可越出目标目录的任意绝对路径。 |

**返回：** 经过日志遮蔽规则处理的文本字符串；仍应在分享前检查具体内容。 Rust 返回类型：`Result<CensoredString>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "logType": "InfoLog",
  "filename": "example.zip"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_get_output_by_filename' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:79`（相对源码根目录）。

<a id="cmd-logs-logs_log_webview"></a>

#### `plugin:logs|logs_log_webview` — 写入前端诊断日志

按级别向启动器日志记录一条前端诊断消息；不会把消息发送到 Minecraft 控制台。

**影响与可用性：** 内部接口：供应用自身流程使用，一般业务自动化不应重复调用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `level` | 是 | &str | 日志级别字符串，例如 info、warn、error、debug 或 trace。 |
| `message` | 是 | &str | 写入诊断日志的文字；不要放密码、令牌或真实账户凭据。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "level": "info",
  "message": "Agent 测试日志"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:logs|logs_log_webview' '@args.json'
```

源码位置：`apps/app/src/api/logs.rs:33`（相对源码根目录）。

<a id="api-metadata"></a>

### 安装元数据

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`metadata_get_game_versions`](#cmd-metadata-metadata_get_game_versions) | 读取 Minecraft 版本清单 | 查询／缓存 |
| [`metadata_get_loader_versions`](#cmd-metadata-metadata_get_loader_versions) | 读取加载器版本清单 | 查询／缓存 |

<a id="cmd-metadata-metadata_get_game_versions"></a>

#### `plugin:metadata|metadata_get_game_versions` — 读取 Minecraft 版本清单

取得可用于安装的 Minecraft 版本元数据清单，可能读取网络缓存。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 序列化的 VersionManifest 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<VersionManifest>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:metadata|metadata_get_game_versions' '{}'
```

源码位置：`apps/app/src/api/metadata.rs:16`（相对源码根目录）。

<a id="cmd-metadata-metadata_get_loader_versions"></a>

#### `plugin:metadata|metadata_get_loader_versions` — 读取加载器版本清单

按 fabric、forge 等加载器名称取得可用版本及其元数据。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `loader` | 是 | &str | 加载器名称，例如 fabric、forge、quilt 或 neoforge；不是加载器版本号。 |

**返回：** 序列化的 Manifest 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Manifest>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "loader": "fabric"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:metadata|metadata_get_loader_versions' '@args.json'
```

源码位置：`apps/app/src/api/metadata.rs:22`（相对源码根目录）。

<a id="api-minecraft-skins"></a>

### 正版皮肤库

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`add_and_equip_custom_skin`](#cmd-minecraft-skins-add_and_equip_custom_skin) | 添加并使用自定义皮肤 | 写入操作 |
| [`equip_skin`](#cmd-minecraft-skins-equip_skin) | 应用已有皮肤 | 写入操作 |
| [`flush_pending_skin_change`](#cmd-minecraft-skins-flush_pending_skin_change) | 提交待应用皮肤 | 写入操作 |
| [`flush_pending_skin_change_for_profile`](#cmd-minecraft-skins-flush_pending_skin_change_for_profile) | 提交指定档案的皮肤 | 写入操作 |
| [`get_available_capes`](#cmd-minecraft-skins-get_available_capes) | 读取可用披风 | 查询／缓存 |
| [`get_available_skins`](#cmd-minecraft-skins-get_available_skins) | 读取可用皮肤 | 查询／缓存 |
| [`get_dragged_skin_data`](#cmd-minecraft-skins-get_dragged_skin_data) | 读取拖入的皮肤图片 | 本地查询 |
| [`normalize_skin_texture`](#cmd-minecraft-skins-normalize_skin_texture) | 规范化皮肤图片 | 查询／缓存 |
| [`remove_custom_skin`](#cmd-minecraft-skins-remove_custom_skin) | 移除自定义皮肤 | 删除／清理 |
| [`save_custom_skin`](#cmd-minecraft-skins-save_custom_skin) | 编辑自定义皮肤 | 写入操作 |
| [`set_custom_skin_order`](#cmd-minecraft-skins-set_custom_skin_order) | 调整自定义皮肤顺序 | 写入操作 |
| [`unequip_skin`](#cmd-minecraft-skins-unequip_skin) | 恢复默认皮肤 | 写入操作 |

<a id="cmd-minecraft-skins-add_and_equip_custom_skin"></a>

#### `plugin:minecraft-skins|add_and_equip_custom_skin` — 添加并使用自定义皮肤

保存上传的皮肤图片并为当前正版档案应用；离线账户并不因此获得所有版本可见的换肤功能。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `textureBlob` | 是 | Bytes | 皮肤 PNG 的原始字节数组（0–255）；应读取真实图片，不能用空数组当皮肤。 |
| `variant` | 是 | MinecraftSkinVariant；可选值 CLASSIC, SLIM, UNKNOWN | 皮肤体型：CLASSIC 宽手臂、SLIM 窄手臂；UNKNOWN 只用于兼容未知返回值。 |
| `cape` | 否 | Option<Cape> | 从可用披风列表取得的完整 Cape 对象；null 表示不指定披风。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 Skin 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Skin>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "textureBlob": "<按 Bytes 声明填写的完整值>",
  "variant": "CLASSIC",
  "cape": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|add_and_equip_custom_skin' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:47`（相对源码根目录）。

<a id="cmd-minecraft-skins-equip_skin"></a>

#### `plugin:minecraft-skins|equip_skin` — 应用已有皮肤

使用可用皮肤列表中返回的完整 Skin 对象切换当前正版档案皮肤。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `skin` | 是 | Skin | 从 get_available_skins 返回的完整 Skin 对象；不要用文件名或皮肤名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "skin": "<复制对应可用列表中返回的完整对象>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|equip_skin' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:62`（相对源码根目录）。

<a id="cmd-minecraft-skins-flush_pending_skin_change"></a>

#### `plugin:minecraft-skins|flush_pending_skin_change` — 提交待应用皮肤

提交当前档案排队的皮肤变更；用于皮肤页面内部处理待保存状态。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|flush_pending_skin_change' '{}'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:115`（相对源码根目录）。

<a id="cmd-minecraft-skins-flush_pending_skin_change_for_profile"></a>

#### `plugin:minecraft-skins|flush_pending_skin_change_for_profile` — 提交指定档案的皮肤

为给定 Minecraft 档案 UUID 提交排队中的皮肤变更，不是选择默认账户。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `profileId` | 是 | uuid::Uuid | Minecraft 档案的 UUID；不是 Modrinth 用户 ID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "profileId": "<Minecraft档案UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|flush_pending_skin_change_for_profile' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:123`（相对源码根目录）。

<a id="cmd-minecraft-skins-get_available_capes"></a>

#### `plugin:minecraft-skins|get_available_capes` — 读取可用披风

返回当前正版档案可用的披风信息；不会获得账户未拥有的披风。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 Cape 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Cape>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|get_available_capes' '{}'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:31`（相对源码根目录）。

<a id="cmd-minecraft-skins-get_available_skins"></a>

#### `plugin:minecraft-skins|get_available_skins` — 读取可用皮肤

返回当前档案可选皮肤及其完整 Skin 对象，供切换或编辑接口使用。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 Skin 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Skin>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|get_available_skins' '{}'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:39`（相对源码根目录）。

<a id="cmd-minecraft-skins-get_dragged_skin_data"></a>

#### `plugin:minecraft-skins|get_dragged_skin_data` — 读取拖入的皮肤图片

从本地路径读取皮肤图片字节，供后续规范化、保存和应用。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | String | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |

**返回：** 序列化的 Bytes 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Bytes>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|get_dragged_skin_data' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:144`（相对源码根目录）。

<a id="cmd-minecraft-skins-normalize_skin_texture"></a>

#### `plugin:minecraft-skins|normalize_skin_texture` — 规范化皮肤图片

把给定图片 URL 或字节解析为规范化皮肤图片字节；不会直接应用皮肤。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `texture` | 是 | UrlOrBlob | 皮肤图片 URL 字符串或原始字节数组；不是带 type 字段的对象。 |

**返回：** 序列化的 Bytes 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Bytes>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "texture": "https://example.com/skin.png"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|normalize_skin_texture' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:136`（相对源码根目录）。

<a id="cmd-minecraft-skins-remove_custom_skin"></a>

#### `plugin:minecraft-skins|remove_custom_skin` — 移除自定义皮肤

从启动器皮肤库移除指定自定义皮肤；传入列表中实际存在的 Skin 对象。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `skin` | 是 | Skin | 从 get_available_skins 返回的完整 Skin 对象；不要用文件名或皮肤名称代替。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "skin": "<复制对应可用列表中返回的完整对象>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|remove_custom_skin' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:70`（相对源码根目录）。

<a id="cmd-minecraft-skins-save_custom_skin"></a>

#### `plugin:minecraft-skins|save_custom_skin` — 编辑自定义皮肤

保存已有皮肤的图片、体型和披风设置；replaceTexture 决定是否替换纹理。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `skin` | 是 | Skin | 从 get_available_skins 返回的完整 Skin 对象；不要用文件名或皮肤名称代替。 |
| `textureBlob` | 是 | Bytes | 皮肤 PNG 的原始字节数组（0–255）；应读取真实图片，不能用空数组当皮肤。 |
| `variant` | 是 | MinecraftSkinVariant；可选值 CLASSIC, SLIM, UNKNOWN | 皮肤体型：CLASSIC 宽手臂、SLIM 窄手臂；UNKNOWN 只用于兼容未知返回值。 |
| `cape` | 否 | Option<Cape> | 从可用披风列表取得的完整 Cape 对象；null 表示不指定披风。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `replaceTexture` | 是 | bool | 是否替换自定义皮肤的纹理图片。 |

**返回：** 序列化的 Skin 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Skin>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "skin": "<复制对应可用列表中返回的完整对象>",
  "textureBlob": "<按 Bytes 声明填写的完整值>",
  "variant": "CLASSIC",
  "cape": null,
  "replaceTexture": false
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|save_custom_skin' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:78`（相对源码根目录）。

<a id="cmd-minecraft-skins-set_custom_skin_order"></a>

#### `plugin:minecraft-skins|set_custom_skin_order` — 调整自定义皮肤顺序

按皮肤纹理键数组保存皮肤库中的排列顺序。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `textureKeys` | 是 | Vec<String> | 按期望顺序排列的皮肤纹理键数组，从皮肤库取得。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "textureKeys": [
    "<对应ID或相对路径>"
  ]
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|set_custom_skin_order' '@args.json'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:99`（相对源码根目录）。

<a id="cmd-minecraft-skins-unequip_skin"></a>

#### `plugin:minecraft-skins|unequip_skin` — 恢复默认皮肤

取消当前自定义皮肤，恢复当前正版档案的默认皮肤状态。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:minecraft-skins|unequip_skin' '{}'
```

源码位置：`apps/app/src/api/minecraft_skins.rs:107`（相对源码根目录）。

<a id="api-mr-auth"></a>

### 已停用的平台账户接口

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`get`](#cmd-mr-auth-get) | 读取上游账户兼容占位 | 当前禁用／兼容占位 |
| [`get_all`](#cmd-mr-auth-get_all) | 读取上游账户列表兼容占位 | 当前禁用／兼容占位 |

<a id="cmd-mr-auth-get"></a>

#### `plugin:mr-auth|get` — 读取上游账户兼容占位

GeCL 已移除 Modrinth 账户登录，这个兼容接口固定返回 null。

**影响与可用性：** 当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。

**参数：** 无，传 `{}`。

**返回：** 固定为 null；当前构建没有 Modrinth 平台账户登录。 Rust 返回类型：`Result<Option<ModrinthCredentials>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:mr-auth|get' '{}'
```

源码位置：`apps/app/src/api/mr_auth.rs:13`（相对源码根目录）。

<a id="cmd-mr-auth-get_all"></a>

#### `plugin:mr-auth|get_all` — 读取上游账户列表兼容占位

GeCL 已移除 Modrinth 账户登录，这个兼容接口固定返回空数组。

**影响与可用性：** 当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。

**参数：** 无，传 `{}`。

**返回：** 固定为空数组 []。 Rust 返回类型：`Result<Vec<ModrinthCredentials>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:mr-auth|get_all' '{}'
```

源码位置：`apps/app/src/api/mr_auth.rs:18`（相对源码根目录）。

<a id="api-onboarding-checklist"></a>

### 引导状态

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`get_onboarding_checklist`](#cmd-onboarding-checklist-get_onboarding_checklist) | 读取引导完成状态 | 本地查询 |

<a id="cmd-onboarding-checklist-get_onboarding_checklist"></a>

#### `plugin:onboarding-checklist|get_onboarding_checklist` — 读取引导完成状态

返回原有新手清单的完成情况；不会重新开启首次设置弹窗。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 OnboardingChecklist 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<OnboardingChecklist>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:onboarding-checklist|get_onboarding_checklist' '{}'
```

源码位置：`apps/app/src/api/onboarding_checklist.rs:12`（相对源码根目录）。

<a id="api-process"></a>

### 游戏进程

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`process_get_all`](#cmd-process-process_get_all) | 列出启动器管理的进程 | 本地查询 |
| [`process_get_by_instance_id`](#cmd-process-process_get_by_instance_id) | 查询游戏关联进程 | 本地查询 |
| [`process_kill`](#cmd-process-process_kill) | 终止指定游戏进程 | 写入操作 |
| [`process_wait_for`](#cmd-process-process_wait_for) | 等待游戏进程结束 | 本地查询 |

<a id="cmd-process-process_get_all"></a>

#### `plugin:process|process_get_all` — 列出启动器管理的进程

返回当前管理的 Minecraft 进程及其进程记录 UUID；不列出 Windows 所有程序。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。 Rust 返回类型：`Result<Vec<ProcessMetadata>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:process|process_get_all' '{}'
```

源码位置：`apps/app/src/api/process.rs:17`（相对源码根目录）。

<a id="cmd-process-process_get_by_instance_id"></a>

#### `plugin:process|process_get_by_instance_id` — 查询游戏关联进程

返回指定游戏当前管理的进程记录。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 数组；每项为：启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。 Rust 返回类型：`Result<Vec<ProcessMetadata>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:process|process_get_by_instance_id' '@args.json'
```

源码位置：`apps/app/src/api/process.rs:22`（相对源码根目录）。

<a id="cmd-process-process_kill"></a>

#### `plugin:process|process_kill` — 终止指定游戏进程

按启动器进程记录 UUID 终止进程；不是传 Windows PID，可能丢失未保存游戏内容。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `uuid` | 是 | Uuid | 启动器进程记录 UUID，从 process_get_all 取得；不是 Windows 的数字 PID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "uuid": "<进程记录UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:process|process_kill' '@args.json'
```

源码位置：`apps/app/src/api/process.rs:29`（相对源码根目录）。

<a id="cmd-process-process_wait_for"></a>

#### `plugin:process|process_wait_for` — 等待游戏进程结束

等待给定进程记录退出后返回；等待超过 CLI 时限不会自动结束游戏。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `uuid` | 是 | Uuid | 启动器进程记录 UUID，从 process_get_all 取得；不是 Windows 的数字 PID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "uuid": "<进程记录UUID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:process|process_wait_for' '@args.json'
```

源码位置：`apps/app/src/api/process.rs:34`（相对源码根目录）。

<a id="api-reports"></a>

### 上游举报接口

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`reports_create`](#cmd-reports-reports_create) | 提交平台举报 | 上游服务写入 |

<a id="cmd-reports-reports_create"></a>

#### `plugin:reports|reports_create` — 提交平台举报

向上游平台提交举报请求，会产生服务端记录；不是生成本地错误报告。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `request` | 是 | CreateReportRequest | 此操作的结构化请求对象，嵌套字段和示例见本条说明与类型附录。 |

**返回：** 序列化的 CreateReportResponse 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<CreateReportResponse>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "request": "<按 CreateReportRequest 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:reports|reports_create' '@args.json'
```

源码位置：`apps/app/src/api/reports.rs:11`（相对源码根目录）。

<a id="api-settings"></a>

### 启动器设置与内容缓存

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`cancel_directory_change`](#cmd-settings-cancel_directory_change) | 取消数据目录迁移 | 写入操作 |
| [`download_sources_get`](#cmd-settings-download_sources_get) | 读取下载源策略 | 本地查询 |
| [`download_sources_set`](#cmd-settings-download_sources_set) | 修改下载源策略 | 写入操作 |
| [`settings_get`](#cmd-settings-settings_get) | 读取启动器完整设置 | 本地查询 |
| [`settings_set`](#cmd-settings-settings_set) | 保存启动器完整设置 | 写入操作 |
| [`store_cleanup`](#cmd-settings-store_cleanup) | 清理可回收缓存 | 删除／清理 |
| [`store_set_cache_limit`](#cmd-settings-store_set_cache_limit) | 设置存储缓存上限 | 写入操作 |
| [`store_usage`](#cmd-settings-store_usage) | 查询内容存储占用 | 本地查询 |
| [`store_verify`](#cmd-settings-store_verify) | 验证或修复内容存储 | 写入操作 |

<a id="cmd-settings-cancel_directory_change"></a>

#### `plugin:settings|cancel_directory_change` — 取消数据目录迁移

取消当前正在进行的应用数据目录迁移，按原流程处理恢复。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:settings|cancel_directory_change' '{}'
```

源码位置：`apps/app/src/api/settings.rs:76`（相对源码根目录）。

<a id="cmd-settings-download_sources_get"></a>

#### `plugin:settings|download_sources_get` — 读取下载源策略

读取官方源、镜像源的选择策略以及慢速切换阈值。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 theseus::download_sources::DownloadSourceSettings 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<theseus::download_sources::DownloadSourceSettings>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:settings|download_sources_get' '{}'
```

源码位置：`apps/app/src/api/settings.rs:85`（相对源码根目录）。

<a id="cmd-settings-download_sources_set"></a>

#### `plugin:settings|download_sources_set` — 修改下载源策略

保存官方优先、镜像优先或只用一种源的策略及慢速观察参数；不负责开关系统代理。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `settings` | 是 | theseus::download_sources::DownloadSourceSettings | 下载源对象（snake_case）：policy、slow_kib_per_second、observation_seconds；策略及范围见复杂参数配方。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "settings": {
    "policy": "official_preferred",
    "slow_kib_per_second": 128,
    "observation_seconds": 8
  }
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:settings|download_sources_set' '@args.json'
```

源码位置：`apps/app/src/api/settings.rs:90`（相对源码根目录）。

<a id="cmd-settings-settings_get"></a>

#### `plugin:settings|settings_get` — 读取启动器完整设置

返回启动器完整设置对象；修改设置前先读取。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 Settings 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Settings>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:settings|settings_get' '{}'
```

源码位置：`apps/app/src/api/settings.rs:62`（相对源码根目录）。

<a id="cmd-settings-settings_set"></a>

#### `plugin:settings|settings_set` — 保存启动器完整设置

校验并保存完整 Settings 对象；应读取后仅修改目标字段，再提交完整对象，避免重置其他设置。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `settings` | 是 | Settings | settings_get 返回的完整 Settings 对象，保留所有其他字段后只改目标字段。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "settings": "<复制settings_get返回的完整对象并修改目标字段>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:settings|settings_set' '@args.json'
```

源码位置：`apps/app/src/api/settings.rs:70`（相对源码根目录）。

<a id="cmd-settings-store_cleanup"></a>

#### `plugin:settings|store_cleanup` — 清理可回收缓存

清理内容存储中符合回收条件的缓存，返回释放的字节数；不是删除全部游戏。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

**参数：** 无，传 `{}`。

**返回：** 非负整数；大小、容量、索引的单位以本条用途及参数说明为准。 Rust 返回类型：`Result<u64>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:settings|store_cleanup' '{}'
```

源码位置：`apps/app/src/api/settings.rs:27`（相对源码根目录）。

<a id="cmd-settings-store_set_cache_limit"></a>

#### `plugin:settings|store_set_cache_limit` — 设置存储缓存上限

按字节数设置内容缓存容量上限；不会改变游戏内存设置。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `bytes` | 是 | u64 | 字节数据或容量，具体取决于此接口的 Rust 类型；整数是字节数，数组是 0–255 文件字节。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "bytes": 104857600
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:settings|store_set_cache_limit' '@args.json'
```

源码位置：`apps/app/src/api/settings.rs:32`（相对源码根目录）。

<a id="cmd-settings-store_usage"></a>

#### `plugin:settings|store_usage` — 查询内容存储占用

返回内容存储的占用、缓存及可回收等统计。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 内容存储的容量和缓存使用统计，各字段名及字节单位见类型附录。 Rust 返回类型：`Result<settings::StoreUsage>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:settings|store_usage' '{}'
```

源码位置：`apps/app/src/api/settings.rs:22`（相对源码根目录）。

<a id="cmd-settings-store_verify"></a>

#### `plugin:settings|store_verify` — 验证或修复内容存储

检查存储文件的完整性；repair 为 false 只检查，为 true 时执行允许的修复，进度通过通道返回。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `repair` | 是 | bool | false 只验证存储完整性；true 还执行允许的修复。 |
| `onProgress` | 是 | tauri::ipc::Channel<(u64, u64)> | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 存储完整性检查／修复统计；进度事件与最终统计分别读取。 Rust 返回类型：`Result<settings::StoreVerification>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "repair": false,
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:settings|store_verify' '@args.json'
```

源码位置：`apps/app/src/api/settings.rs:37`（相对源码根目录）。

<a id="api-shortcuts"></a>

### 系统快捷方式

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`create_instance_shortcut`](#cmd-shortcuts-create_instance_shortcut) | 创建游戏启动快捷方式 | 写入操作 |

<a id="cmd-shortcuts-create_instance_shortcut"></a>

#### `plugin:shortcuts|create_instance_shortcut` — 创建游戏启动快捷方式

在指定输出位置生成启动目标游戏的系统快捷方式，可附带直接加入服务器或单人世界的参数。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceName` | 是 | String | 快捷方式显示名称，不决定实际游戏 ID。 |
| `instanceId` | 是 | String | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `outputPath` | 是 | PathBuf | 快捷方式输出的绝对路径；Windows 通常使用 .lnk 文件路径。 |
| `server` | 否 | Option<String> | 快捷方式的服务器地址；null 不设置自动加入服务器。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `singleplayerWorld` | 否 | Option<String> | 快捷方式要直接进入的 saves 下世界文件夹名；null 不指定。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<PathBuf>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceName": "测试游戏",
  "instanceId": "<游戏ID>",
  "outputPath": "D:/Desktop/游戏.lnk",
  "server": null,
  "singleplayerWorld": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:shortcuts|create_instance_shortcut' '@args.json'
```

源码位置：`apps/app/src/api/shortcuts/mod.rs:28`（相对源码根目录）。

<a id="api-tags"></a>

### 搜索筛选标签

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`tags_get_categories`](#cmd-tags-tags_get_categories) | 读取项目分类 | 查询／缓存 |
| [`tags_get_donation_platforms`](#cmd-tags-tags_get_donation_platforms) | 读取捐赠平台标签 | 查询／缓存 |
| [`tags_get_game_versions`](#cmd-tags-tags_get_game_versions) | 读取搜索用游戏版本标签 | 查询／缓存 |
| [`tags_get_loaders`](#cmd-tags-tags_get_loaders) | 读取加载器标签 | 查询／缓存 |
| [`tags_get_report_types`](#cmd-tags-tags_get_report_types) | 读取举报类型 | 查询／缓存 |

<a id="cmd-tags-tags_get_categories"></a>

#### `plugin:tags|tags_get_categories` — 读取项目分类

返回模组、资源包等内容的分类标签，供搜索筛选使用。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 Category 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Category>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:tags|tags_get_categories' '{}'
```

源码位置：`apps/app/src/api/tags.rs:18`（相对源码根目录）。

<a id="cmd-tags-tags_get_donation_platforms"></a>

#### `plugin:tags|tags_get_donation_platforms` — 读取捐赠平台标签

返回上游支持的捐赠平台定义；不会发起付款。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 DonationPlatform 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<DonationPlatform>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:tags|tags_get_donation_platforms' '{}'
```

源码位置：`apps/app/src/api/tags.rs:42`（相对源码根目录）。

<a id="cmd-tags-tags_get_game_versions"></a>

#### `plugin:tags|tags_get_game_versions` — 读取搜索用游戏版本标签

返回内容搜索使用的 Minecraft 版本标签；不是完整游戏安装元数据。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 GameVersion 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<GameVersion>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:tags|tags_get_game_versions' '{}'
```

源码位置：`apps/app/src/api/tags.rs:36`（相对源码根目录）。

<a id="cmd-tags-tags_get_loaders"></a>

#### `plugin:tags|tags_get_loaders` — 读取加载器标签

返回内容搜索的加载器标签和相关项目类型信息。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：序列化的 Loader 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<Loader>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:tags|tags_get_loaders' '{}'
```

源码位置：`apps/app/src/api/tags.rs:30`（相对源码根目录）。

<a id="cmd-tags-tags_get_report_types"></a>

#### `plugin:tags|tags_get_report_types` — 读取举报类型

返回上游允许的举报类型名称；不会提交举报。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<Vec<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:tags|tags_get_report_types' '{}'
```

源码位置：`apps/app/src/api/tags.rs:24`（相对源码根目录）。

<a id="api-users"></a>

### 平台用户与作者

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`block_user`](#cmd-users-block_user) | 屏蔽平台用户 | 上游服务写入 |
| [`change_user_avatar`](#cmd-users-change_user_avatar) | 修改平台用户头像 | 上游服务写入 |
| [`delete_user_avatar`](#cmd-users-delete_user_avatar) | 删除平台用户头像 | 上游服务写入 |
| [`get_blocked_users`](#cmd-users-get_blocked_users) | 读取平台屏蔽名单 | 上游服务查询 |
| [`get_user_collections`](#cmd-users-get_user_collections) | 读取平台用户收藏集 | 查询／缓存 |
| [`get_user_organizations`](#cmd-users-get_user_organizations) | 读取平台用户组织 | 查询／缓存 |
| [`get_user_preferences`](#cmd-users-get_user_preferences) | 读取平台用户偏好 | 上游服务查询 |
| [`get_user_profile`](#cmd-users-get_user_profile) | 读取平台作者资料 | 查询／缓存 |
| [`get_user_projects`](#cmd-users-get_user_projects) | 读取平台作者项目 | 查询／缓存 |
| [`patch_user`](#cmd-users-patch_user) | 修改平台用户资料 | 上游服务写入 |
| [`patch_user_preferences`](#cmd-users-patch_user_preferences) | 修改平台用户偏好 | 上游服务写入 |
| [`search_user`](#cmd-users-search_user) | 搜索平台用户 | 查询／缓存 |
| [`unblock_user`](#cmd-users-unblock_user) | 取消屏蔽平台用户 | 上游服务写入 |

<a id="cmd-users-block_user"></a>

#### `plugin:users|block_user` — 屏蔽平台用户

在上游账户关系中屏蔽指定用户，需要上游账户权限；不是删除本地游玩账户。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|block_user' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:66`（相对源码根目录）。

<a id="cmd-users-change_user_avatar"></a>

#### `plugin:users|change_user_avatar` — 修改平台用户头像

向上游上传头像图片，需要对应用户的编辑权限；不是修改 Minecraft 皮肤。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |
| `image` | 是 | Vec<u8> | 头像图片原始字节数组，每项 0–255；不是图片文件路径。 |
| `extension` | 是 | &str | 上传头像的图片扩展名，不含路径；应与实际图片格式一致。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>",
  "image": "<从实际文件读取的0到255字节数组>",
  "extension": "png"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|change_user_avatar' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:49`（相对源码根目录）。

<a id="cmd-users-delete_user_avatar"></a>

#### `plugin:users|delete_user_avatar` — 删除平台用户头像

删除上游用户的自定义头像，需要账户权限。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|delete_user_avatar' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:61`（相对源码根目录）。

<a id="cmd-users-get_blocked_users"></a>

#### `plugin:users|get_blocked_users` — 读取平台屏蔽名单

返回当前上游账户屏蔽的用户 ID；GeCL 的离线账户不能提供该授权。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

**参数：** 无，传 `{}`。

**返回：** 数组；每项为：字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。 Rust 返回类型：`Result<Vec<String>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_blocked_users' '{}'
```

源码位置：`apps/app/src/api/users.rs:76`（相对源码根目录）。

<a id="cmd-users-get_user_collections"></a>

#### `plugin:users|get_user_collections` — 读取平台用户收藏集

查询指定 Modrinth 用户的收藏集；能读取的范围由平台权限决定。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_user_collections' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:26`（相对源码根目录）。

<a id="cmd-users-get_user_organizations"></a>

#### `plugin:users|get_user_organizations` — 读取平台用户组织

查询指定 Modrinth 用户所属组织。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_user_organizations' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:21`（相对源码根目录）。

<a id="cmd-users-get_user_preferences"></a>

#### `plugin:users|get_user_preferences` — 读取平台用户偏好

查询上游账户偏好，需要对应的上游权限；不是启动器设置。

**影响与可用性：** 上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_user_preferences' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:31`（相对源码根目录）。

<a id="cmd-users-get_user_profile"></a>

#### `plugin:users|get_user_profile` — 读取平台作者资料

查询指定 Modrinth 用户或作者的公开资料。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_user_profile' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:11`（相对源码根目录）。

<a id="cmd-users-get_user_projects"></a>

#### `plugin:users|get_user_projects` — 读取平台作者项目

查询指定 Modrinth 作者的项目列表。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|get_user_projects' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:16`（相对源码根目录）。

<a id="cmd-users-patch_user"></a>

#### `plugin:users|patch_user` — 修改平台用户资料

按平台允许的补丁字段修改远程用户资料，需要授权。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |
| `patch` | 是 | Value | 上游用户资料的补丁 JSON 对象；只能使用服务端允许编辑的字段。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>",
  "patch": "<按 Value 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|patch_user' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:44`（相对源码根目录）。

<a id="cmd-users-patch_user_preferences"></a>

#### `plugin:users|patch_user_preferences` — 修改平台用户偏好

修改远程账户偏好设置，需要上游账户权限。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |
| `preferences` | 是 | Value | 外观设置或上游偏好对象，按此接口的类型及下方配方填写。 |

**返回：** 序列化的 Value 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Value>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>",
  "preferences": "<按 Value 声明填写的完整值>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|patch_user_preferences' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:36`（相对源码根目录）。

<a id="cmd-users-search_user"></a>

#### `plugin:users|search_user` — 搜索平台用户

按查询词搜索 Modrinth 用户；不是搜索本地离线账户。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `query` | 是 | &str | 搜索词或完整查询串，依此接口用途确定；不会在 Minecraft 中执行。 |

**返回：** 数组；每项为：序列化的 SearchUser 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<SearchUser>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "query": "AppleSkin"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|search_user' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:6`（相对源码根目录）。

<a id="cmd-users-unblock_user"></a>

#### `plugin:users|unblock_user` — 取消屏蔽平台用户

在上游账户关系中解除屏蔽，需要上游账户权限。

**影响与可用性：** 上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `userId` | 是 | &str | Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "userId": "<平台用户ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:users|unblock_user' '@args.json'
```

源码位置：`apps/app/src/api/users.rs:71`（相对源码根目录）。

<a id="api-utils"></a>

### 系统工具与诊断

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`cancel_debug_info_export`](#cmd-utils-cancel_debug_info_export) | 取消诊断包导出 | 写入操作 |
| [`export_debug_info`](#cmd-utils-export_debug_info) | 导出诊断信息包 | 写入操作 |
| [`get_image_thumbnail`](#cmd-utils-get_image_thumbnail) | 读取图片缩略图 | 本地查询 |
| [`get_opening_command`](#cmd-utils-get_opening_command) | 读取启动时的操作载荷 | 本地查询 |
| [`get_os`](#cmd-utils-get_os) | 读取操作系统信息 | 本地查询 |
| [`highlight_in_folder`](#cmd-utils-highlight_in_folder) | 在文件管理器中定位 | 界面／系统打开 |
| [`is_network_metered`](#cmd-utils-is_network_metered) | 检测按流量计费网络 | 本地查询 |
| [`open_path`](#cmd-utils-open_path) | 用系统程序打开路径 | 界面／系统打开 |
| [`progress_bars_list`](#cmd-utils-progress_bars_list) | 读取通用进度条 | 本地查询 |
| [`should_disable_mouseover`](#cmd-utils-should_disable_mouseover) | 读取悬停兼容标志 | 内部接口 |
| [`show_app_db_backups_folder`](#cmd-utils-show_app_db_backups_folder) | 打开数据库备份目录 | 界面／系统打开 |
| [`show_launcher_logs_folder`](#cmd-utils-show_launcher_logs_folder) | 打开启动器日志目录 | 界面／系统打开 |

<a id="cmd-utils-cancel_debug_info_export"></a>

#### `plugin:utils|cancel_debug_info_export` — 取消诊断包导出

按导出任务 ID 请求取消正在进行的诊断信息打包；不会删除游戏。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 调用者生成并保留的诊断导出任务 ID，取消时须使用同一个值。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:utils|cancel_debug_info_export' '@args.json'
```

源码位置：`apps/app/src/api/utils.rs:191`（相对源码根目录）。

<a id="cmd-utils-export_debug_info"></a>

#### `plugin:utils|export_debug_info` — 导出诊断信息包

把启动器诊断信息写入指定本地路径并提供进度；返回是否完成，不会主动上传。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `id` | 是 | String | 调用者生成并保留的诊断导出任务 ID，取消时须使用同一个值。 |
| `path` | 是 | PathBuf | 诊断 ZIP 的绝对输出文件路径，例如 D:/Reports/gecl.zip。 |
| `onProgress` | 是 | tauri::ipc::Channel< theseus::debug_info::DebugInfoExportProgress, > | 进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。 |

**返回：** 布尔值，表示诊断导出是否完成；取消与失败应结合实际返回和错误判断。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "id": "<对象ID>",
  "path": "D:/Reports/gecl.zip",
  "onProgress": "$channel"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:utils|export_debug_info' '@args.json'
```

源码位置：`apps/app/src/api/utils.rs:159`（相对源码根目录）。

<a id="cmd-utils-get_image_thumbnail"></a>

#### `plugin:utils|get_image_thumbnail` — 读取图片缩略图

从允许访问的本地图片生成或读取缩略图，返回前端可用的图片地址；尺寸限制在 32 到 512。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |
| `size` | 是 | u32 | 缩略图目标边长，单位像素；后端限制在 32–512 范围。 |

**返回：** 前端可加载的缩略图地址字符串；不是原图片字节。 Rust 返回类型：`Result<String>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "<允许访问的本地图片绝对路径>",
  "size": 128
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:utils|get_image_thumbnail' '@args.json'
```

源码位置：`apps/app/src/api/thumbnails.rs:17`（相对源码根目录）。

<a id="cmd-utils-get_opening_command"></a>

#### `plugin:utils|get_opening_command` — 读取启动时的操作载荷

读取这次启动的深链接或打开文件载荷；不代表执行任意命令行。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 CommandPayload 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<CommandPayload>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|get_opening_command' '{}'
```

源码位置：`apps/app/src/api/utils.rs:207`（相对源码根目录）。

<a id="cmd-utils-get_os"></a>

#### `plugin:utils|get_os` — 读取操作系统信息

返回当前运行平台信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 OS 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`OS`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|get_os' '{}'
```

源码位置：`apps/app/src/api/utils.rs:66`（相对源码根目录）。

<a id="cmd-utils-highlight_in_folder"></a>

#### `plugin:utils|highlight_in_folder` — 在文件管理器中定位

打开文件所在位置并尝试选中指定文件。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:utils|highlight_in_folder' '@args.json'
```

源码位置：`apps/app/src/api/utils.rs:112`（相对源码根目录）。

<a id="cmd-utils-is_network_metered"></a>

#### `plugin:utils|is_network_metered` — 检测按流量计费网络

返回系统报告的当前网络是否按流量计费；不是测速或检查代理。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`Result<bool>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|is_network_metered' '{}'
```

源码位置：`apps/app/src/api/utils.rs:77`（相对源码根目录）。

<a id="cmd-utils-open_path"></a>

#### `plugin:utils|open_path` — 用系统程序打开路径

按系统关联打开文件或目录；不接受脚本代码作为操作参数。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `path` | 是 | PathBuf | 本机目标文件或目录的绝对路径，操作对象以本条用途为准。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "path": "D:/Example/file"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:utils|open_path' '@args.json'
```

源码位置：`apps/app/src/api/utils.rs:126`（相对源码根目录）。

<a id="cmd-utils-progress_bars_list"></a>

#### `plugin:utils|progress_bars_list` — 读取通用进度条

返回启动器当前通用加载进度条映射；安装任务另有 install_job_list 接口。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

**参数：** 无，传 `{}`。

**返回：** 序列化的 DashMap<uuid::Uuid, theseus::LoadingBar> 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<DashMap<uuid::Uuid, theseus::LoadingBar>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|progress_bars_list' '{}'
```

源码位置：`apps/app/src/api/utils.rs:85`（相对源码根目录）。

<a id="cmd-utils-should_disable_mouseover"></a>

#### `plugin:utils|should_disable_mouseover` — 读取悬停兼容标志

返回当前平台是否需禁用部分悬停效果，用于前端兼容逻辑。

**影响与可用性：** 内部接口：供应用自身流程使用，一般业务自动化不应重复调用。

**参数：** 无，传 `{}`。

**返回：** 布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。 Rust 返回类型：`bool`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|should_disable_mouseover' '{}'
```

源码位置：`apps/app/src/api/utils.rs:93`（相对源码根目录）。

<a id="cmd-utils-show_app_db_backups_folder"></a>

#### `plugin:utils|show_app_db_backups_folder` — 打开数据库备份目录

在文件管理器打开启动器数据库备份所在目录。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|show_app_db_backups_folder' '{}'
```

源码位置：`apps/app/src/api/utils.rs:149`（相对源码根目录）。

<a id="cmd-utils-show_launcher_logs_folder"></a>

#### `plugin:utils|show_launcher_logs_folder` — 打开启动器日志目录

在文件管理器打开启动器自己的日志目录，不是某个游戏的日志目录。

**影响与可用性：** 界面／系统打开：可能打开窗口或外部关联程序。

**参数：** 无，传 `{}`。

**返回：** 成功时 result 为 null；失败时返回统一 error。没有额外结果对象。 Rust 返回类型：`()`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

```powershell
.\gecl-agent.exe invoke 'plugin:utils|show_launcher_logs_folder' '{}'
```

源码位置：`apps/app/src/api/utils.rs:139`（相对源码根目录）。

<a id="api-worlds"></a>

### 世界与多人服务器

| 指令 | 作用 | 状态／影响 |
|---|---|---|
| [`add_server_to_instance`](#cmd-worlds-add_server_to_instance) | 添加多人服务器 | 写入操作 |
| [`backup_world`](#cmd-worlds-backup_world) | 备份世界到游戏备份目录 | 写入操作 |
| [`delete_world`](#cmd-worlds-delete_world) | 删除单人世界 | 删除／清理 |
| [`desync_server`](#cmd-worlds-desync_server) | 取消服务器条目同步 | 写入操作 |
| [`edit_server_in_instance`](#cmd-worlds-edit_server_in_instance) | 修改多人服务器条目 | 写入操作 |
| [`ensure_managed_server_in_instance`](#cmd-worlds-ensure_managed_server_in_instance) | 确保服务器条目存在 | 写入操作 |
| [`get_instance_protocol_version`](#cmd-worlds-get_instance_protocol_version) | 读取游戏网络协议版本 | 本地查询 |
| [`get_instance_worlds`](#cmd-worlds-get_instance_worlds) | 列出游戏世界和服务器 | 本地查询 |
| [`get_recent_worlds`](#cmd-worlds-get_recent_worlds) | 列出最近世界 | 本地查询 |
| [`get_server_status`](#cmd-worlds-get_server_status) | 查询 Minecraft 服务器状态 | 查询／缓存 |
| [`get_singleplayer_world`](#cmd-worlds-get_singleplayer_world) | 读取一个单人世界 | 本地查询 |
| [`remove_server_from_instance`](#cmd-worlds-remove_server_from_instance) | 移除多人服务器条目 | 删除／清理 |
| [`rename_world`](#cmd-worlds-rename_world) | 修改世界显示名称 | 写入操作 |
| [`reset_world_icon`](#cmd-worlds-reset_world_icon) | 重置世界图标 | 写入操作 |
| [`set_world_display_status`](#cmd-worlds-set_world_display_status) | 设置世界显示状态 | 写入操作 |
| [`start_join_server`](#cmd-worlds-start_join_server) | 启动并加入服务器 | 写入操作 |
| [`start_join_singleplayer_world`](#cmd-worlds-start_join_singleplayer_world) | 启动并进入单人世界 | 写入操作 |

<a id="cmd-worlds-add_server_to_instance"></a>

#### `plugin:worlds|add_server_to_instance` — 添加多人服务器

在目标游戏的 servers.dat 添加一个服务器条目并返回其索引；不安装或启动服务端。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |
| `address` | 是 | String | Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。 |
| `packStatus` | 是 | ServerPackStatus；可选值 enabled, disabled, prompt | 服务器资源包策略：enabled 接受、disabled 拒绝、prompt 询问。 |
| `projectId` | 否 | Option<String> | 平台项目 ID；Modrinth 是字符串，CurseForge 是数字，不能互换。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |
| `contentKind` | 否 | Option<String> | 服务器关联内容类型标识；普通自定义服务器可为 null。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 非负整数；大小、容量、索引的单位以本条用途及参数说明为准。 Rust 返回类型：`Result<usize>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "name": "测试名称",
  "address": "play.example.com:25565",
  "packStatus": "enabled",
  "projectId": null,
  "contentKind": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|add_server_to_instance' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:151`（相对源码根目录）。

<a id="cmd-worlds-backup_world"></a>

#### `plugin:worlds|backup_world` — 备份世界到游戏备份目录

旧版备份接口：把世界打包到游戏的 backups 目录并返回大小；指定位置及恢复请用 gecl 的 world_backup 接口。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `world` | 是 | &str | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |

**返回：** 旧版备份生成的 ZIP 文件大小，单位字节；备份保存在该游戏的 backups 下。 Rust 返回类型：`Result<u64>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "world": "<世界文件夹名>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|backup_world' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:138`（相对源码根目录）。

<a id="cmd-worlds-delete_world"></a>

#### `plugin:worlds|delete_world` — 删除单人世界

删除指定游戏 saves 中的世界文件夹；world 必须是世界文件夹名，不是显示名称。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `world` | 是 | &str | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "world": "<世界文件夹名>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|delete_world' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:144`（相对源码根目录）。

<a id="cmd-worlds-desync_server"></a>

#### `plugin:worlds|desync_server` — 取消服务器条目同步

解除游戏中某个同步服务器条目的同步，按 mode 保留或移除本地条目。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `serverId` | 是 | &str | 全局同步服务器的内部 ID，不是服务器 IP 地址。 |
| `mode` | 是 | theseus::instance::DesyncServerMode；可选值 keep_in_other_instances, remove_from_other_instances | keep_in_other_instances 让其他游戏保留自己的副本；remove_from_other_instances 从其他游戏移除对应同步项。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "serverId": "<serverId>",
  "mode": "keep_in_other_instances"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|desync_server' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:211`（相对源码根目录）。

<a id="cmd-worlds-edit_server_in_instance"></a>

#### `plugin:worlds|edit_server_in_instance` — 修改多人服务器条目

按服务器列表索引修改名称、地址与服务器资源包策略；不会编辑远程服务器配置。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `index` | 是 | usize | 目标列表下标，从 0 开始；使用刚查询的列表，避免删除或编辑错项。 |
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |
| `address` | 是 | String | Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。 |
| `packStatus` | 是 | ServerPackStatus；可选值 enabled, disabled, prompt | 服务器资源包策略：enabled 接受、disabled 拒绝、prompt 询问。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "index": 0,
  "name": "测试名称",
  "address": "play.example.com:25565",
  "packStatus": "enabled"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|edit_server_in_instance' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:183`（相对源码根目录）。

<a id="cmd-worlds-ensure_managed_server_in_instance"></a>

#### `plugin:worlds|ensure_managed_server_in_instance` — 确保服务器条目存在

按给定名称与地址创建或更新受管理的游戏内服务器条目。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `name` | 是 | String | 新建对象或条目的显示名称，不能代替内部 ID。 |
| `address` | 是 | String | Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "name": "测试名称",
  "address": "play.example.com:25565"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|ensure_managed_server_in_instance' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:171`（相对源码根目录）。

<a id="cmd-worlds-get_instance_protocol_version"></a>

#### `plugin:worlds|get_instance_protocol_version` — 读取游戏网络协议版本

返回游戏所用 Minecraft 协议版本，无法识别时为空，用于查询服务器状态。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 序列化的 ProtocolVersion 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 找不到或没有对应结果时为 null。 Rust 返回类型：`Result<Option<ProtocolVersion>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|get_instance_protocol_version' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:220`（相对源码根目录）。

<a id="cmd-worlds-get_instance_worlds"></a>

#### `plugin:worlds|get_instance_worlds` — 列出游戏世界和服务器

读取目标游戏的本地世界及多人服务器列表，包含显示状态与相关元数据。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |

**返回：** 数组；每项为：序列化的 World 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<World>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|get_instance_worlds' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:56`（相对源码根目录）。

<a id="cmd-worlds-get_recent_worlds"></a>

#### `plugin:worlds|get_recent_worlds` — 列出最近世界

按数量及显示状态筛选最近访问的世界记录，附带所属游戏信息。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `limit` | 是 | usize | 最多返回的记录数量。 |
| `displayStatuses` | 否 | Option<EnumSet<DisplayStatus>>；数组项 normal, hidden, favorite | 显示状态筛选数组，例如 ["normal","favorite"]；null 使用接口默认范围。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 数组；每项为：序列化的 WorldWithInstance 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<Vec<WorldWithInstance>>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "limit": 10,
  "displayStatuses": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|get_recent_worlds' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:39`（相对源码根目录）。

<a id="cmd-worlds-get_server_status"></a>

#### `plugin:worlds|get_server_status` — 查询 Minecraft 服务器状态

按地址发送状态查询，返回在线信息等；不会加入服务器或下载整合包。

**影响与可用性：** 查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `address` | 是 | &str | Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。 |
| `protocolVersion` | 否 | Option<ProtocolVersion> | Minecraft 网络协议版本，从协议查询取得；省略可按服务端查询逻辑处理。 此顶层参数允许省略或传 null；其业务差异以本条用途为准。 |

**返回：** 序列化的 ServerStatus 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<ServerStatus>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "address": "play.example.com:25565",
  "protocolVersion": null
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|get_server_status' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:227`（相对源码根目录）。

<a id="cmd-worlds-get_singleplayer_world"></a>

#### `plugin:worlds|get_singleplayer_world` — 读取一个单人世界

按游戏内部 ID 和 saves 下的文件夹名读取该世界元数据。

**影响与可用性：** 本地查询：不提交业务数据变更；部分读取可能维护缓存。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `world` | 是 | &str | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |

**返回：** 序列化的 World 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。 Rust 返回类型：`Result<World>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "world": "<世界文件夹名>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|get_singleplayer_world' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:68`（相对源码根目录）。

<a id="cmd-worlds-remove_server_from_instance"></a>

#### `plugin:worlds|remove_server_from_instance` — 移除多人服务器条目

按列表索引从 servers.dat 移除服务器条目；不会删除远程服务器。

**影响与可用性：** 删除／清理：会删除本条说明的目标，不能把它当作只读查询。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `index` | 是 | usize | 目标列表下标，从 0 开始；使用刚查询的列表，避免删除或编辑错项。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "index": 0
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|remove_server_from_instance' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:202`（相对源码根目录）。

<a id="cmd-worlds-rename_world"></a>

#### `plugin:worlds|rename_world` — 修改世界显示名称

修改世界元数据中的显示名称，不应把 newName 当作更换世界文件夹路径。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `world` | 是 | &str | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |
| `newName` | 是 | &str | 新的显示名称；世界重命名只修改世界显示名称。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "world": "<世界文件夹名>",
  "newName": "新名称"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|rename_world' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:120`（相对源码根目录）。

<a id="cmd-worlds-reset_world_icon"></a>

#### `plugin:worlds|reset_world_icon` — 重置世界图标

删除或重置该世界自定义图标，让游戏重新生成；不删除世界数据。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `world` | 是 | &str | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "world": "<世界文件夹名>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|reset_world_icon' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:131`（相对源码根目录）。

<a id="cmd-worlds-set_world_display_status"></a>

#### `plugin:worlds|set_world_display_status` — 设置世界显示状态

将世界或服务器标为普通、隐藏或收藏；只影响启动器显示。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instance` | 是 | &str | 内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。 |
| `worldType` | 是 | WorldType；可选值 singleplayer, server | 世界类型：singleplayer 单人世界或 server 多人服务器。 |
| `worldId` | 是 | &str | worldType 对应的世界文件夹名或服务器标识，从世界列表取得。 |
| `displayStatus` | 是 | DisplayStatus；可选值 normal, hidden, favorite | 显示状态：normal 普通、hidden 隐藏、favorite 收藏。 |

**返回：** 成功时 result 为 null；失败时返回统一 error。 Rust 返回类型：`Result<()>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instance": "<游戏ID>",
  "worldType": "singleplayer",
  "worldId": "<世界文件夹名>",
  "displayStatus": "normal"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|set_world_display_status' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:104`（相对源码根目录）。

<a id="cmd-worlds-start_join_server"></a>

#### `plugin:worlds|start_join_server` — 启动并加入服务器

启动目标游戏并传入服务器地址，进入后尝试加入该服务器。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `address` | 是 | &str | Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。 |

**返回：** 启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。 Rust 返回类型：`Result<ProcessMetadata>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "address": "play.example.com:25565"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|start_join_server' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:246`（相对源码根目录）。

<a id="cmd-worlds-start_join_singleplayer_world"></a>

#### `plugin:worlds|start_join_singleplayer_world` — 启动并进入单人世界

启动目标游戏并传入指定单人世界，按版本支持情况进入世界。

**影响与可用性：** 写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。

| 参数 | 必填 | 类型／取值 | 具体含义 |
|---|---|---|---|
| `instanceId` | 是 | &str | 启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。 |
| `world` | 是 | String | saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。 |

**返回：** 启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。 Rust 返回类型：`Result<ProcessMetadata>`。

调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。

把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：

```json
{
  "instanceId": "<游戏ID>",
  "world": "<世界文件夹名>"
}
```

```powershell
.\gecl-agent.exe invoke 'plugin:worlds|start_join_singleplayer_world' '@args.json'
```

源码位置：`apps/app/src/api/worlds.rs:235`（相对源码根目录）。

<!-- END GENERATED COMMANDS -->
