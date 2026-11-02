"""Build the application Agent reference from registrations plus reviewed Chinese help.

This is a source catalog, not a runtime capability probe. Complex Rust types retain
an explicit unresolved-schema marker rather than pretending to be fully validated.
"""
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HELP = json.loads((ROOT / 'docs/AGENT_COMMAND_HELP.json').read_text(encoding='utf-8'))


def split_args(value):
    values, current, depth = [], '', 0
    for char in value:
        if char == ',' and depth == 0:
            values.append(current.strip())
            current = ''
            continue
        current += char
        if char in '<([':
            depth += 1
        elif char in '>)]':
            depth -= 1
    if current.strip():
        values.append(current.strip())
    return values


def uncomment(value):
    return re.sub(r'//[^\n]*', '', value)


def registered_names(source):
    handlers = re.search(r'generate_handler!\[(.*?)\]', source, re.S)
    if not handlers:
        return []
    names = []
    for value in uncomment(handlers[1]).split(','):
        value = value.strip()
        if not value:
            continue
        if not re.fullmatch(r'(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*', value):
            raise ValueError('Unsupported handler registration: ' + value)
        names.append(value.split('::')[-1])
    return names


def signature_from(name, paths):
    for path in paths:
        source = path.read_text(encoding='utf-8')
        match = re.search(r'\bfn\s+' + re.escape(name) + r'\b(.*?)(?=\{)', source, re.S)
        if match:
            raw = 'fn ' + name + ' '.join(uncomment(match[1]).split())
            return raw, path.relative_to(ROOT).as_posix(), source[:match.start()].count('\n') + 1
    raise ValueError('Cannot resolve registered handler: ' + name)


def macro_signature(name):
    source = (ROOT / 'apps/app/src/api/cache.rs').read_text(encoding='utf-8')
    invocation = re.search(r'impl_cache_methods!\((.*?)\);', source, re.S)
    if not invocation:
        raise ValueError('Missing cache macro invocation')
    variants = {}
    for pair in split_args(invocation[1]):
        variant, kind = split_args(pair.strip()[1:-1])
        key = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', variant).lower()
        variants[key] = kind
    for key, kind in variants.items():
        if name == 'get_' + key:
            return f'fn {name}(id: &str, cache_behaviour: Option<CacheBehaviour>) -> Result<Option<{kind}>>'
        if name == 'get_' + key + '_many':
            return f'fn {name}(ids: Vec<String>, cache_behaviour: Option<CacheBehaviour>) -> Result<Vec<{kind}>>'
    return None


def collect_entries():
    entries = []
    for path in sorted((ROOT / 'apps/app/src/api').rglob('*.rs')):
        source = path.read_text(encoding='utf-8')
        plugin = re.search(r'Builder(?:::<[^>]+>)?::new\("([\w-]+)"\)', source)
        if not plugin:
            continue
        paths = [path] + sorted(p for p in path.parent.rglob('*.rs') if p != path)
        for name in registered_names(source):
            signature = macro_signature(name) if plugin[1] == 'cache' else None
            if signature:
                location, line = path.relative_to(ROOT).as_posix(), 8
            else:
                signature, location, line = signature_from(name, paths)
            entries.append({'command': f'plugin:{plugin[1]}|{name}', 'rustSignature': signature, 'source': location, 'sourceLine': line})
    path = ROOT / 'apps/app/src/main.rs'
    for name in registered_names(path.read_text(encoding='utf-8')):
        signature, location, line = signature_from(name, [path, ROOT / 'apps/app/src/updater_impl_noop.rs'])
        entries.append({'command': name, 'rustSignature': signature, 'source': location, 'sourceLine': line})
    commands = [entry['command'] for entry in entries]
    if len(commands) != len(set(commands)):
        raise ValueError('Duplicate command registrations')
    missing, extra = set(commands) - HELP.keys(), HELP.keys() - set(commands)
    if missing or extra:
        raise ValueError(f'Help coverage mismatch: missing={sorted(missing)}, extra={sorted(extra)}')
    return sorted(entries, key=lambda item: item['command'])


ENUMS = {
    'ModLoader': ['vanilla', 'forge', 'fabric', 'quilt', 'neoforge'],
    'ProjectType': ['mod', 'datapack', 'resourcepack', 'shaderpack'],
    'CacheBehaviour': ['stale_while_revalidate_skip_offline', 'stale_while_revalidate', 'must_revalidate', 'bypass'],
    'DownloadReason': ['standalone', 'dependency', 'modpack', 'update'],
    'ImportLauncherType': ['MultiMC', 'PrismLauncher', 'ATLauncher', 'GDLauncher', 'Curseforge', 'Unknown'],
    'LogType': ['InfoLog', 'CrashReport'],
    'MinecraftSkinVariant': ['CLASSIC', 'SLIM', 'UNKNOWN'],
    'WorldType': ['singleplayer', 'server'],
    'DisplayStatus': ['normal', 'hidden', 'favorite'],
    'ServerPackStatus': ['enabled', 'disabled', 'prompt'],
    'ScreenshotEditSaveMode': ['create_copy', 'replace_edit'],
    'DesyncServerMode': ['keep_in_other_instances', 'remove_from_other_instances'],
    'InstanceSyncedOption': ['game_options', 'command_history', 'multiplayer_servers', 'creative_hotbars', 'screenshots', 'resource_packs', 'data_packs'],
    'SyncedOptionJoinResolution': ['use_synced', 'use_instance'],
    'CacheValueType': ['project', 'project_v3', 'version', 'version_v3', 'user', 'team', 'organization', 'file', 'loader_manifest', 'minecraft_manifest', 'categories', 'report_types', 'loaders', 'game_versions', 'donation_platforms', 'file_hash', 'file_update', 'search_results', 'search_results_v3', 'modpack_files', 'embedded_content_metadata', 'project_versions'],
}


