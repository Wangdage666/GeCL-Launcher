use crate::state::instances::adapters::sqlite::{content_rows, instance_rows};
use crate::state::{
    ContentSet, ContentSetStatus, ContentSourceKind, Instance,
    InstanceInstallStage, InstanceLaunchOverrides, InstanceLink,
    LauncherFeatureVersion, ModLoader, ReleaseChannel, State,
};
use anyhow::{Context, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static REGISTRY_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalGame {
    pub source: PathBuf,
    pub version_file: PathBuf,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
    pub mods: Vec<String>,
    pub shaderpacks: Vec<String>,
    pub resourcepacks: Vec<String>,
    pub worlds: Vec<String>,
}

fn linked(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

fn parse_version(value: &Value) -> Option<(String, ModLoader, Option<String>)> {
    if value.get("geclFormat") == Some(&Value::from(1)) {
        let game = value.get("gameVersion")?.as_str()?;
        if !safe_version(game) {
            return None;
        }
        let loader: ModLoader =
            serde_json::from_value(value.get("loader")?.clone()).ok()?;
        let version = value
            .get("loaderVersion")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if loader != ModLoader::Vanilla
            && !version.as_deref().is_some_and(safe_version)
        {
            return None;
        }
        return Some((game.to_owned(), loader, version));
    }
    let main = value.get("mainClass")?.as_str()?;
    let game = value
        .get("clientVersion")
        .or_else(|| value.get("inheritsFrom"))
        .or_else(|| value.get("id"))?
        .as_str()?;
    if !safe_version(game)
        || !game.as_bytes().first().is_some_and(u8::is_ascii_digit)
    {
        return None;
    }
    let libraries = value.get("libraries")?.as_array()?;
    for (group, loader) in [
        ("net.fabricmc:fabric-loader:", ModLoader::Fabric),
        ("org.quiltmc:quilt-loader:", ModLoader::Quilt),
        ("net.neoforged:neoforge:", ModLoader::NeoForge),
        ("net.minecraftforge:forge:", ModLoader::Forge),
        ("net.minecraftforge:fmlloader:", ModLoader::Forge),
    ] {
        if let Some(version) = libraries
            .iter()
            .filter_map(|lib| lib.get("name")?.as_str())
            .find_map(|name| name.strip_prefix(group))
        {
            let version = version.split(':').next()?;
            if !safe_version(version) {
                return None;
            }
            return Some((game.to_owned(), loader, Some(version.to_owned())));
        }
    }
    if main != "net.minecraft.client.main.Main"
        && main != "net.minecraft.launchwrapper.Launch"
    {
        return None;
    }
    // LaunchWrapper without a known loader may be OptiFine or another custom setup.
    if main.contains("launchwrapper") {
        return None;
    }
    Some((game.to_owned(), ModLoader::Vanilla, None))
}

async fn names(
    root: &Path,
    kind: &str,
    directory: bool,
) -> anyhow::Result<Vec<String>> {
    let path = root.join(kind);
    if !tokio::fs::try_exists(&path).await? {
        return Ok(Vec::new());
    }
    if linked(&tokio::fs::symlink_metadata(&path).await?) {
        bail!("内容目录包含链接，请使用普通文件夹导入");
    }
    let mut entries = tokio::fs::read_dir(path).await?;
    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let meta = tokio::fs::symlink_metadata(entry.path()).await?;
        if linked(&meta) {
            bail!("内容目录包含链接，请使用普通文件夹导入");
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if directory && meta.is_dir()
            || !directory && (meta.is_file() || meta.is_dir())
        {
            if !directory
                && kind == "mods"
                && !(name.ends_with(".jar") || name.ends_with(".jar.disabled"))
            {
                continue;
            }
            result.push(name);
        }
        if result.len() > 20_000 {
            bail!("目录内容过多，请选择单个游戏版本文件夹");
        }
    }
    result.sort();
    Ok(result)
}

async fn inspect(
    source: &Path,
    version_file: &Path,
) -> anyhow::Result<Option<LocalGame>> {
    let source_meta = tokio::fs::symlink_metadata(source).await?;
    if !source_meta.is_dir() || linked(&source_meta) {
        return Ok(None);
    }
    let meta = match tokio::fs::symlink_metadata(version_file).await {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if !meta.is_file() || linked(&meta) || meta.len() > 2_000_000 {
        return Ok(None);
    }
    let bytes = tokio::fs::read(version_file).await?;
    let value: Value = match serde_json::from_slice(
        bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes),
    ) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some((game_version, loader, loader_version)) = parse_version(&value)
    else {
        return Ok(None);
    };
    let after = tokio::fs::metadata(version_file).await?;
    if meta.len() != after.len() || meta.modified()? != after.modified()? {
        return Ok(None);
    }
    let canonical = tokio::fs::canonicalize(source).await?;
    let version_file = tokio::fs::canonicalize(version_file).await?;
    if !version_file.starts_with(&canonical) {
        bail!("版本描述必须位于所选游戏目录内");
    }
    let name = if value.get("geclFormat") == Some(&Value::from(1)) {
        value
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty() && name.len() <= 512)
            .map(str::to_owned)
    } else {
        version_file
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
    }
    .unwrap_or_else(|| {
        source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    Ok(Some(LocalGame {
        source: canonical,
        version_file,
        name,
        game_version,
        loader,
        loader_version,
        mods: names(source, "mods", false).await?,
        shaderpacks: names(source, "shaderpacks", false).await?,
        resourcepacks: names(source, "resourcepacks", false).await?,
        worlds: names(source, "saves", true).await?,
    }))
}

pub async fn preview_folder(path: PathBuf) -> anyhow::Result<Vec<LocalGame>> {
    let path = tokio::fs::canonicalize(path)
        .await
        .context("无法读取所选目录")?;
    if linked(&tokio::fs::symlink_metadata(&path).await?) {
        bail!("请选择普通文件夹");
    }
    let name = path
        .file_name()
        .context("请选择游戏文件夹，而不是磁盘根目录")?
        .to_string_lossy();
    for file in [
        path.join("gecl-profile.json"),
        path.join(format!("{name}.json")),
    ] {
        if let Some(game) = inspect(&path, &file).await? {
            return Ok(vec![game]);
        }
    }
    let mut games = Vec::new();
    for root in [&path, &path.join(".minecraft")] {
        let versions = root.join("versions");
        if !tokio::fs::try_exists(&versions).await? {
            continue;
        }
        if linked(&tokio::fs::symlink_metadata(&versions).await?) {
            bail!("versions 目录不能是链接");
        }
        let mut children = tokio::fs::read_dir(versions).await?;
        while let Some(entry) = children.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let file = entry
                .path()
                .join(format!("{}.json", entry.file_name().to_string_lossy()));
            let isolated = entry.path().join("mods").is_dir()
                || entry.path().join("saves").is_dir();
            let source = if isolated {
                entry.path()
            } else {
                root.to_path_buf()
            };
            if let Some(mut game) = inspect(&source, &file).await? {
                game.name = entry.file_name().to_string_lossy().into_owned();
                games.push(game);
            }
            if games.len() > 256 {
                bail!("版本太多，请选择单个版本目录");
            }
        }
    }
    if games.is_empty() {
        bail!(
            "没有找到可识别的 Minecraft 版本描述。请选择包含同名 .json 的游戏文件夹，或包含 versions 的 .minecraft 目录；只有 mods 文件夹无法确定游戏版本。"
        );
    }
    games.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(games)
}

async fn register(
    game: &LocalGame,
    path: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Option<Instance>> {
    crate::state::content_store::validate_relative(path)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM instances WHERE lower(path) = lower(?)",
    )
    .bind(path)
    .fetch_one(&mut *tx)
    .await?;
    if exists != 0 {
        return Ok(None);
    }
    let now = Utc::now();
    let id = format!("local:{}", uuid::Uuid::new_v4());
    let content_id = format!("content-set:{}", uuid::Uuid::new_v4());
    let instance = Instance {
        id: id.clone(),
        path: path.to_owned(),
        applied_content_set_id: Some(content_id.clone()),
        install_stage: InstanceInstallStage::NotInstalled,
        launcher_feature_version: LauncherFeatureVersion::MOST_RECENT,
        update_channel: ReleaseChannel::Release,
        name: game.name.clone(),
        icon_path: None,
        created: now,
        modified: now,
        last_played: None,
        submitted_time_played: 0,
        recent_time_played: 0,
    };
    let content = ContentSet {
        id: content_id,
        instance_id: id.clone(),
        name: "Default".to_owned(),
        source_kind: ContentSourceKind::Local,
        status: ContentSetStatus::Available,
        game_version: game.game_version.clone(),
        protocol_version: None,
        loader: game.loader,
        loader_version: game.loader_version.clone(),
        created: now,
        modified: now,
    };
    instance_rows::insert_instance(&instance, &mut tx).await?;
    content_rows::insert_content_set(&content, &mut tx).await?;
    instance_rows::upsert_instance_link(&id, &InstanceLink::Unmanaged, &mut tx)
        .await?;
    instance_rows::upsert_instance_launch_overrides(
        &InstanceLaunchOverrides::empty(id.clone()),
        &mut tx,
    )
    .await?;
    sqlx::query("INSERT INTO instance_sync_preferences(instance_id,feature,enabled) SELECT ?,feature,0 FROM sync_feature_settings").bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Some(instance))
}

pub async fn discover(state: &State) -> anyhow::Result<Vec<Instance>> {
    let _registration = REGISTRY_LOCK.lock().await;
    let root = state.directories.instances_dir();
    let mut entries = tokio::fs::read_dir(&root).await?;
    let known = instance_rows::list_instances(&state.pool)
        .await?
        .into_iter()
        .map(|i| i.path.to_lowercase())
        .collect::<HashSet<_>>();
    let mut added = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || known.contains(&name.to_lowercase())
            || !entry.file_type().await?.is_dir()
        {
            continue;
        }
        for file in [
            entry.path().join("gecl-profile.json"),
            entry.path().join(format!("{name}.json")),
        ] {
            match inspect(&entry.path(), &file).await {
                Ok(Some(game)) => {
                    if let Some(instance) =
                        register(&game, &name, &state.pool).await?
                    {
                        added.push(instance);
                    }
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!("Local game discovery deferred: {error}");
                    break;
                }
            }
        }
    }
    Ok(added)
}

