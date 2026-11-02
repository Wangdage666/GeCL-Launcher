use anyhow::{Context, bail};
use bytes::Bytes;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Serialize)]
pub struct PackArtwork {
    pub name: String,
    pub icon: Option<PathBuf>,
}

pub async fn cache(source: PathBuf) -> crate::Result<PathBuf> {
    let state = crate::State::get().await?;
    crate::api::instance::cache_icon_from_path(&source, &state).await
}

pub async fn inspect(source: PathBuf) -> crate::Result<PackArtwork> {
    let (name, icon) =
        tokio::task::spawn_blocking(move || inspect_archive(&source)).await??;
    let state = crate::State::get().await?;
    let icon = match icon {
        Some(bytes) => {
            match crate::api::instance::cache_icon(Bytes::from(bytes), &state)
                .await
            {
                Ok(path) => Some(path),
                Err(error) => {
                    tracing::debug!(%error, "Ignoring invalid optional pack icon");
                    None
                }
            }
        }
        None => None,
    };
    Ok(PackArtwork { name, icon })
}

fn inspect_archive(source: &Path) -> anyhow::Result<(String, Option<Vec<u8>>)> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(source)?)?;
    let manifest =
        read_entry(&mut archive, "modrinth.index.json", 16 * 1024 * 1024)?
            .context("整合包缺少清单")?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest)?;
    let name = manifest
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.trim().is_empty())
        .context("整合包缺少名称")?
        .chars()
        .take(256)
        .collect();
    let mut icon = None;
    for entry in [
        "icon.png",
        "client-overrides/icon.png",
        "overrides/icon.png",
    ] {
        match read_entry(&mut archive, entry, MAX_ICON_BYTES) {
            Ok(Some(bytes)) => {
                icon = Some(bytes);
                break;
            }
            Ok(None) | Err(_) => continue,
        }
    }
    Ok((name, icon))
}

fn read_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
    limit: u64,
) -> anyhow::Result<Option<Vec<u8>>> {
    let entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if entry.size() > limit {
        bail!("整合包元信息过大");
    }
    let mut bytes = Vec::new();
    entry.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("整合包元信息过大");
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn reads_root_icon_and_pack_name_without_extracting_other_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("pack.mrpack");
        let mut zip =
            zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, bytes) in [
            ("modrinth.index.json", r#"{"name":"自制整合包"}"#.as_bytes()),
            ("icon.png", b"image"),
            ("../../outside", b"ignored"),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        let (name, icon) = inspect_archive(&path).unwrap();
        assert_eq!(name, "自制整合包");
        assert_eq!(icon.unwrap(), b"image");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