def schema_for(kind):
    kind = kind.strip()
    short = kind.split('::')[-1]
    if kind.startswith('Option<'):
        return {'anyOf': [schema_for(kind[7:-1]), {'type': 'null'}]}
    if kind.startswith('Vec<') or kind.startswith('EnumSet<'):
        start = kind.index('<') + 1
        return {'type': 'array', 'items': schema_for(kind[start:-1])}
    if 'Channel<' in kind:
        return {'const': '$channel'}
    if short in ENUMS:
        return {'type': 'string', 'enum': ENUMS[short]}
    if kind == 'bool':
        return {'type': 'boolean'}
    if kind in ['u8', 'u16', 'u32', 'u64', 'usize', 'i8', 'i16', 'i32', 'i64', 'isize']:
        result = {'type': 'integer'}
        if kind.startswith('u'):
            result['minimum'] = 0
        if kind == 'u8':
            result['maximum'] = 255
        return result
    if kind in ['f32', 'f64']:
        return {'type': 'number'}
    if kind in ['String', '&str', 'PathBuf', '&Path', 'SafeRelativeUtf8UnixPathBuf']:
        return {'type': 'string'}
    if 'Uuid' in kind:
        return {'type': 'string', 'format': 'uuid'}
    if short == 'Bytes':
        return {'type': 'array', 'items': {'type': 'integer', 'minimum': 0, 'maximum': 255}}
    return {'x-rust-type': kind, 'x-schema-complete': False}


