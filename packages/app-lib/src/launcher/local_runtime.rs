use crate::state::content_store::validate_relative;
use crate::util::{content_hash::temporary_file, fetch::sha1_file_async};
use daedalus::minecraft::{AssetsIndex, DownloadType, VersionInfo};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

fn valid_hash(hash: &str) -> bool {
    hash.len() == 40 && hash.bytes().all(|c| c.is_ascii_hexdigit())
}

async fn ordinary_file(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut current = root.to_path_buf();
    for part in relative.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return false;
        }
        current.push(part);
        let Ok(meta) = tokio::fs::symlink_metadata(&current).await else {
            return false;
        };
        if linked(&meta) {
            return false;
        }
    }
    tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file())
}

async fn verified(path: &Path, hash: &str, size: u64) -> bool {
    let Ok(meta) = tokio::fs::symlink_metadata(path).await else {
        return false;
    };
    if !meta.is_file() || linked(&meta) || meta.len() != size {
        return false;
    }
    sha1_file_async(path).await.is_ok_and(|(length, digest)| {
        length == size && digest.eq_ignore_ascii_case(hash)
    })
}

/// Publish only a complete, verified copy. Dropping the future also drops its temporary file.
async fn reuse_file(
    target: &Path,
    candidates: &[PathBuf],
    roots: &[PathBuf],
    hash: &str,
    size: u64,
) -> crate::Result<bool> {
    if !valid_hash(hash) {
        return Ok(false);
    }
    if verified(target, hash, size).await {
        return Ok(true);
    }
    for source in candidates {
        let mut allowed = false;
        for root in roots {
            if ordinary_file(source, root).await {
                allowed = true;
                break;
            }
        }
        if !allowed {
            continue;
        }
        let Ok(mut input) = tokio::fs::File::open(source).await else {
            continue;
        };
        if input.metadata().await?.len() != size {
            continue;
        }
        let parent = target
            .parent()
            .ok_or_else(|| anyhow::anyhow!("运行文件路径无效"))?;
        tokio::fs::create_dir_all(parent).await?;
        let (mut output, temporary) = temporary_file(Some(parent)).await?;
        let mut hasher = sha1_smol::Sha1::new();
        let mut length = 0u64;
        let mut buffer = vec![0; 256 * 1024];
        loop {
            let read = input.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            length += read as u64;
            if length > size {
                break;
            }
            hasher.update(&buffer[..read]);
            output.write_all(&buffer[..read]).await?;
        }
        if length != size || !hasher.hexdigest().eq_ignore_ascii_case(hash) {
            drop(output);
            continue;
        }
        output.sync_all().await?;
        drop(output);
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(move || temporary.persist(target))
            .await?
            .map_err(|error| error.error)?;
        return Ok(true);
    }
    // Invalid files in our runtime cache must not satisfy the downloader's existence check.
    if let Ok(meta) = tokio::fs::symlink_metadata(target).await {
        if meta.is_file() && !linked(&meta) {
            tokio::fs::remove_file(target).await?;
        }
    }
    Ok(false)
}

async fn sources(
    instance: &Path,
) -> crate::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut candidates = vec![instance.to_path_buf()];
    let mut jars = Vec::new();
    let marker = instance.join("gecl-profile.json");
    if let Ok(meta) = tokio::fs::symlink_metadata(&marker).await {
        if meta.is_file() && !linked(&meta) && meta.len() < 2_000_000 {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(
                &tokio::fs::read(marker).await?,
            ) {
                if value.get("geclFormat").and_then(|v| v.as_u64()) == Some(1) {
                    if let Some(source) =
                        value.get("runtimeSource").and_then(|v| v.as_str())
                    {
                        candidates.push(PathBuf::from(source));
                    }
                    if let Some(file) =
                        value.get("runtimeVersionFile").and_then(|v| v.as_str())
                    {
                        jars.push(PathBuf::from(file).with_extension("jar"));
                    }
                }
            }
        }
    }
    let mut roots = Vec::new();
    for candidate in candidates {
        let Ok(meta) = tokio::fs::symlink_metadata(&candidate).await else {
            continue;
        };
        if !meta.is_dir() || linked(&meta) {
            continue;
        }
        let root = tokio::fs::canonicalize(candidate).await?;
        if root
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == "versions")
        {
            if let Some(shared) = root.parent().and_then(Path::parent) {
                roots.push(shared.to_path_buf());
            }
        }
        roots.push(root);
    }
    roots.sort();
    roots.dedup();
    Ok((roots, jars))
}