fn excluded(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "assets"
            | "libraries"
            | "versions"
            | "launcher_accounts.json"
            | "launcher_profiles.json"
            | "launcher_msa_credentials.bin"
            | "accounts.json"
            | "accounts.offline.json"
            | "gecl-profile.json"
    ) || name.ends_with("-natives")
}

pub async fn finalize_import(id: &str) -> anyhow::Result<()> {
    let state = State::get().await?;
    let instance = instance_rows::get_instance_by_id(id, &state.pool)
        .await?
        .context("找不到导入的游戏版本")?;
    super::super::instances::watcher::watch_instance_folder(
        id,
        &instance.path,
        &state.file_watcher,
        &state.directories,
    )
    .await;
    if let Err(error) = crate::api::instance::sync_content_files(id).await {
        tracing::warn!("Imported content indexing deferred: {error}");
    }
    if let Err(error) = crate::event::emit::emit_instance(
        id,
        crate::event::InstancePayloadType::Created,
    )
    .await
    {
        tracing::warn!("Imported game refresh notification failed: {error}");
    }
    Ok(())
}

async fn copy_files(
    source: &Path,
    target: &Path,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> anyhow::Result<()> {
    let mut pending = vec![(source.to_path_buf(), PathBuf::new())];
    let mut files = Vec::new();
    let mut total = 0u64;
    while let Some((directory, relative)) = pending.pop() {
        if cancelled() {
            bail!("导入已取消");
        }
        let mut entries = tokio::fs::read_dir(directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            if relative.as_os_str().is_empty()
                && excluded(&entry.file_name().to_string_lossy())
            {
                continue;
            }
            let meta = tokio::fs::symlink_metadata(entry.path()).await?;
            if linked(&meta) {
                bail!("游戏文件夹包含链接，已停止导入；源文件未改动");
            }
            let destination = relative.join(entry.file_name());
            if meta.is_dir() {
                tokio::fs::create_dir_all(target.join(&destination)).await?;
                pending.push((entry.path(), destination));
            } else if meta.is_file() {
                total =
                    total.checked_add(meta.len()).context("文件总大小溢出")?;
                files.push((entry.path(), destination));
            }
            if files.len() + pending.len() > 200_000 {
                bail!("文件过多，请选择单个游戏版本目录");
            }
        }
    }
    let mut copied = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    progress(0, total);
    for (source, relative) in files {
        if cancelled() {
            bail!("导入已取消");
        }
        let before = tokio::fs::symlink_metadata(&source).await?;
        if linked(&before) {
            bail!("源文件在导入期间变成了链接");
        }
        let mut from = tokio::fs::File::open(&source).await?;
        let mut to = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target.join(relative))
            .await?;
        loop {
            if cancelled() {
                bail!("导入已取消");
            }
            let n = from.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            to.write_all(&buffer[..n]).await?;
            copied += n as u64;
            progress(copied, total);
        }
        to.sync_all().await?;
        let after = tokio::fs::metadata(&source).await?;
        if before.len() != after.len()
            || before.modified()? != after.modified()?
        {
            bail!("源文件正在变化，请退出游戏后重试");
        }
    }
    Ok(())
}