PARAMS = dict(line.split('|', 1) for line in '''
addToRecents|是否把生成图标的配置加入最近使用列表。
address|Minecraft 服务器地址，可包含端口，例如 play.example.com:25565。
archive|备份 ZIP 的本机绝对路径；备份时为输出文件，恢复时为已有输入文件。
b|是否启用系统窗口装饰；true 开启，false 关闭。
baseInstanceId|初始化共享数据时采用的游戏 ID；省略表示不指定基础游戏。
basePath|待导入启动器的根目录，必须符合 launcherType 对应目录结构。
bytes|字节数据或容量，具体取决于此接口的 Rust 类型；整数是字节数，数组是 0–255 文件字节。
cacheBehaviour|缓存策略；省略或 null 使用默认。四种取值见参数类型表。
cacheTypes|需要清理的缓存类别数组；不会删除已安装游戏。
cape|从可用披风列表取得的完整 Cape 对象；null 表示不指定披风。
clearContents|是否省略返回日志记录中的正文，减少传输；不删除磁盘日志。
config|生成图标配置 InstanceIconConfig；字段按类型附录填写。
configPaths|发布共享游戏时包含的配置相对路径数组；可能上传这些文件。
contentKind|服务器关联内容类型标识；普通自定义服务器可为 null。
contents|Minecraft 命令历史的完整文本，按换行分隔；不是待执行命令。
createOnly|true 只允许创建新文件，已有文件会报错；false 允许按原接口覆盖。
cursor|日志字节游标；0 从开头读取，后续使用上次返回的新游标。
dependencies|从 Modrinth 版本元数据取得的 Dependency 数组。
dependentOnVersionId|被依赖方所属的 Modrinth 版本 ID，用于记录依赖下载来源；无则 null。
description|整合包描述文本；省略时按原导出流程决定默认值。
desiredEnabled|true 启用，false 禁用；省略或 null 时切换当前状态。
destination|重命名或移动后的游戏内相对路径，不能越出游戏目录。
displayStatus|显示状态：normal 普通、hidden 隐藏、favorite 收藏。
displayStatuses|显示状态筛选数组，例如 ["normal","favorite"]；null 使用接口默认范围。
dryRun|true 只预检解压冲突；false 执行解压。
editInstance|EditInstance 修改对象，只填要改的字段；具体字段见复杂类型附录。
enabled|是否启用此项功能或同步资源。
events|启动初始化事件通道，传字面量 "$channel"；只供应用启动流程使用。
excludedOverrides|导出时排除的游戏内相对路径数组。
exportLocation|导出 .mrpack 的绝对文件路径，例如 D:/Packs/export.mrpack。
exportPath|截图导出的本机目标位置；根据接口处理规则提供有效路径。
extension|上传头像的图片扩展名，不含路径；应与实际图片格式一致。
fileId|CurseForge 文件数字 ID；自动安装时 null 表示按游戏兼容条件选择。
filePath|游戏内文件相对路径，例如 mods/example.jar 或临时存放的 ZIP。
filename|文件名称，不是可越出目标目录的任意绝对路径。
filter|保存对话框过滤对象，例如 {"name":"ZIP 文件","extensions":["zip"]}；null 不指定过滤器。
force|是否强制执行完整修复；true 可能重新校验或下载运行文件。
gameVersions|该资源适用的 Minecraft 版本字符串数组，例如 ["1.20.1"]。
groupIds|游戏分组内部 ID 数组；用于排序或成员关系，不能用显示名称代替。
groups|待导入的 ScreenshotGroupImport 数组，字段 id、name、screenshot_ids。
hasPendingRecipients|成员移除流程是否还包含待接受邀请者，按当前共享成员状态填写。
iconPath|本机图片绝对路径；可空语义见对应接口，导出未指定时按游戏现有图标处理。
id|目标对象的内部标识；不同接口的 ID 体系不同，见本条专用说明。
ids|待查询对象的字符串 ID 数组；对象类型由 get_*_many 的命令名称确定。
image|头像图片原始字节数组，每项 0–255；不是图片文件路径。
includeFinished|是否包含已经结束的安装任务；应显式传 true 或 false。
includedOverrides|导出时包含的游戏内相对路径数组，可包含 mods、config 等目录。
index|目标列表下标，从 0 开始；使用刚查询的列表，避免删除或编辑错项。
instance|内部游戏 ID，与 instanceId 含义相同；不是 .minecraft 绝对目录。
instanceFolder|其他启动器根目录中的版本文件夹名，或验证接口要求的绝对文件夹路径；详见本条。
instanceIconUrl|共享游戏的封面 URL；省略时不提供替代图片。
instanceId|启动器内部游戏 ID，从 instance_list、导入结果或任务快照取得，不能用显示名称代替。
instanceIds|多个内部游戏 ID 的数组。
instanceName|快捷方式显示名称，不决定实际游戏 ID。
inviteId|上游共享邀请的 ID，从邀请查询结果取得。
javaVersion|Java 主版本数字或完整配置对象，依此接口类型确定；17 表示 Java 17，配置对象需先检测取得。
jobId|任务标识。安装任务和 CurseForge 安装用 UUID；文件夹导入、世界备份用自己记录的字符串。
key|截图键对象 {"instance_id":"游戏ID","file_name":"图片.png"}，应由列表返回数据构造。
keys|截图键数组，每项包含 instance_id、file_name；不是截图显示标题数组。
launcherType|其他启动器类型枚举，区分大小写；PCL 本地目录请用 gecl 文件夹导入。
level|日志级别字符串，例如 info、warn、error、debug 或 trace。
limit|最多返回的记录数量。
loader|加载器名称，例如 fabric、forge、quilt 或 neoforge；不是加载器版本号。
locale|翻译语言代码，例如 zh_cn；不是任意中文语言名称。
location|整合包来源对象；本地 fromFile 与在线 fromVersionId 的字段示例见下方。
locked|true 锁定资源更新，false 解锁；不改变资源的启用状态。
logType|日志类型，严格使用 "InfoLog" 或 "CrashReport"。
majorVersion|要求的 Java 主版本整数，例如 8、17、21；不是 Minecraft 版本号。
managerId|上游共享管理者 ID；普通本地导入不使用。
maxAgeSeconds|共享邀请有效期秒数；省略时由上游默认规则处理。
maxUses|共享邀请最多使用次数；省略时由上游默认规则处理。
message|写入诊断日志的文字；不要放密码、令牌或真实账户凭据。
mode|保存或解除同步的模式；具体可选字符串和意义见本条类型说明。
name|新建对象或条目的显示名称，不能代替内部 ID。
newName|新的显示名称；世界重命名只修改世界显示名称。
onProgress|进度通道，传字面量 "$channel"，事件收集在 CLI 返回的 events 数组中。
onlyIfEmpty|true 仅在游戏还没有图标时设置；省略按接口默认处理。
option|游戏同步选项枚举；data_packs 虽在枚举中但当前不支持开启。
optionIds|需要读取译名的游戏设置 ID 数组；取自设置编辑状态。
outputPath|快捷方式输出的绝对路径；Windows 通常使用 .lnk 文件路径。
overrideConflicts|解压冲突时是否覆盖已有文件；配合 dryRun 先预检。
packId|全局同步资源的内部 ID；不是在线整合包项目 ID。
packStatus|服务器资源包策略：enabled 接受、disabled 拒绝、prompt 询问。
parent|导出候选文件的相对子目录；省略或 null 从游戏根目录开始。
patch|上游用户资料的补丁 JSON 对象；只能使用服务端允许编辑的字段。
path|目标路径，是否相对路径由本条接口说明确定。
pngBytes|编辑后 PNG 图片的实际字节数组；不是路径或 Base64 文本。
postInstallEdit|整合包安装后调整对象（camelCase）；可选 name、iconPath，省略表示不覆盖原信息。
preferences|外观设置或上游偏好对象，按此接口的类型及下方配方填写。
profileId|Minecraft 档案的 UUID；不是 Modrinth 用户 ID。
projectId|平台项目 ID；Modrinth 是字符串，CurseForge 是数字，不能互换。
projectPath|目标资源路径；安装本地文件时为绝对源路径，管理已装文件时为游戏内相对路径。
projectType|资源类型：mod、datapack、resourcepack 或 shaderpack；shader 也是后端接收的别名。
protocolVersion|Minecraft 网络协议版本，从协议查询取得；省略可按服务端查询逻辑处理。
query|搜索词或完整查询串，依此接口用途确定；不会在 Minecraft 中执行。
reason|下载用途：standalone 单独安装、dependency 前置、modpack 整合包、update 更新。
recursive|是否允许递归删除目录及其内容；删除前确认路径确属目标。
refreshSources|是否刷新译名来源缓存；true 可能读取额外文件或网络。
repair|false 只验证存储完整性；true 还执行允许的修复。
replaceInviteId|用新链接替换的旧共享邀请 ID；省略不指定旧邀请。
replaceTexture|是否替换自定义皮肤的纹理图片。
request|此操作的结构化请求对象，嵌套字段和示例见本条说明与类型附录。
resolution|从加入同步预览中选择冲突处理方式；未发生冲突时可为 null。
restore|false 创建备份，true 从 archive 恢复覆盖；恢复流程先保留当前世界副本。
saveId|原生保存对话框返回的保存会话 ID；不是文件路径。
screenshotIds|截图记录的内部 ID 数组，从截图列表取得。
server|快捷方式的服务器地址；null 不设置自动加入服务器。
serverAddress|启动时要自动加入的服务器地址；null 正常进入游戏。
serverId|全局同步服务器的内部 ID，不是服务器 IP 地址。
serverManagerIconUrl|上游共享服务器管理者的图标 URL；省略不指定。
serverManagerName|上游共享服务器管理者名称；省略不指定。
settings|完整设置对象；设置来源策略和启动器完整设置的结构不同，见本条。
sharedInstanceId|上游在线共享游戏的内部 ID；不是本地游戏 ID。
shouldRestart|是否在上游待安装更新处理后重启；不是立即重启命令。
singleplayerWorld|快捷方式要直接进入的 saves 下世界文件夹名；null 不指定。
skin|从 get_available_skins 返回的完整 Skin 对象；不要用文件名或皮肤名称代替。
source|本机源文件绝对路径；文件重命名接口的 source 则是游戏内相对路径。
sourceInstanceId|复制来源的内部游戏 ID；不会按名称模糊查找。
symbolBytes|生成图标使用的图案资源字节数组，配合 InstanceIconConfig 使用。
targetInstanceId|截图要移动到的目标游戏 ID。
targets|候选目标配置数组，每项包含 game_version 与 loader 字符串。
texture|皮肤图片 URL 字符串或原始字节数组；不是带 type 字段的对象。
textureBlob|皮肤 PNG 的原始字节数组（0–255）；应读取真实图片，不能用空数组当皮肤。
textureKeys|按期望顺序排列的皮肤纹理键数组，从皮肤库取得。
updates|批量变更数组；内容更新、游戏分组和截图分组的条目结构不同，见本条。
user|本地 Minecraft/离线游玩账户 UUID，从 auth 的 get_users 返回结果取得。
userId|Modrinth 平台用户或作者 ID；不是本地游玩账户 UUID。
userIds|多个 Modrinth 平台用户 ID 数组。
username|离线游戏名，使用 3–16 位英文字母、数字或下划线；不是微软邮箱。
uuid|启动器进程记录 UUID，从 process_get_all 取得；不是 Windows 的数字 PID。
variant|皮肤体型：CLASSIC 宽手臂、SLIM 窄手臂；UNKNOWN 只用于兼容未知返回值。
version|筛选的 Java 主版本数字；省略或 null 不按主版本筛选。
versionFile|preview_folder 返回的具体版本 JSON 的绝对路径；不要自己猜文件名。
versionId|Modrinth 文件版本 ID；导出包时则是自定义包版本标签，见本条说明。
window|窗口外观对象（主题、渲染与装饰配置）；不是运行时注入的 Window 参数。
world|saves 下的世界文件夹名称，例如 World1；不是 level.dat 中的显示名称或绝对路径。
worldId|worldType 对应的世界文件夹名或服务器标识，从世界列表取得。
worldType|世界类型：singleplayer 单人世界或 server 多人服务器。
size|缩略图目标边长，单位像素；后端限制在 32–512 范围。
'''.strip().splitlines())