/// Seed the shared runtime cache from an imported folder using resolved manifest hashes.
pub(super) async fn reuse(
    instance: &Path,
    version: &VersionInfo,
    metadata: &Path,
) -> crate::Result<()> {
    let (roots, jars) = sources(instance).await?;
    reuse_from_roots(&roots, &jars, version, metadata).await
}

pub(super) async fn reuse_assets(
    instance: &Path,
    index: &AssetsIndex,
    metadata: &Path,
) -> crate::Result<()> {
    let (roots, _) = sources(instance).await?;
    let (files, bytes) = reuse_asset_objects(&roots, index, metadata).await?;
    tracing::info!(
        files,
        bytes,
        "Verified local Minecraft assets available before download"
    );
    Ok(())
}

async fn reuse_asset_objects(
    roots: &[PathBuf],
    index: &AssetsIndex,
    metadata: &Path,
) -> crate::Result<(u64, u64)> {
    let mut files = 0;
    let mut bytes = 0;
    for asset in index.objects.values() {
        if !valid_hash(&asset.hash) {
            continue;
        }
        let relative = PathBuf::from("assets/objects")
            .join(&asset.hash[..2])
            .join(&asset.hash);
        let candidates =
            roots.iter().map(|r| r.join(&relative)).collect::<Vec<_>>();
        if reuse_file(
            &metadata.join(relative),
            &candidates,
            roots,
            &asset.hash,
            asset.size as u64,
        )
        .await?
        {
            files += 1;
            bytes += asset.size as u64;
        }
    }
    Ok((files, bytes))
}

