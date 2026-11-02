use anyhow::{Context, bail, ensure};
use serde::Serialize;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const MAX_RESTORE_BYTES: u64 = 128 * 1024 * 1024 * 1024;
const LOCATION_KEY: &str = "gecl_world_backup_directory";

#[derive(Clone, Serialize)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

#[derive(Serialize)]
pub struct BackupResult {
    pub path: PathBuf,
    pub recovery: Option<PathBuf>,
}

pub async fn directory() -> crate::Result<Option<String>> {
    let state = crate::State::get().await?;
    Ok(
        sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
            .bind(LOCATION_KEY)
            .fetch_optional(&state.pool)
            .await?,
    )
}

pub async fn run(
    instance_id: String,
    world: String,
    archive: PathBuf,
    restore: bool,
    cancelled: Arc<AtomicBool>,
    progress: impl Fn(Progress) + Send + 'static,
) -> crate::Result<BackupResult> {
    let state = crate::State::get().await?;
    let _guard = state.lock_instance_content(&instance_id).await;
    ensure!(
        !crate::state::process::instance_has_running_process(
            &instance_id,
            &state
        )
        .await?,
        "请先关闭这个游戏版本再操作存档"
    );
    ensure!(
        !crate::launcher::progress::list()
            .iter()
            .any(|launch| launch.instance_id == instance_id
                && launch.finished_at.is_none()),
        "请等待游戏启动结束并关闭游戏"
    );
    let instance = crate::api::instance::get_full_path(&instance_id).await?;
    let saves = instance.join("saves");
    let target = checked_world(&saves, &world)?;
    let lock = crate::api::worlds::get_world_session_lock(&target).await?;
    let parent = archive.parent().context("请选择备份文件位置")?;
    let parent = std::fs::canonicalize(parent)?;
    let saves_canonical = std::fs::canonicalize(&saves)?;
    ensure!(
        !parent.starts_with(&saves_canonical),
        "备份文件不能保存在存档目录里"
    );
    if !restore {
        ensure!(
            !std::fs::symlink_metadata(&archive).is_ok_and(|m| linked(&m)),
            "备份位置不能是链接"
        );
    }
    let backup_directory =
        dunce::simplified(&parent).to_string_lossy().into_owned();
    let result =
        tokio::task::spawn_blocking(move || -> anyhow::Result<BackupResult> {
            if restore {
                let staged =
                    stage_restore(&saves, &archive, &cancelled, &progress)?;
                check_cancel(&cancelled)?;
                let recovery_root = instance.join(".gecl-world-recovery");
                ensure_plain_directory(&recovery_root)?;
                let recovery =
                    recovery_root.join(uuid::Uuid::new_v4().to_string());
                drop(lock);
                std::fs::rename(&target, &recovery)
                    .context("无法保留恢复前的存档")?;
                if let Err(error) =
                    std::fs::rename(staged.path().join("world"), &target)
                {
                    std::fs::rename(&recovery, &target).with_context(|| {
                        format!("恢复失败，旧存档保留在 {}", recovery.display())
                    })?;
                    return Err(error.into());
                }
                Ok(BackupResult {
                    path: target,
                    recovery: Some(recovery),
                })
            } else {
                backup(&target, &archive, &cancelled, &progress)?;
                drop(lock);
                Ok(BackupResult {
                    path: archive,
                    recovery: None,
                })
            }
        })
        .await??;
    sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(LOCATION_KEY).bind(backup_directory).execute(&state.pool).await?;
    Ok(result)
}

fn linked(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn ensure_plain_directory(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        std::fs::create_dir(path)?;
    }
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_dir() && !linked(&metadata),
        "存档路径不能是链接"
    );
    Ok(())
}

fn checked_world(saves: &Path, world: &str) -> anyhow::Result<PathBuf> {
    ensure!(
        !world.is_empty()
            && !world.contains(['/', '\\', ':'])
            && world != "."
            && world != "..",
        "无效存档路径"
    );
    ensure!(saves.is_dir(), "找不到存档目录");
    ensure_plain_directory(saves)?;
    let path = saves.join(world);
    ensure!(path.is_dir(), "找不到这个世界");
    ensure_plain_directory(&path)?;
    ensure!(
        path.join("level.dat").is_file(),
        "这个文件夹没有有效的世界存档"
    );
    Ok(path)
}

fn check_cancel(cancelled: &AtomicBool) -> anyhow::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!("操作已取消");
    }
    Ok(())
}

fn collect(
    root: &Path,
    current: &Path,
    files: &mut Vec<(PathBuf, u64, std::time::SystemTime)>,
    cancelled: &AtomicBool,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)? {
        check_cancel(cancelled)?;
        let entry = entry?;
        let path = entry.path();
        if path == root.join("session.lock") {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        ensure!(
            !linked(&metadata),
            "存档包含链接，无法安全备份：{}",
            path.display()
        );
        if metadata.is_dir() {
            collect(root, &path, files, cancelled)?;
        } else {
            ensure!(metadata.is_file(), "存档包含特殊文件");
            files.push((path, metadata.len(), metadata.modified()?));
        }
    }
    Ok(())
}