EFFECTS = {
    'network_write': '联网写入：会提交网络请求并修改账户或本地状态，按本条用途及授权条件使用。',
    'read': '本地查询：不提交业务数据变更；部分读取可能维护缓存。',
    'network_read': '查询／缓存：可能联网并写入缓存，不主动安装、更新或发布游戏内容。',
    'write': '写入操作：会修改本地状态、文件，或按此接口用途发起安装／运行操作。',
    'delete': '删除／清理：会删除本条说明的目标，不能把它当作只读查询。',
    'dialog': '原生对话框：需要桌面可交互；未完成选择前调用可能一直等待。',
    'ui': '界面／系统打开：可能打开窗口或外部关联程序。',
    'internal': '内部接口：供应用自身流程使用，一般业务自动化不应重复调用。',
    'disabled': '当前禁用／兼容占位：保留注册名称，但不提供正常业务能力。',
    'upstream_read': '上游服务查询：依赖上游服务或账户权限；GeCL 未提供平台账户登录，注册不代表可用。',
    'upstream_write': '上游服务写入：依赖上游授权，可能发布、邀请、举报或修改远程数据；并非当前 GeCL 已验收的本地能力。',
}
GROUPS = {
    'root': '应用与窗口', 'auth': '游玩账户与微软设备登录', 'cache': 'Modrinth 查询与缓存',
    'files': '游戏文件与系统保存对话框', 'friends': '上游好友兼容接口', 'gecl': 'GeCL 专用功能与 CurseForge',
    'import': '其他启动器目录识别', 'install': '安装任务与整合包', 'instance': '游戏、内容、分组、截图与同步',
    'jre': 'Java 运行环境', 'logs': '游戏日志', 'metadata': '安装元数据', 'minecraft-skins': '正版皮肤库',
    'mr-auth': '已停用的平台账户接口', 'onboarding-checklist': '引导状态', 'process': '游戏进程',
    'reports': '上游举报接口', 'settings': '启动器设置与内容缓存', 'shortcuts': '系统快捷方式',
    'tags': '搜索筛选标签', 'users': '平台用户与作者', 'utils': '系统工具与诊断', 'worlds': '世界与多人服务器',
}


def namespace(command):
    return command.split('|')[0][7:] if command.startswith('plugin:') else 'root'


def bare_type(kind):
    return kind.split('::')[-1]


def describe_param(entry, name, kind):
    command = entry['command']
    ns = namespace(command)
    desc = PARAMS[name]
    if name == 'id':
        if ns == 'auth':
            desc = 'begin_device_login 返回的设备登录会话 UUID；不是账户 ID。'
        elif ns == 'gecl' and '|curseforge_' in command:
            desc = 'CurseForge 项目的数字 ID，例如 238222；不是文件 ID。'
        elif ns == 'gecl' and command.endswith('dismiss_launch'):
            desc = 'launch_progress 返回的启动进度记录 UUID；不会因此结束游戏。'
        elif ns == 'gecl':
            desc = '由外观接口返回的背景副本 ID；不是外部图片绝对路径。'
        elif ns == 'cache' and 'search_results' in command:
            desc = 'Modrinth 搜索查询串，例如 ?query=AppleSkin&limit=20；不是只传搜索词。v3 接口使用其对应搜索参数。'
        elif ns == 'cache':
            desc = '本条 get_* 对应的 Modrinth 对象字符串 ID；项目、版本、用户、团队、组织 ID 各不通用。'
        elif ns == 'utils':
            desc = '调用者生成并保留的诊断导出任务 ID，取消时须使用同一个值。'
        else:
            desc = '游戏分组或截图分组的内部 ID，从对应分组列表取得。'
    if name == 'path':
        if ns == 'files' and not command.endswith('file_read_dragged_file'):
            desc = '游戏根目录内的相对路径，例如 mods/example.jar；file_list 的空字符串表示游戏根目录。'
        elif ns == 'jre':
            desc = '待检测的本机 Java 可执行文件绝对路径，例如 D:/Java/bin/java.exe。'
        elif command.endswith('preview_folder'):
            desc = '已安装游戏目录或 .minecraft 根目录的绝对路径；预览返回可导入的版本 JSON。'
        elif command.endswith('export_debug_info'):
            desc = '诊断 ZIP 的绝对输出文件路径，例如 D:/Reports/gecl.zip。'
        else:
            desc = '本机目标文件或目录的绝对路径，操作对象以本条用途为准。'
    if name == 'instanceFolder':
        desc = ('待验证游戏目录的绝对路径。' if ns == 'import' else 'get_importable_instances 返回的版本文件夹名称，配合 basePath 使用。')
    if name == 'projectId' and '|curseforge_' in command:
        desc = 'CurseForge 项目数字 ID，可从 curseforge_search 或 curseforge_project 取得。'
    if name == 'projectPath':
        if command.endswith(('instance_add_project_from_path', 'instance_is_file_on_modrinth')):
            desc = '本机源模组或资源文件的绝对路径，例如 D:/Downloads/example.jar。'
        else:
            desc = '目标游戏内已装资源的相对路径，例如 mods/example.jar；从内容列表取得。'
    if name == 'source' and command.endswith('file_rename'):
        desc = '重命名前的游戏内相对路径，例如 mods/old.jar。'
    if name == 'versionId' and command.endswith('instance_export_mrpack'):
        desc = '导出包的自定义版本标签，例如 1.0.0；不是在线文件版本 ID。'
    if name == 'javaVersion':
        desc = ('完整 JavaVersion 对象；先调用 jre_get_jre 或 get_java_versions 获取，不要仅传整数。' if kind == 'JavaVersion' else '要安装的 Java 主版本整数，例如 8、17 或 21。')
    if name == 'mode':
        if 'ScreenshotEditSaveMode' in kind:
            desc = 'create_copy 另存副本；replace_edit 覆盖当前截图。'
        else:
            desc = 'keep_in_other_instances 让其他游戏保留自己的副本；remove_from_other_instances 从其他游戏移除对应同步项。'
    if name == 'updates':
        if ns == 'install':
            desc = '内容更新数组，每项 {"project_path":"mods/a.jar","version_id":"Modrinth版本ID"}；字段为 snake_case。'
        elif 'screenshot' in command:
            desc = '截图分组更新数组，每项 {"screenshot_id":"截图ID","group_id":"分组ID或null"}；null 清除分组。'
        else:
            desc = '游戏分组更新数组，每项 {"instance_id":"游戏ID","group_ids":["分组ID"]}；空数组清除所属分组。'
    if name == 'resolution':
        desc = '发生同步冲突时选 use_synced 使用共享数据，或 use_instance 使用当前游戏数据；先查询加入预览。'
    if name == 'request' and '|curseforge_' in command:
        desc = 'CurseForge 查询对象（camelCase）：query 搜索词、gameVersion 游戏版本、loader 加载器、index 起始下标、pageSize 每页数量，另有 sort、category、slug。'
    if name == 'request' and command.endswith('install_create_instance'):
        desc = '新建游戏对象：name 名称、gameVersion Minecraft 版本、loader 加载器、loaderVersion 加载器版本（如 stable）、iconPath 可选本地图标。'
    if name == 'request' and 'GameSettingsRequest' in kind:
        desc = '带修订号的游戏设置变更；先读取编辑状态，保留 expected_*_revision 与 option 修订号，预览后再保存，不能编造为 0。'
    if name == 'request' and 'InstallProjectWithDependenciesRequest' in kind:
        desc = 'Modrinth 安装请求（snake_case）：project_id 项目ID、version_id 可选文件版本ID、content_type 资源类型、selected 可选依赖选择。'
    if name == 'request' and 'ExternalSaveRequest' in kind:
        desc = '外部下载请求：url 文件 URL，headers 请求头对象；保存目标已由 saveId 对应的原生对话框决定。'
    if name == 'preferences' and ns == 'gecl':
        desc = '外观偏好 Preferences（camelCase）：accentColor、backgroundId、backgroundEnabled、blur、overlay、surfaceOpacity、position；范围与示例见复杂参数配方。'
    if name == 'settings':
        desc = ('下载源对象（snake_case）：policy、slow_kib_per_second、observation_seconds；策略及范围见复杂参数配方。' if 'download_sources_' in command else 'settings_get 返回的完整 Settings 对象，保留所有其他字段后只改目标字段。')
    if kind.startswith('Option<'):
        desc += ' 此顶层参数允许省略或传 null；其业务差异以本条用途为准。'
    return desc