async fn reuse_from_roots(
    roots: &[PathBuf],
    extra_jars: &[PathBuf],
    version: &VersionInfo,
    metadata: &Path,
) -> crate::Result<()> {
    validate_relative(&version.id)?;
    validate_relative(&version.asset_index.id)?;
    if let Some(client) = version.downloads.get(&DownloadType::Client) {
        let mut jars = extra_jars.to_vec();
        for root in roots {
            let mut entries = tokio::fs::read_dir(root).await?;
            let mut count = 0;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().is_some_and(|e| e == "jar") {
                    jars.push(entry.path());
                }
                count += 1;
                if count >= 256 {
                    break;
                }
            }
        }
        let target = metadata
            .join("versions")
            .join(&version.id)
            .join(format!("{}.jar", version.id));
        reuse_file(&target, &jars, roots, &client.sha1, client.size as u64)
            .await?;
    }
    for library in &version.libraries {
        let Some(artifact) =
            library.downloads.as_ref().and_then(|d| d.artifact.as_ref())
        else {
            continue;
        };
        if !valid_hash(&artifact.sha1) {
            continue;
        }
        let relative = daedalus::get_path_from_artifact(&library.name)?;
        validate_relative(&relative)?;
        let candidates = roots
            .iter()
            .map(|r| r.join("libraries").join(&relative))
            .collect::<Vec<_>>();
        reuse_file(
            &metadata.join("libraries").join(relative),
            &candidates,
            roots,
            &artifact.sha1,
            artifact.size as u64,
        )
        .await?;
    }
    let relative = PathBuf::from("assets/indexes")
        .join(format!("{}.json", version.asset_index.id));
    let candidates =
        roots.iter().map(|r| r.join(&relative)).collect::<Vec<_>>();
    let index_path = metadata.join(&relative);
    reuse_file(
        &index_path,
        &candidates,
        roots,
        &version.asset_index.sha1,
        version.asset_index.size as u64,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(bytes: &[u8]) -> String {
        sha1_smol::Sha1::from(bytes).hexdigest()
    }

    #[tokio::test]
    async fn changed_index_reuses_shared_objects_from_original_import_source() {
        let source = tempfile::tempdir().unwrap();
        let imported = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let game = source.path().join("versions/Renamed Pack");
        tokio::fs::create_dir_all(&game).await.unwrap();
        let unchanged = b"unchanged asset";
        let old = b"old translation";
        let updated = b"new translation";
        let client = b"original client";
        let old_index = serde_json::to_vec(&serde_json::json!({"objects":{"sound":{"hash":digest(unchanged),"size":unchanged.len()},"language":{"hash":digest(old),"size":old.len()}}})).unwrap();
        let current = serde_json::to_vec(&serde_json::json!({"objects":{"sound":{"hash":digest(unchanged),"size":unchanged.len()},"language":{"hash":digest(updated),"size":updated.len()}}})).unwrap();
        tokio::fs::create_dir_all(source.path().join("assets/indexes"))
            .await
            .unwrap();
        tokio::fs::write(
            source.path().join("assets/indexes/5.json"),
            &old_index,
        )
        .await
        .unwrap();
        for bytes in [unchanged.as_slice(), old.as_slice()] {
            let hash = digest(bytes);
            let path = source
                .path()
                .join("assets/objects")
                .join(&hash[..2])
                .join(&hash);
            tokio::fs::create_dir_all(path.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::write(path, bytes).await.unwrap();
        }
        let descriptor = game.join("Renamed Pack.json");
        tokio::fs::write(&descriptor, b"{}").await.unwrap();
        tokio::fs::write(game.join("Renamed Pack.jar"), client)
            .await
            .unwrap();
        let marker = serde_json::json!({"geclFormat":1,"runtimeSource":tokio::fs::canonicalize(&game).await.unwrap(),"runtimeVersionFile":tokio::fs::canonicalize(&descriptor).await.unwrap()});
        tokio::fs::write(
            imported.path().join("gecl-profile.json"),
            serde_json::to_vec(&marker).unwrap(),
        )
        .await
        .unwrap();
        let version: VersionInfo = serde_json::from_value(serde_json::json!({"id":"1.20.1-test","assetIndex":{"id":"5","sha1":digest(&current),"size":current.len(),"totalSize":100,"url":"http://127.0.0.1:1/never"},"assets":"5","downloads":{"client":{"sha1":digest(client),"size":client.len(),"url":"http://127.0.0.1:1/never"}},"libraries":[],"mainClass":"net.minecraft.client.main.Main","minimumLauncherVersion":21,"releaseTime":"2023-06-12T00:00:00Z","time":"2023-06-12T00:00:00Z","type":"release"})).unwrap();
        reuse(imported.path(), &version, cache.path())
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read(
                cache.path().join("versions/1.20.1-test/1.20.1-test.jar")
            )
            .await
            .unwrap(),
            client
        );
        assert!(!cache.path().join("assets/indexes/5.json").exists());
        tokio::fs::create_dir_all(cache.path().join("assets/indexes"))
            .await
            .unwrap();
        tokio::fs::write(cache.path().join("assets/indexes/5.json"), &current)
            .await
            .unwrap();
        let current_index = serde_json::from_slice(&current).unwrap();
        reuse_assets(imported.path(), &current_index, cache.path())
            .await
            .unwrap();
        let hash = digest(unchanged);
        assert_eq!(
            tokio::fs::read(
                cache
                    .path()
                    .join("assets/objects")
                    .join(&hash[..2])
                    .join(hash)
            )
            .await
            .unwrap(),
            unchanged
        );
        let hash = digest(updated);
        assert!(
            !cache
                .path()
                .join("assets/objects")
                .join(&hash[..2])
                .join(hash)
                .exists()
        );
        assert_eq!(
            tokio::fs::read(source.path().join("assets/indexes/5.json"))
                .await
                .unwrap(),
            old_index
        );
        assert_eq!(
            tokio::fs::read(cache.path().join("assets/indexes/5.json"))
                .await
                .unwrap(),
            current
        );
    }

    #[tokio::test]
    #[ignore = "explicit read-only source, version and current-index paths required"]
    async fn imported_runtime_real_source_audit() {
        let source = PathBuf::from(
            std::env::var_os("GECL_RUNTIME_AUDIT_SOURCE").expect("source"),
        );
        let version_file = PathBuf::from(
            std::env::var_os("GECL_RUNTIME_AUDIT_VERSION").expect("version"),
        );
        let index_file = PathBuf::from(
            std::env::var_os("GECL_RUNTIME_AUDIT_INDEX").expect("index"),
        );
        let version: VersionInfo = serde_json::from_slice(
            &tokio::fs::read(version_file).await.unwrap(),
        )
        .unwrap();
        let index_bytes = tokio::fs::read(index_file).await.unwrap();
        assert_eq!(digest(&index_bytes), version.asset_index.sha1);
        let index: AssetsIndex = serde_json::from_slice(&index_bytes).unwrap();
        let imported = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let marker = serde_json::json!({"geclFormat":1,"runtimeSource":source});
        tokio::fs::write(
            imported.path().join("gecl-profile.json"),
            serde_json::to_vec(&marker).unwrap(),
        )
        .await
        .unwrap();
        reuse(imported.path(), &version, cache.path())
            .await
            .unwrap();
        assert!(
            !cache
                .path()
                .join("assets/indexes")
                .join(format!("{}.json", version.asset_index.id))
                .exists(),
            "Audit requires the differing source index that triggered the bug"
        );
        tokio::fs::create_dir_all(cache.path().join("assets/indexes"))
            .await
            .unwrap();
        tokio::fs::write(
            cache
                .path()
                .join("assets/indexes")
                .join(format!("{}.json", version.asset_index.id)),
            &index_bytes,
        )
        .await
        .unwrap();
        reuse_assets(imported.path(), &index, cache.path())
            .await
            .unwrap();
        let client = &version.downloads[&DownloadType::Client];
        assert!(
            verified(
                &cache
                    .path()
                    .join("versions")
                    .join(&version.id)
                    .join(format!("{}.jar", version.id)),
                &client.sha1,
                client.size as u64
            )
            .await
        );
        let mut available = 0u64;
        let mut missing = 0u64;
        let mut files = 0u64;
        for asset in index.objects.values() {
            let relative = PathBuf::from("assets/objects")
                .join(&asset.hash[..2])
                .join(&asset.hash);
            if verified(
                &cache.path().join(relative),
                &asset.hash,
                asset.size as u64,
            )
            .await
            {
                files += 1;
                available += asset.size as u64;
            } else {
                missing += asset.size as u64;
            }
        }
        assert!(available > 0);
        eprintln!(
            "RUNTIME_AUDIT {}",
            serde_json::json!({"verified_files":files,"verified_bytes":available,"missing_bytes":missing,"required_bytes":available+missing,"client_reused":true,"source_read_only":true,"network_requests":0})
        );
    }

    #[tokio::test]
    async fn imported_runtime_rejects_paths_outside_source_and_keeps_valid_cache()
     {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        tokio::fs::write(outside.path(), b"verified").await.unwrap();
        let target = root.path().join("cache/file.jar");
        let hash = digest(b"verified");
        assert!(
            !reuse_file(
                &target,
                &[outside.path().to_path_buf()],
                &[root.path().to_path_buf()],
                &hash,
                8
            )
            .await
            .unwrap()
        );
        assert!(!target.exists());
        let source = root.path().join("original.jar");
        tokio::fs::write(&source, b"verified").await.unwrap();
        assert!(
            reuse_file(
                &target,
                &[source.clone()],
                &[root.path().to_path_buf()],
                &hash,
                8
            )
            .await
            .unwrap()
        );
        let before = tokio::fs::metadata(&target)
            .await
            .unwrap()
            .modified()
            .unwrap();
        tokio::fs::write(source, b"modified").await.unwrap();
        assert!(reuse_file(&target, &[], &[], &hash, 8).await.unwrap());
        assert_eq!(
            tokio::fs::metadata(target)
                .await
                .unwrap()
                .modified()
                .unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn imported_runtime_reuses_verified_files_and_leaves_missing_files_for_download()
     {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = source.path();
        let client = b"official-client";
        let library = b"official-library";
        let asset = b"official-sound";
        let missing = b"not-installed";
        let hash = digest(asset);
        let index = serde_json::to_vec(&serde_json::json!({"objects": {"sound.ogg":{"hash":hash,"size":asset.len()},"missing.ogg":{"hash":digest(missing),"size":missing.len()}}})).unwrap();
        let asset_path =
            PathBuf::from("assets/objects").join(&hash[..2]).join(&hash);
        let lib_path =
            PathBuf::from("libraries/example/library/1/library-1.jar");
        for (relative, bytes) in [
            (PathBuf::from("renamed-pack.jar"), client.as_slice()),
            (lib_path.clone(), library.as_slice()),
            (PathBuf::from("assets/indexes/test.json"), index.as_slice()),
            (asset_path.clone(), asset.as_slice()),
        ] {
            let path = root.join(relative);
            tokio::fs::create_dir_all(path.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::write(path, bytes).await.unwrap();
        }
        let version: VersionInfo = serde_json::from_value(serde_json::json!({"id":"1.20.1-0.19.3","assetIndex":{"id":"test","sha1":digest(&index),"size":index.len(),"totalSize":asset.len(),"url":"http://127.0.0.1:1/never"},"assets":"test","downloads":{"client":{"sha1":digest(client),"size":client.len(),"url":"http://127.0.0.1:1/never"}},"libraries":[{"name":"example:library:1","downloads":{"artifact":{"sha1":digest(library),"size":library.len(),"url":"http://127.0.0.1:1/never"}}}],"mainClass":"net.minecraft.client.main.Main","minimumLauncherVersion":21,"releaseTime":"2023-06-12T00:00:00Z","time":"2023-06-12T00:00:00Z","type":"release"})).unwrap();
        reuse_from_roots(&[root.to_path_buf()], &[], &version, cache.path())
            .await
            .unwrap();
        let resolved_index = serde_json::from_slice(&index).unwrap();
        reuse_asset_objects(
            &[root.to_path_buf()],
            &resolved_index,
            cache.path(),
        )
        .await
        .unwrap();
        let target = cache
            .path()
            .join("versions/1.20.1-0.19.3/1.20.1-0.19.3.jar");
        assert_eq!(tokio::fs::read(&target).await.unwrap(), client);
        assert_eq!(
            tokio::fs::read(cache.path().join(lib_path)).await.unwrap(),
            library
        );
        assert_eq!(
            tokio::fs::read(cache.path().join(asset_path))
                .await
                .unwrap(),
            asset
        );
        assert!(
            !cache
                .path()
                .join("assets/objects")
                .join(&digest(missing)[..2])
                .join(digest(missing))
                .exists()
        );
        tokio::fs::write(&target, b"invalid-client!").await.unwrap();
        reuse_from_roots(&[root.to_path_buf()], &[], &version, cache.path())
            .await
            .unwrap();
        assert_eq!(tokio::fs::read(&target).await.unwrap(), client);
        tokio::fs::write(root.join("renamed-pack.jar"), b"corrupt-client!")
            .await
            .unwrap();
        tokio::fs::write(&target, b"corrupt-client!").await.unwrap();
        reuse_from_roots(&[root.to_path_buf()], &[], &version, cache.path())
            .await
            .unwrap();
        assert!(!target.exists());
        assert_eq!(
            tokio::fs::read(root.join("renamed-pack.jar"))
                .await
                .unwrap(),
            b"corrupt-client!"
        );
        assert!(
            !std::fs::read_dir(cache.path().join("versions/1.20.1-0.19.3"))
                .unwrap()
                .any(|e| e
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".tmp"))
        );
    }
}