fn backup(
    world: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
    progress: &impl Fn(Progress),
) -> anyhow::Result<()> {
    let mut files = Vec::new();
    collect(world, world, &mut files, cancelled)?;
    let total = files.iter().map(|(_, size, _)| size).sum::<u64>();
    let mut temporary = tempfile::NamedTempFile::new_in(
        destination.parent().context("无效备份路径")?,
    )?;
    let mut writer = zip::ZipWriter::new(temporary.as_file_mut());
    let mut buffer = vec![0; 256 * 1024];
    let mut done = 0;
    for (path, size, modified) in files {
        check_cancel(cancelled)?;
        let mut input = std::fs::File::open(&path)?;
        let before = input.metadata()?;
        ensure!(
            before.len() == size && before.modified()? == modified,
            "存档在备份时发生变化，请重试"
        );
        writer.start_file(
            path.strip_prefix(world)?
                .to_string_lossy()
                .replace('\\', "/"),
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .large_file(size > u32::MAX as u64),
        )?;
        let mut copied = 0;
        loop {
            check_cancel(cancelled)?;
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
            copied += read as u64;
            done += read as u64;
            progress(Progress { done, total });
        }
        ensure!(
            copied == size && input.metadata()?.modified()? == modified,
            "存档在备份时发生变化，请重试"
        );
    }
    writer.finish()?;
    temporary.as_file().sync_all()?;
    check_cancel(cancelled)?;
    temporary
        .persist(destination)
        .map_err(|error| error.error)?;
    progress(Progress { done: total, total });
    Ok(())
}

fn safe_entry(name: &str) -> anyhow::Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && !name.contains(['\\', ':'])
            && !name.starts_with('/'),
        "备份包含不安全路径"
    );
    let name = name.trim_end_matches('/');
    for part in name.split('/') {
        ensure!(
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with(['.', ' ']),
            "备份包含不安全路径"
        );
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        ensure!(
            ![
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4",
                "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3",
                "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
            ]
            .contains(&stem.as_str()),
            "备份包含无效文件名"
        );
    }
    let path = PathBuf::from(name);
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "备份包含不安全路径"
    );
    Ok(path)
}

fn stage_restore(
    saves: &Path,
    source: &Path,
    cancelled: &AtomicBool,
    progress: &impl Fn(Progress),
) -> anyhow::Result<tempfile::TempDir> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(source)?)?;
    ensure!(archive.len() <= 1_000_000, "备份文件数量过多");
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        check_cancel(cancelled)?;
        let entry = archive.by_index(i)?;
        safe_entry(entry.name())?;
        ensure!(
            seen.insert(entry.name().trim_end_matches('/').to_lowercase()),
            "备份包含重名文件"
        );
        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "备份包含链接"
        );
        total = total.checked_add(entry.size()).context("备份尺寸无效")?;
        ensure!(total <= MAX_RESTORE_BYTES, "备份解压后超过 128 GB");
    }
    ensure!(seen.contains("level.dat"), "请选择 GeCL 生成的世界备份 ZIP");
    let temporary = tempfile::Builder::new()
        .prefix(".gecl-restore-")
        .tempdir_in(saves.parent().context("无效存档路径")?)?;
    let world = temporary.path().join("world");
    std::fs::create_dir(&world)?;
    let mut buffer = vec![0; 256 * 1024];
    let mut done = 0;
    for i in 0..archive.len() {
        check_cancel(cancelled)?;
        let mut entry = archive.by_index(i)?;
        let relative = safe_entry(entry.name())?;
        if relative == Path::new("session.lock") {
            continue;
        }
        let path = world.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&path)?;
            continue;
        }
        std::fs::create_dir_all(path.parent().context("无效备份路径")?)?;
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        let expected = entry.size();
        let mut copied = 0;
        loop {
            check_cancel(cancelled)?;
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            copied += read as u64;
            ensure!(copied <= expected, "备份文件大小不符");
            output.write_all(&buffer[..read])?;
            done += read as u64;
            progress(Progress { done, total });
        }
        ensure!(copied == expected, "备份文件不完整");
        output.sync_all()?;
    }
    ensure!(world.join("level.dat").is_file(), "备份缺少世界数据");
    progress(Progress { done: total, total });
    Ok(temporary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_restore_round_trip_and_cancel_preserves_output() {
        let root = tempfile::tempdir().unwrap();
        let saves = root.path().join("saves");
        let world = saves.join("中文世界");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"world metadata").unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), b"region bytes")
            .unwrap();
        std::fs::write(world.join("session.lock"), b"lock").unwrap();
        let archive = root.path().join("world.zip");
        let cancelled = AtomicBool::new(false);
        backup(&world, &archive, &cancelled, &|_| {}).unwrap();
        let staged =
            stage_restore(&saves, &archive, &cancelled, &|_| {}).unwrap();
        assert_eq!(
            std::fs::read(staged.path().join("world/region/r.0.0.mca"))
                .unwrap(),
            b"region bytes"
        );
        assert!(!staged.path().join("world/session.lock").exists());
        let bytes = std::fs::read(&archive).unwrap();
        cancelled.store(true, Ordering::Relaxed);
        assert!(backup(&world, &archive, &cancelled, &|_| {}).is_err());
        assert_eq!(std::fs::read(archive).unwrap(), bytes);
    }
    #[test]
    fn unsafe_backup_paths_are_rejected() {
        for name in [
            "../outside",
            "C:/x",
            "/root",
            "a\\b",
            "a/../b",
            "a/CON",
            "a/x.",
        ] {
            assert!(safe_entry(name).is_err(), "{name}");
        }
        assert!(safe_entry("region/r.0.0.mca").is_ok());
    }
}