RESULTS = {
    'Project': 'Modrinth 项目资料，包括名称、简介、作者团队、图标、下载量、项目类型和版本关联等；具体字段按平台对象定义读取。',
    'ProjectV3': 'Modrinth v3 项目资料；与旧版 Project 字段结构不同，不能直接混用。',
    'Version': 'Modrinth 文件版本资料，包括 id、version_number、game_versions、loaders、files、dependencies 等。',
    'SearchResults': 'Modrinth 搜索结果对象；hits 为项目记录，total_hits 为该平台结果总数，另有 offset、limit 等分页信息。',
    'SearchResultsV3': 'Modrinth v3 搜索结果对象；按 v3 的字段结构读取，不套用旧版字段。',
    'Page<theseus::curseforge::Project>': 'CurseForge 项目结果页及分页信息；项目条目提供该平台名称、作者、图片和下载统计。',

    'InstallJobSnapshot': '安装任务快照；记录 job_id、status、phase、progress、instance_id、error 等。返回任务不表示已经安装完成，继续用 install_job_get 查询。',
    'ProcessMetadata': '启动器管理的游戏进程信息；用其中的进程记录 UUID 查询或停止，不要把它当 Windows PID。返回进程不等于游戏已进入主菜单。',
    'ResolveContentPlan': '解析并执行的内容安装计划，包含依赖和文件处理信息；后续可重新查询游戏内容确认。',
    'GameSettingsEditorState': '游戏设置编辑状态，包括设置项、当前值和并发修改修订信息，供预览及保存请求使用。',
    'SaveGameSettingsResult': '设置保存结果，包含 state、applied、migrated、deferred、unsupported、failed、conflicts；调用成功不代表每一项都已应用。',
    'LatestLogCursor': '增量日志对象：output 为本次文本，cursor 为下一次读取游标，new_file 表示日志文件是否切换。',
    'Credentials': '游玩账户的安全序列化资料；用于识别账户及档案，不应作为可导出的登录令牌。',
    'BackupResult': '备份或恢复结果，包含输出位置与恢复前副本等信息；按实际返回路径检查文件。',
    'StoreUsage': '内容存储的容量和缓存使用统计，各字段名及字节单位见类型附录。',
    'StoreVerification': '存储完整性检查／修复统计；进度事件与最终统计分别读取。',
    'GeclDeviceLogin': '设备登录会话，包含会话 ID、用户验证码、验证地址和轮询信息；需要用户在浏览器完成微软授权。',
    'GeclDevicePoll': '设备登录轮询状态；按返回状态判断等待、成功、到期或错误，不要把所有响应都视为登录成功。',
    'JavaVersion': '检测得到的 Java 完整配置（路径、版本和架构等），可原样供 set_java_version 使用。',
    'PackArtwork': '本地整合包的封面预览信息和应用缓存图片位置；不是安装完成结果。',
    'SetupStatus': '首次设置状态和默认路径信息；finish_setup 仍需满足目录和账户条件。',
    'Appearance': '当前外观偏好、背景副本与窗口配置等信息；保存后可再次查询核对。',
    'Background': '导入的背景副本信息，使用其 ID 写入 Preferences；导入本身不等于应用背景。',
    'LocalGame': '识别到的游戏版本信息，包含后续导入所需的源目录及版本 JSON 路径。',
    'CensoredString': '经过日志遮蔽规则处理的文本字符串；仍应在分享前检查具体内容。',
}