pub async fn import_folder(
    source: PathBuf,
    version_file: PathBuf,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> anyhow::Result<String> {
    let state = State::get().await?;
    let mut game = inspect(&source, &version_file)
        .await?
        .context("版本描述已更改或无法识别，请重新选择文件夹")?;
    let root =
        tokio::fs::canonicalize(state.directories.instances_dir()).await?;
    if game.source.parent() == Some(root.as_path()) {
        let _registration = REGISTRY_LOCK.lock().await;
        let name = game
            .source
            .file_name()
            .context("无效游戏目录")?
            .to_string_lossy()
            .into_owned();
        if let Some(instance) = register(&game, &name, &state.pool).await? {
            return Ok(instance.id);
        }
        return Ok(instance_rows::get_instance_by_path(&name, &state.pool)
            .await?
            .context("游戏登记失败")?
            .id);
    }
    if root.starts_with(&game.source) || game.source.starts_with(&root) {
        bail!("请勿导入游戏数据目录本身或其内部临时目录");
    }
    let temporary = root.join(format!(".gecl-import-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir(&temporary).await?;
    let result = async {
        copy_files(&game.source,&temporary,cancelled,progress).await?;
        if cancelled() { bail!("导入已取消"); }
        let marker = serde_json::json!({"geclFormat":1,"name":game.name,"gameVersion":game.game_version,"loader":game.loader,"loaderVersion":game.loader_version,"runtimeSource":game.source,"runtimeVersionFile":game.version_file});
        tokio::fs::write(temporary.join("gecl-profile.json"),serde_json::to_vec_pretty(&marker)?).await?;
        let clean = game.name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'],"_");
        let destination = loop {
            let candidate = root.join(format!("{} ({})",clean,uuid::Uuid::new_v4().simple()));
            if !tokio::fs::try_exists(&candidate).await? { break candidate; }
        };
        let _registration = REGISTRY_LOCK.lock().await;
        if cancelled() { bail!("导入已取消"); }
        tokio::fs::rename(&temporary,&destination).await?;
        game.source = destination.clone();
        let name = destination.file_name().context("无效导入目录")?.to_string_lossy().into_owned();
        match register(&game,&name,&state.pool).await {
            Ok(Some(instance)) => Ok(instance.id),
            result => {
                // This UUID destination was created by this import; never remove the source.
                tokio::fs::rename(&destination,&temporary).await?;
                match result { Err(error) => Err(error), _ => anyhow::bail!("导入目录登记冲突") }
            }
        }
    }.await;
    if tokio::fs::try_exists(&temporary).await? {
        tokio::fs::remove_dir_all(&temporary).await?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fabric() -> Value {
        serde_json::json!({"id":"Renamed pack","clientVersion":"1.20.1","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"net.fabricmc:fabric-loader:0.19.3"}]})
    }

    #[test]
    fn gecl_reads_real_version_fields_without_guessing_folder_names() {
        let result = parse_version(&fabric()).unwrap();
        assert_eq!(
            result,
            (
                "1.20.1".to_owned(),
                ModLoader::Fabric,
                Some("0.19.3".to_owned())
            )
        );
        assert!(
            parse_version(&serde_json::json!({"id":"1.20.1","mods":[]}))
                .is_none()
        );
        let mut invalid = fabric();
        invalid["clientVersion"] = Value::from("../../other");
        assert!(parse_version(&invalid).is_none());
        invalid = fabric();
        invalid["libraries"] = serde_json::json!([]);
        assert!(parse_version(&invalid).is_none());
    }

    #[tokio::test]
    async fn gecl_preview_and_copy_preserve_source_and_exclude_accounts() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        let name = from.path().file_name().unwrap().to_string_lossy();
        tokio::fs::write(
            from.path().join(format!("{name}.json")),
            serde_json::to_vec(&fabric()).unwrap(),
        )
        .await
        .unwrap();
        for folder in ["mods", "shaderpacks", "resourcepacks", "saves/world"] {
            tokio::fs::create_dir_all(from.path().join(folder))
                .await
                .unwrap();
        }
        tokio::fs::write(
            from.path().join("mods/example.jar.disabled"),
            b"disabled",
        )
        .await
        .unwrap();
        tokio::fs::write(from.path().join("saves/world/level.dat"), b"world")
            .await
            .unwrap();
        tokio::fs::write(from.path().join("accounts.json"), b"private")
            .await
            .unwrap();
        let game = preview_folder(from.path().to_owned())
            .await
            .unwrap()
            .remove(0);
        assert_eq!(game.mods, vec!["example.jar.disabled"]);
        assert_eq!(game.worlds, vec!["world"]);
        copy_files(from.path(), to.path(), &|| false, &|_, _| {})
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read(to.path().join("saves/world/level.dat"))
                .await
                .unwrap(),
            b"world"
        );
        assert!(from.path().join("accounts.json").exists());
        assert!(!to.path().join("accounts.json").exists());
        assert!(
            copy_files(from.path(), to.path(), &|| true, &|_, _| {})
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn gecl_registration_is_idempotent_and_does_not_enable_sync() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let root = tempfile::tempdir().unwrap();
        let game = LocalGame {
            source: root.path().to_owned(),
            version_file: root.path().join("game.json"),
            name: "Test".to_owned(),
            game_version: "1.20.1".to_owned(),
            loader: ModLoader::Fabric,
            loader_version: Some("0.19.3".to_owned()),
            mods: vec![],
            shaderpacks: vec![],
            resourcepacks: vec![],
            worlds: vec![],
        };
        let instance = register(&game, "Test", &pool).await.unwrap().unwrap();
        assert!(register(&game, "test", &pool).await.unwrap().is_none());
        let loaded = crate::state::get_instance(&instance.id, &pool)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.applied_content_set.loader_version,
            game.loader_version
        );
        assert_eq!(
            loaded.instance.install_stage,
            InstanceInstallStage::NotInstalled
        );
        assert!(!loaded.synced_options.game_options);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM instance_sync_preferences WHERE enabled != 0").fetch_one(&pool).await.unwrap(),0);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