def describe_result(kind):
    if kind in ('', '()'):
        return '成功时 result 为 null；失败时返回统一 error。没有额外结果对象。'
    if kind.endswith('Result<()>'):
        return '成功时 result 为 null；失败时返回统一 error。'
    match = re.match(r'(?:[\w]+::)*Result<(.*)>$', kind)
    if match:
        return describe_result(split_args(match[1])[0])
    if kind.startswith('Option<'):
        return describe_result(kind[7:-1]) + ' 找不到或没有对应结果时为 null。'
    if kind == 'Vec<u8>':
        return '文件原始字节数组，每项为 0–255；若是文本需由调用者按正确编码解码。'
    if kind.startswith('Vec<'):
        return '数组；每项为：' + describe_result(kind[4:-1])
    short = bare_type(kind)
    if short in RESULTS:
        return RESULTS[short]
    if kind == 'bool':
        return '布尔值 true / false，含义由本条用途决定；不是 JSON 的字符串。'
    if kind in ('String', 'PathBuf'):
        return '字符串。路径类操作返回本机路径，查询类操作返回对应正文或标识；具体用途见本条。'
    if kind in ('u64', 'u32', 'usize'):
        return '非负整数；大小、容量、索引的单位以本条用途及参数说明为准。'
    return f'序列化的 {kind} 数据；字段定义可在 AGENT_TYPES.json 按类型名称查找。'


SPECIAL_RESULTS = {
    'Project': 'Modrinth 项目资料，包括名称、简介、作者团队、图标、下载量、项目类型和版本关联等；具体字段按平台对象定义读取。',
    'ProjectV3': 'Modrinth v3 项目资料；与旧版 Project 字段结构不同，不能直接混用。',
    'Version': 'Modrinth 文件版本资料，包括 id、version_number、game_versions、loaders、files、dependencies 等。',
    'SearchResults': 'Modrinth 搜索结果对象；hits 为项目记录，total_hits 为该平台结果总数，另有 offset、limit 等分页信息。',
    'SearchResultsV3': 'Modrinth v3 搜索结果对象；按 v3 的字段结构读取，不套用旧版字段。',
    'Page<theseus::curseforge::Project>': 'CurseForge 项目结果页及分页信息；项目条目提供该平台名称、作者、图片和下载统计。',

    'plugin:jre|jre_get_max_memory': '非负整数，单位 KiB（系统内存字节数 / 1024）；要显示 MiB 再除以 1024。',
    'plugin:worlds|backup_world': '旧版备份生成的 ZIP 文件大小，单位字节；备份保存在该游戏的 backups 下。',
    'plugin:gecl|curseforge_install': '已安装文件的相对路径数组；这是直接等待安装的调用，不返回 install_job 的任务快照。进度在 events 中。',
    'plugin:gecl|curseforge_installed': '对象映射：键为内部游戏 ID，值为该游戏已安装的 CurseForge 项目数字 ID 数组。',
    'plugin:auth|get_default_user': '当前默认游玩账户的 UUID 字符串；没有默认账户时为 null。',
    'plugin:mr-auth|get': '固定为 null；当前构建没有 Modrinth 平台账户登录。',
    'plugin:mr-auth|get_all': '固定为空数组 []。',
    'plugin:files|files_select_external': '保存会话 ID 字符串；用户取消时为 null。后续 files_save_external 使用这个 ID，不传输出路径。',
    'plugin:gecl|import_folder': '导入成功后的内部游戏 ID 字符串，可用来读取内容、设置图标或启动游戏。',
    'plugin:instance|instance_export_mrpack': '成功为 null；在 exportLocation 指定位置检查导出的 .mrpack 文件。',
    'plugin:utils|export_debug_info': '布尔值，表示诊断导出是否完成；取消与失败应结合实际返回和错误判断。',
    'plugin:utils|get_image_thumbnail': '前端可加载的缩略图地址字符串；不是原图片字节。',
    'get_update_size': '当前构建返回 updates are disabled in this build 错误，不返回文件大小。',
    'enqueue_update_for_installation': '当前构建返回 updates are disabled in this build 错误。',
    'remove_enqueued_update': '当前构建执行兼容空操作，result 为 null。',
}


COMPLEX_EXAMPLES = {
    'EditInstance': {'name': '新的游戏名称'},
    'InstallPostInstallEditRequest': {'name': '我的整合包'},
    'InstallCreateInstanceRequest': {'name': '测试游戏', 'gameVersion': '1.20.1', 'loader': 'fabric', 'loaderVersion': 'stable', 'iconPath': None},
    'CreatePackLocation': {'type': 'fromFile', 'path': 'D:/Packs/example.mrpack'},
    'Search': {'query': 'AppleSkin', 'gameVersion': '1.20.1', 'loader': 'forge', 'index': 0, 'pageSize': 20},
    'DownloadSourceSettings': {'policy': 'official_preferred', 'slow_kib_per_second': 128, 'observation_seconds': 8},
    'Preferences': {'accentColor': '#176fb0', 'backgroundId': None, 'backgroundEnabled': True, 'blur': 8, 'overlay': 18, 'surfaceOpacity': 88, 'position': 'center'},
    'ExternalSaveRequest': {'url': 'https://example.com/example.zip', 'headers': {}},
    'ExternalSaveFilter': {'name': 'ZIP 文件', 'extensions': ['zip']},
    'InstallProjectWithDependenciesRequest': {'project_id': '<Modrinth项目ID>', 'version_id': None, 'content_type': 'mod'},
    'ScreenshotKey': {'instance_id': '<游戏ID>', 'file_name': '<列表中的图片.png>'},
    'ContentUpdateSelection': {'project_path': 'mods/<原文件名.jar>', 'version_id': '<Modrinth版本ID>'},
    'InstanceGroupMembershipUpdate': {'instance_id': '<游戏ID>', 'group_ids': ['<分组ID>']},
    'ScreenshotGroupMembershipUpdate': {'screenshot_id': '<截图ID>', 'group_id': '<分组ID>'},
    'ScreenshotGroupImport': {'id': '<截图分组ID>', 'name': '截图分组', 'screenshot_ids': ['<截图ID>']},
    'InstanceInstallTarget': {'game_version': '1.20.1', 'loader': 'forge'},
    'SyncedServer': {'id': '<同步服务器ID>', 'name': '测试服务器', 'address': 'play.example.com:25565', 'accept_textures': None},
    'UrlOrBlob': 'https://example.com/skin.png',
}
VALUES = {
    'username': 'TestUser', 'instanceId': '<游戏ID>', 'instance': '<游戏ID>',
    'sourceInstanceId': '<来源游戏ID>', 'targetInstanceId': '<目标游戏ID>',
    'projectId': '<Modrinth项目ID>', 'versionId': '<Modrinth版本ID>',
    'jobId': '<任务UUID>', 'user': '<账户UUID>', 'uuid': '<进程记录UUID>',
    'id': '<对象ID>', 'userId': '<平台用户ID>', 'profileId': '<Minecraft档案UUID>',
    'name': '测试名称', 'newName': '新名称', 'query': 'AppleSkin', 'loader': 'fabric',
    'version': 17, 'javaVersion': 17, 'majorVersion': 17, 'limit': 10, 'index': 0,
    'cursor': 0, 'size': 128, 'level': 'info', 'message': 'Agent 测试日志',
    'world': '<世界文件夹名>', 'worldId': '<世界文件夹名>',
    'path': 'D:/Example/file', 'source': 'D:/Example/source.png',
    'archive': 'D:/Backups/world.zip', 'basePath': 'D:/Launcher',
    'instanceFolder': '<查询到的版本文件夹>', 'projectPath': 'mods/<模组文件.jar>',
    'filePath': 'mods/<模组文件.jar>', 'destination': 'mods/<新文件名.jar>',
    'exportLocation': 'D:/Packs/export.mrpack', 'exportPath': 'D:/Pictures/Export',
    'outputPath': 'D:/Desktop/游戏.lnk', 'filename': 'example.zip', 'contents': '/help\n',
    'iconPath': 'D:/Pictures/pack.png', 'versionFile': '<预览返回的版本JSON路径>',
    'locale': 'zh_cn', 'address': 'play.example.com:25565', 'instanceName': '测试游戏',
    'extension': 'png', 'parent': None, 'restore': False, 'includeFinished': True,
    'bytes': 104857600,
}


def sample(kind, name):
    kind = kind.strip()
    if kind.startswith('Option<'):
        return None
    if kind.startswith('Vec<'):
        inner = kind[4:-1]
        if inner == 'u8':
            return '<从实际文件读取的0到255字节数组>'
        if bare_type(inner) in COMPLEX_EXAMPLES:
            return [COMPLEX_EXAMPLES[bare_type(inner)]]
        if inner in ('String', '&str'):
            return ['<对应ID或相对路径>']
        return [sample(inner, name)]
    if 'Channel<' in kind:
        return '$channel'
    if bare_type(kind) in COMPLEX_EXAMPLES:
        return COMPLEX_EXAMPLES[bare_type(kind)]
    if kind == 'JavaVersion':
        return '<复制jre_get_jre返回的完整对象>'
    if kind == 'Settings':
        return '<复制settings_get返回的完整对象并修改目标字段>'
    if kind in ('Skin', 'Cape'):
        return '<复制对应可用列表中返回的完整对象>'
    if bare_type(kind) in ENUMS:
        return ENUMS[bare_type(kind)][0]
    if name in VALUES:
        return VALUES[name]
    if kind == 'bool':
        return False
    if schema_for(kind).get('type') in ('integer', 'number'):
        return 0
    if 'Uuid' in kind:
        return '<实际UUID>'
    if schema_for(kind).get('type') == 'string':
        return '<' + name + '>'
    return '<按 ' + kind + ' 声明填写的完整值>'


def add_details(entry):
    entry.update(HELP[entry['command']])
    entry['effectDescription'] = EFFECTS[entry['effect']]
    signature = entry['rustSignature']
    opening = signature.index('(')
    depth, closing = 1, opening + 1
    while depth:
        if signature[closing] == '(':
            depth += 1
        elif signature[closing] == ')':
            depth -= 1
        closing += 1
    properties, required = {}, []
    for arg in split_args(signature[opening + 1:closing - 1]):
        if ':' not in arg:
            raise ValueError('Unparsed argument: ' + arg)
        name, kind = arg.split(':', 1)
        kind = kind.strip()
        if any(token in kind for token in ['AppHandle', 'WebviewWindow', 'tauri::Webview<', 'tauri::Window', 'State<']):
            continue
        pieces = name.strip().split('_')
        name = pieces[0] + ''.join(piece.title() for piece in pieces[1:])
        properties[name] = dict(schema_for(kind), description=describe_param(entry, name, kind), **{'x-rust-type': kind})
        if not kind.startswith('Option<'):
            required.append(name)
    entry['inputSchema'] = {'$schema': 'https://json-schema.org/draft/2020-12/schema', 'type': 'object', 'properties': properties, 'required': required, 'additionalProperties': False}
    result_kind = signature[closing:].strip().removeprefix('->').strip() or '()'
    entry['returnType'] = result_kind
    entry['resultDescription'] = SPECIAL_RESULTS.get(entry['command'], describe_result(result_kind))
    args = {name: sample(prop['x-rust-type'], name) for name, prop in properties.items()}
    cmd, ns = entry['command'], namespace(entry['command'])
    if ns == 'cache':
        if 'search_results' in cmd:
            if 'id' in args:
                args['id'] = '?query=AppleSkin&limit=20'
            if 'ids' in args:
                args['ids'] = ['?query=AppleSkin&limit=20']
    if '|curseforge_' in cmd:
        if 'id' in args:
            args['id'] = 238222
        if 'projectId' in args:
            args['projectId'] = 238222
        if 'fileId' in args and 'fileId' in required:
            args['fileId'] = '<实际CurseForge文件数字ID>'
    if cmd.endswith('instance_export_mrpack'):
        args.update(includedOverrides=['mods', 'config', 'resourcepacks', 'shaderpacks'], excludedOverrides=[], versionId='1.0.0', name='我的整合包')
    if cmd.endswith('instance_add_project_from_path') or cmd.endswith('instance_is_file_on_modrinth'):
        args['projectPath'] = 'D:/Downloads/example.jar'
    if ns == 'files':
        if 'path' in args and not cmd.endswith('file_read_dragged_file'):
            args['path'] = '' if cmd.endswith('file_list') else 'config/<目标文件>'
        if cmd.endswith('file_rename'):
            args['source'] = 'mods/<原文件.jar>'
        if cmd.endswith('file_extract_zip'):
            args.update(filePath='<游戏内的压缩包.zip>', dryRun=True)
    if cmd.endswith(('preview_folder', 'choose_directory')):
        args['path'] = 'D:/Minecraft'
    if cmd.endswith(('import_folder', 'world_backup', 'cancel_world_backup', 'cancel_import')):
        args['jobId'] = '<本次操作自定任务ID>'
    if cmd.endswith('get_image_thumbnail'):
        args['path'] = '<允许访问的本地图片绝对路径>'
    if cmd.endswith('export_debug_info'):
        args['path'] = 'D:/Reports/gecl.zip'
    if cmd.endswith('instance_get_install_candidates'):
        args['targets'] = [{'game_version': '1.20.1', 'loader': 'forge'}]
    entry['argumentTemplate'] = args
    entry['templateNote'] = '调用模板；把所有尖括号占位值替换为真实值。复杂对象应按来源查询结果或类型声明填写，模板不是已验证可执行的测试数据。'
    if entry['effect'] in ('disabled', 'internal', 'upstream_read', 'upstream_write'):
        entry['templateNote'] += ' 此接口的当前可用性还受上方状态限制，不应当作常规可用功能。'


def collect_types():
    types = {}
    for folder in ['apps/app/src', 'packages/app-lib/src', 'packages/modrinth-content-management/src']:
        for path in sorted((ROOT / folder).rglob('*.rs')):
            source = path.read_text(encoding='utf-8')
            for match in re.finditer(r'\b(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+([A-Z]\w*)[^;{]*\{', source):
                depth, end = 1, match.end()
                while end < len(source) and depth:
                    if source[end] == '{':
                        depth += 1
                    elif source[end] == '}':
                        depth -= 1
                    end += 1
                # Capture only this item's preceding attributes, not a neighbour's serde rules.
                start = source.rfind('\n}', 0, match.start()) + 2
                prefix = source[start:match.start()]
                attrs = '\n'.join(re.findall(r'#\[serde\([\s\S]*?\)\]', prefix))
                types.setdefault(match[1], []).append({'source': path.relative_to(ROOT).as_posix(), 'sourceLine': source[:match.start()].count('\n') + 1, 'declaration': (attrs + '\n' + source[match.start():end]).strip()})
    return types


def md_cell(value):
    return str(value).replace('|', '\\|').replace('\n', '<br>')


def kind_label(prop):
    label = prop['x-rust-type']
    schema = prop['anyOf'][0] if 'anyOf' in prop else prop
    if 'enum' in schema:
        label += '；可选值 ' + ', '.join(schema['enum'])
    elif schema.get('type') == 'array' and 'enum' in schema.get('items', {}):
        label += '；数组项 ' + ', '.join(schema['items']['enum'])
    return label


def write_reference(entries):
    markdown = ROOT / 'docs/AGENT_COMMANDS.md'
    intro = markdown.read_text(encoding='utf-8').split('<!-- BEGIN GENERATED COMMANDS -->')[0].rstrip()
    text = intro + '\n\n<!-- BEGIN GENERATED COMMANDS -->\n\n## 全部接口逐条说明\n\n'
    text += f'本表按当前源码注册表整理，共 **{len(entries)} 条应用接口**。接口注册与实际可用性分开说明；没有把上游平台功能当作已经开放的 GeCL 本地功能。\n\n'
    text += '每条都列出用途、参数意义、结果、数据影响及调用模板。外层参数是 camelCase；嵌套对象必须遵守自己的字段格式。尖括号内容均为需要替换的占位值，不是能直接照抄执行的真实 ID。无参数接口传 `{}`。\n\n'
    groups = {}
    for entry in entries:
        groups.setdefault(namespace(entry['command']), []).append(entry)
    for ns, group in groups.items():
        text += f'- [{GROUPS[ns]}（{len(group)} 条）](#api-{ns})\n'
    for ns, group in groups.items():
        text += f'\n<a id="api-{ns}"></a>\n\n### {GROUPS[ns]}\n\n'
        text += '| 指令 | 作用 | 状态／影响 |\n|---|---|---|\n'
        for entry in group:
            name = entry['command'].split('|')[-1]
            text += f'| [`{name}`](#cmd-{ns}-{name}) | {entry["title"]} | {entry["effectDescription"].split("：")[0]} |\n'
        for entry in group:
            name = entry['command'].split('|')[-1]
            text += f'\n<a id="cmd-{ns}-{name}"></a>\n\n#### `{entry["command"]}` — {entry["title"]}\n\n'
            text += entry['description'] + '\n\n'
            text += '**影响与可用性：** ' + entry['effectDescription'] + '\n\n'
            props = entry['inputSchema']['properties']
            if props:
                text += '| 参数 | 必填 | 类型／取值 | 具体含义 |\n|---|---|---|---|\n'
                for key, prop in props.items():
                    text += f'| `{key}` | {"是" if key in entry["inputSchema"]["required"] else "否"} | {md_cell(kind_label(prop))} | {md_cell(prop["description"])} |\n'
            else:
                text += '**参数：** 无，传 `{}`。\n'
            text += '\n**返回：** ' + entry['resultDescription'] + f' Rust 返回类型：`{entry["returnType"]}`。\n\n'
            text += entry['templateNote'] + '\n\n'
            if props:
                text += '把下列 JSON 保存为 UTF-8 的 `args.json`（替换占位值后调用）：\n\n```json\n'
                text += json.dumps(entry['argumentTemplate'], ensure_ascii=False, indent=2) + '\n```\n\n'
                text += f"```powershell\n.\\gecl-agent.exe invoke '{entry['command']}' '@args.json'\n```\n\n"
            else:
                text += f"```powershell\n.\\gecl-agent.exe invoke '{entry['command']}' '{{}}'\n```\n\n"
            text += f'源码位置：`{entry["source"]}:{entry["sourceLine"]}`（相对源码根目录）。\n'
    markdown.write_text(text + '\n<!-- END GENERATED COMMANDS -->\n', encoding='utf-8')


def main():
    entries = collect_entries()
    for entry in entries:
        add_details(entry)
    types = collect_types()
    (ROOT / 'docs/AGENT_COMMANDS.json').write_text(json.dumps(entries, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (ROOT / 'docs/AGENT_TYPES.json').write_text(json.dumps(types, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    write_reference(entries)
    print(json.dumps({'commands': len(entries), 'parameters': sum(len(e['inputSchema']['properties']) for e in entries), 'effects': dict(Counter(e['effect'] for e in entries)), 'types': len(types)}, ensure_ascii=False))


if __name__ == '__main__':
    main()
