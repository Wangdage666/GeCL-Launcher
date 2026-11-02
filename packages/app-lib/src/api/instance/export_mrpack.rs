use super::get::get;
use super::paths::get_full_path;
use crate::ErrorKind;
use crate::event::LoadingBarType;
use crate::event::emit::{emit_loading, init_loading};
use crate::pack::install_from::{
    EnvType, PackDependency, PackFile, PackFileHash, PackFormat,
};
use crate::state::content_store::{
    FileContent, ReadableContent, content_file_path, is_managed_content_path,
};
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::{
    CacheBehaviour, CachedEntry, InstanceMetadata, ModLoader, SideType, State,
    VersionEnvironment,
};
use crate::util::io::{self, IOError};
use anyhow::{Context, bail};
use futures::{StreamExt, stream};
use path_util::SafeRelativeUtf8UnixPathBuf;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const DEFAULT_SELECTED_EXPORT_PATH_PREFIXES: &[&str] = &[
    "mods",
    "datapacks",
    "resourcepacks",
    "shaderpacks",
    "config",
];
const EXPORT_CANDIDATE_METADATA_CONCURRENCY: usize = 32;
const EXPORT_COPY_BUFFER_SIZE: usize = 256 * 1024;
const STANDARD_ZIP_FILE_SIZE_ERROR: &str = "your modpack cannot be exported as it contains a file over the size limit of 4 GB";

const NEVER_EXPORTABLE_PATH_PREFIXES: &[&str] = &[
    "profile.json",
    "modrinth_logs",
    "mods/.connector",
    ".sable/natives",
    "local/crash_assistant",
    "mods/mcef-libraries",
    "mods/mcef-cache",
    "config/super_resolution/libraries",
    "config/Veinminer/update",
    "config/epicfight/native",
    "essential",
    ".mixin.out",
    ".fabric",
    "__MACOSX",
];
const NEVER_EXPORTABLE_PATH_SUFFIXES: &[&str] = &[".DS_Store"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackExportCandidate {
    pub path: SafeRelativeUtf8UnixPathBuf,
    #[serde(rename = "type")]
    pub kind: PackExportCandidateType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    pub disabled: bool,
    pub default_selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PackExportCandidateType {
    Directory,
    File,
}

#[derive(Default)]
struct ExportSelectionNode {
    selected: Option<bool>,
    has_included_rule: bool,
    children: HashMap<String, ExportSelectionNode>,
}

#[derive(Default)]
struct ExportSelection {
    root: ExportSelectionNode,
}

impl ExportSelection {
    fn new(included_paths: Vec<String>, excluded_paths: Vec<String>) -> Self {
        let mut rules = HashMap::new();

        for (paths, selected) in
            [(included_paths, true), (excluded_paths, false)]
        {
            for path in paths {
                let Ok(path) = SafeRelativeUtf8UnixPathBuf::try_from(path)
                else {
                    continue;
                };
                if path.as_str().is_empty() || !is_path_exportable(&path) {
                    continue;
                }
                rules.insert(path.as_str().to_string(), selected);
            }
        }

        let mut selection = Self::default();
        for (path, selected) in rules {
            selection.root.insert(&path, selected);
        }
        selection
    }

    fn is_included(&self, path: &SafeRelativeUtf8UnixPathBuf) -> bool {
        self.resolve(path).0
    }

    fn should_visit_directory(
        &self,
        path: &SafeRelativeUtf8UnixPathBuf,
    ) -> bool {
        let (selected, node) = self.resolve(path);
        selected || node.is_some_and(|node| node.has_included_rule)
    }

    fn resolve(
        &self,
        path: &SafeRelativeUtf8UnixPathBuf,
    ) -> (bool, Option<&ExportSelectionNode>) {
        let mut node = &self.root;
        let mut selected = node.selected.unwrap_or(false);

        for segment in path.as_str().split('/') {
            let Some(child) = node.children.get(segment) else {
                return (selected, None);
            };
            node = child;
            selected = node.selected.unwrap_or(selected);
        }

        (selected, Some(node))
    }
}

impl ExportSelectionNode {
    fn insert(&mut self, path: &str, selected: bool) {
        if selected {
            self.has_included_rule = true;
        }

        let mut node = self;
        for segment in path.split('/') {
            node = node.children.entry(segment.to_string()).or_default();
            if selected {
                node.has_included_rule = true;
            }
        }
        node.selected = Some(selected);
    }
}

#[tracing::instrument(skip_all)]
pub async fn export_mrpack(
    instance_id: &str,
    export_path: PathBuf,
    included_export_candidates: Vec<String>,
    excluded_export_candidates: Vec<String>,
    version_id: Option<String>,
    description: Option<String>,
    _name: Option<String>,
) -> crate::Result<()> {
    export_mrpack_with_icon(
        instance_id,
        export_path,
        included_export_candidates,
        excluded_export_candidates,
        version_id,
        description,
        _name,
        None,
    )
    .await
}

pub async fn export_mrpack_with_icon(
    instance_id: &str,
    export_path: PathBuf,
    included_export_candidates: Vec<String>,
    excluded_export_candidates: Vec<String>,
    version_id: Option<String>,
    description: Option<String>,
    name: Option<String>,
    icon_path: Option<PathBuf>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let destination = Path::new(&export_path);
    let parent = destination.parent().ok_or(ErrorKind::InputError(
        "invalid export destination".to_string(),
    ))?;
    let parent = tokio::fs::canonicalize(parent).await?;
    let store = tokio::fs::canonicalize(state.directories.store_dir()).await?;
    let profiles =
        tokio::fs::canonicalize(state.directories.instances_dir()).await?;
    if parent.starts_with(store)
        || parent.starts_with(profiles)
        || tokio::fs::symlink_metadata(destination)
            .await
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        bail!(ErrorKind::InputError(
            "export to a regular file outside the store and instance directories"
                .to_string(),
        ));
    }
    let _permit: tokio::sync::SemaphorePermit =
        state.io_semaphore.0.acquire().await?;
    let metadata = get(instance_id).await?.context(format!(
        "tried to export a nonexistent instance {instance_id}"
    ))?;
    let export_selection = ExportSelection::new(
        included_export_candidates,
        excluded_export_candidates,
    );

    let version_id = version_id.unwrap_or("1.0.0".to_string());
    let (mut packfile, override_files) = prepare_export(
        &metadata,
        version_id,
        description,
        &export_selection,
        &state,
    )
    .await?;

    if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
        packfile.name = name;
    }
    let selected_icon = icon_path
        .or_else(|| metadata.instance.icon_path.as_deref().map(PathBuf::from));
    let icon = if let Some(path) = selected_icon {
        let cached = super::cache_icon_from_path(&path, &state).await?;
        Some(tokio::fs::read(cached).await?)
    } else {
        None
    };

    let total_bytes = override_files
        .iter()
        .fold(1_u64, |total, file| total.saturating_add(file.size));
    let loading_bar = init_loading(
        LoadingBarType::PackExport {
            instance_id: metadata.instance.id.clone(),
            instance_name: metadata.instance.name.clone(),
        },
        total_bytes as f64,
        "Exporting instance to .mrpack",
    )
    .await?;
    let data = serde_json::to_vec_pretty(&packfile)?;
    tokio::task::spawn_blocking(move || {
        write_export_archive(
            &export_path,
            override_files,
            &data,
            icon.as_deref(),
            |bytes_written| {
                emit_loading(&loading_bar, bytes_written as f64, None)
            },
        )
    })
    .await??;

    Ok(())
}

fn ensure_standard_zip_file_size(size: u64) -> crate::Result<()> {
    if size > zip::ZIP64_BYTES_THR {
        bail!(STANDARD_ZIP_FILE_SIZE_ERROR);
    }

    Ok(())
}

fn write_mrpack_archive<W, F>(
    writer: W,
    override_files: Vec<ExportSnapshot>,
    packfile_data: &[u8],
    icon: Option<&[u8]>,
    mut emit_progress: F,
) -> crate::Result<()>
where
    W: Write + Seek,
    F: FnMut(u64) -> crate::Result<()>,
{
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    let mut writer = ZipWriter::new(writer);
    let mut buffer = vec![0_u8; EXPORT_COPY_BUFFER_SIZE];

    for file in override_files {
        let path: &Path = file.snapshot.as_ref();
        let relative_path = &file.path;
        writer
            .start_file(format!("overrides/{relative_path}"), options)
            .map_err(std::io::Error::from)?;
        let mut source = std::fs::File::open(path)
            .map_err(|error| IOError::with_path(error, path))?;
        loop {
            let bytes_read = source
                .read(&mut buffer)
                .map_err(|error| IOError::with_path(error, path))?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buffer[..bytes_read])?;
            emit_progress(bytes_read as u64)?;
        }
    }

    if let Some(icon) = icon {
        writer
            .start_file("icon.png", options)
            .map_err(std::io::Error::from)?;
        writer.write_all(icon)?;
    }

    writer
        .start_file("modrinth.index.json", options)
        .map_err(std::io::Error::from)?;
    writer.write_all(packfile_data)?;
    writer.finish().map_err(std::io::Error::from)?;
    emit_progress(1)?;

    Ok(())
}

fn is_path_exportable(relative_path: &SafeRelativeUtf8UnixPathBuf) -> bool {
    let path = relative_path.as_str();

    !NEVER_EXPORTABLE_PATH_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }) && !NEVER_EXPORTABLE_PATH_SUFFIXES
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

#[tracing::instrument]
pub async fn get_pack_export_candidates(
    instance_id: &str,
) -> crate::Result<Vec<PackExportCandidate>> {
    let state = State::get().await?;
    crate::state::instances::commands::sync_content_files(instance_id, &state)
        .await?;
    get_pack_export_candidates_for_parent(instance_id, None).await
}

#[tracing::instrument]
pub async fn get_pack_export_candidates_for_parent(
    instance_id: &str,
    parent: Option<SafeRelativeUtf8UnixPathBuf>,
) -> crate::Result<Vec<PackExportCandidate>> {
    let instance_base_dir = get_full_path(instance_id).await?;
    let parent_dir = if let Some(parent) = parent {
        if parent.as_str().is_empty() || !is_path_exportable(&parent) {
            return Ok(Vec::new());
        }

        instance_base_dir.join(parent.as_str())
    } else {
        instance_base_dir.clone()
    };
    let parent_dir = io::canonicalize(parent_dir)?;
    if !parent_dir.starts_with(&instance_base_dir) {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    let mut read_dir = io::read_dir(&parent_dir).await?;
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &parent_dir))?
    {
        paths.push(entry.path());
    }

    let candidates = stream::iter(paths)
        .map(|path| {
            let instance_base_dir = &instance_base_dir;
            async move {
                build_pack_export_candidate(
                    instance_id,
                    instance_base_dir,
                    &path,
                )
                .await
            }
        })
        .buffer_unordered(EXPORT_CANDIDATE_METADATA_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;
    let mut path_list = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if let Some(candidate) = candidate? {
            path_list.push(candidate);
        }
    }

    Ok(path_list)
}

async fn build_pack_export_candidate(
    instance_id: &str,
    instance_base_dir: &PathBuf,
    path: &PathBuf,
) -> crate::Result<Option<PackExportCandidate>> {
    let relative_path = pack_get_relative_path(instance_base_dir, path)?;
    if !is_path_exportable(&relative_path) {
        return Ok(None);
    }

    let state = State::get().await?;
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|error| IOError::with_path(error, path))?;
    let content = if metadata.is_dir() {
        ReadableContent::Local(path.clone())
    } else {
        let logical_path = logical_content_path(&relative_path)?;
        let file = content_rows::get_instance_file_by_relative_path(
            instance_id,
            logical_path.as_str(),
            &state.pool,
        )
        .await?;
        let Some(content) = export_content(
            &state,
            file.as_ref(),
            path,
            metadata.file_type().is_symlink(),
        )
        .await?
        else {
            return Ok(None);
        };
        content
    };
    let metadata = tokio::fs::metadata(content.path()).await?;
    let relative_path = if metadata.is_file() {
        logical_content_path(&relative_path)?
    } else {
        relative_path
    };
    if !metadata.is_dir() && !metadata.is_file() {
        return Ok(None);
    }

    let kind = if metadata.is_dir() {
        PackExportCandidateType::Directory
    } else {
        PackExportCandidateType::File
    };
    let size = metadata.is_file().then_some(metadata.len());
    let modified = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let default_selected = is_default_selected_export_candidate(&relative_path);

    Ok(Some(PackExportCandidate {
        path: relative_path,
        kind,
        size,
        modified,
        count: None,
        disabled: false,
        default_selected,
    }))
}

fn logical_content_path(
    path: &SafeRelativeUtf8UnixPathBuf,
) -> crate::Result<SafeRelativeUtf8UnixPathBuf> {
    if is_managed_content_path(path.as_str()) {
        Ok(SafeRelativeUtf8UnixPathBuf::try_from(
            path.as_str().trim_end_matches(".disabled").to_string(),
        )?)
    } else {
        Ok(path.clone())
    }
}

async fn export_content(
    state: &State,
    file: Option<&crate::state::InstanceFile>,
    path: &Path,
    is_symlink: bool,
) -> crate::Result<Option<ReadableContent>> {
    if !is_symlink {
        return Ok(Some(ReadableContent::Local(path.to_path_buf())));
    }
    if let Some(file) = file {
        match state.content_store.file_content(file).await? {
            FileContent::Stored { stored_file, .. } => {
                if tokio::fs::canonicalize(path).await?
                    != tokio::fs::canonicalize(&stored_file.path).await?
                {
                    return Ok(None);
                }
                return Ok(Some(ReadableContent::Stored(stored_file)));
            }
            FileContent::Damaged(_) => {
                bail!(ErrorKind::InputError(format!(
                    "repair {} before exporting this instance",
                    file.relative_path
                )));
            }
            FileContent::Unmanaged => {}
        }
    }
    Ok((!is_symlink).then(|| ReadableContent::Local(path.to_path_buf())))
}

fn is_default_selected_export_candidate(
    relative_path: &SafeRelativeUtf8UnixPathBuf,
) -> bool {
    let path = relative_path.as_str();

    DEFAULT_SELECTED_EXPORT_PATH_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn pack_get_relative_path(
    instance_path: &PathBuf,
    path: &PathBuf,
) -> crate::Result<SafeRelativeUtf8UnixPathBuf> {
    Ok(SafeRelativeUtf8UnixPathBuf::try_from(
        path.strip_prefix(instance_path)
            .context(ErrorKind::FSError(format!(
                "path {path:?} does not correspond to an instance"
            )))?
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )?)
}

fn get_mrpack_environment(
    environment: Option<VersionEnvironment>,
) -> HashMap<EnvType, SideType> {
    let (client, server) =
        match environment.unwrap_or(VersionEnvironment::Unknown) {
            VersionEnvironment::ClientAndServer
            | VersionEnvironment::ClientOnlyServerOptional
            | VersionEnvironment::ServerOnly
            | VersionEnvironment::ServerOnlyClientOptional
            | VersionEnvironment::ClientOrServer
            | VersionEnvironment::ClientOrServerPrefersBoth
            | VersionEnvironment::Unknown => {
                (SideType::Required, SideType::Required)
            }
            VersionEnvironment::ClientOnly
            | VersionEnvironment::SingleplayerOnly => {
                (SideType::Required, SideType::Unsupported)
            }
            VersionEnvironment::DedicatedServerOnly => {
                (SideType::Unsupported, SideType::Required)
            }
        };

    HashMap::from([(EnvType::Client, client), (EnvType::Server, server)])
}

#[tracing::instrument(skip_all)]
pub async fn create_mrpack_json(
    metadata: &InstanceMetadata,
    version_id: String,
    description: Option<String>,
) -> crate::Result<PackFormat> {
    let state = State::get().await?;
    let selection = ExportSelection::new(
        DEFAULT_SELECTED_EXPORT_PATH_PREFIXES
            .iter()
            .map(|path| path.to_string())
            .collect(),
        Vec::new(),
    );
    Ok(
        prepare_export(metadata, version_id, description, &selection, &state)
            .await?
            .0,
    )
}

fn pack_metadata(
    metadata: &InstanceMetadata,
    version_id: String,
    description: Option<String>,
) -> crate::Result<PackFormat> {
    let mut dependencies = HashMap::new();
    match (
        metadata.applied_content_set.loader,
        metadata.applied_content_set.loader_version.clone(),
    ) {
        (ModLoader::Forge, Some(v)) => {
            dependencies.insert(PackDependency::Forge, v)
        }
        (ModLoader::NeoForge, Some(v)) => {
            dependencies.insert(PackDependency::NeoForge, v)
        }
        (ModLoader::Fabric, Some(v)) => {
            dependencies.insert(PackDependency::FabricLoader, v)
        }
        (ModLoader::Quilt, Some(v)) => {
            dependencies.insert(PackDependency::QuiltLoader, v)
        }
        (ModLoader::Vanilla, _) => None,
        _ => {
            bail!("loader version mismatch");
        }
    };
    dependencies.insert(
        PackDependency::Minecraft,
        metadata.applied_content_set.game_version.clone(),
    );

    Ok(PackFormat {
        game: "minecraft".to_string(),
        format_version: 1,
        version_id,
        name: metadata.instance.name.clone(),
        summary: description,
        files: Vec::new(),
        dependencies,
    })
}

struct ExportSnapshot {
    path: SafeRelativeUtf8UnixPathBuf,
    snapshot: tempfile::TempPath,
    size: u64,
    sha1: String,
    sha512: String,
}

async fn collect_export_snapshots(
    instance_id: &str,
    selection: &ExportSelection,
    state: &State,
) -> crate::Result<Vec<ExportSnapshot>> {
    let base = get_full_path(instance_id).await?;
    let registered = content_rows::get_instance_files(instance_id, &state.pool)
        .await?
        .into_iter()
        .map(|file| (content_file_path(&file), file))
        .collect::<HashMap<_, _>>();
    let mut files = Vec::new();
    let mut directories = vec![base.clone()];
    while let Some(directory) = directories.pop() {
        let mut entries = io::read_dir(&directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let relative = pack_get_relative_path(&base, &path)?;
            if !is_path_exportable(&relative) {
                continue;
            }
            let kind = entry.file_type().await?;
            if kind.is_dir() {
                if selection.should_visit_directory(&relative) {
                    directories.push(path);
                }
                continue;
            }
            if (!kind.is_file() && !kind.is_symlink())
                || !selection.is_included(&logical_content_path(&relative)?)
            {
                continue;
            }
            let Some(content) = export_content(
                state,
                registered.get(relative.as_str()),
                &path,
                kind.is_symlink(),
            )
            .await?
            else {
                continue;
            };
            ensure_standard_zip_file_size(
                tokio::fs::metadata(content.path()).await?.len(),
            )?;
            let staged = state.content_store.stage_file(content.path()).await?;
            let (_, sha1) =
                crate::util::fetch::sha1_file_async(&staged.path).await?;
            files.push(ExportSnapshot {
                path: relative,
                snapshot: staged.path,
                size: staged.size,
                sha1,
                sha512: staged.sha512,
            });
        }
    }
    files.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    Ok(files)
}

async fn prepare_export(
    metadata: &InstanceMetadata,
    version_id: String,
    description: Option<String>,
    selection: &ExportSelection,
    state: &State,
) -> crate::Result<(PackFormat, Vec<ExportSnapshot>)> {
    let mut pack = pack_metadata(metadata, version_id, description)?;
    let files =
        collect_export_snapshots(&metadata.instance.id, selection, state)
            .await?;
    let origins =
        crate::curseforge::saved_files(&metadata.instance.id, &state.pool)
            .await?;
    let mut unidentified = Vec::new();
    for file in files {
        let reference = is_managed_content_path(file.path.as_str())
            .then(|| origins.get(&file.sha512))
            .flatten()
            .and_then(|origin| curseforge_reference(&file, origin));
        if let Some(reference) = reference {
            pack.files.push(reference);
        } else {
            unidentified.push(file);
        }
    }
    let hashes = unidentified
        .iter()
        .filter(|file| is_managed_content_path(file.path.as_str()))
        .map(|file| file.sha1.as_str())
        .collect::<HashSet<_>>();
    let hashes = hashes.into_iter().collect::<Vec<_>>();
    let recognized = if hashes.is_empty() {
        Vec::new()
    } else {
        CachedEntry::get_file_many(&hashes, Some(CacheBehaviour::MustRevalidate), &state.pool, &state.api_semaphore)
            .await.unwrap_or_else(|error| { tracing::warn!("Could not identify export files; embedding local snapshots: {error}"); Vec::new() })
    };
    let ids = recognized
        .iter()
        .map(|file| file.version_id.as_str())
        .collect::<HashSet<_>>();
    let ids = ids.into_iter().collect::<Vec<_>>();
    let versions = if ids.is_empty() {
        Vec::new()
    } else {
        CachedEntry::get_version_v3_many(&ids, Some(CacheBehaviour::MustRevalidate), &state.pool, &state.api_semaphore)
            .await.unwrap_or_else(|error| { tracing::warn!("Could not resolve export downloads; embedding local snapshots: {error}"); Vec::new() })
    };
    let mut embedded = Vec::new();
    for file in unidentified {
        let reference = if is_managed_content_path(file.path.as_str()) {
            versions
                .iter()
                .find_map(|version| modrinth_reference(&file, version))
        } else {
            None
        };
        if let Some(reference) = reference {
            pack.files.push(reference);
        } else {
            embedded.push(file);
        }
    }
    Ok((pack, embedded))
}

fn modrinth_reference(
    snapshot: &ExportSnapshot,
    version: &crate::state::VersionV3,
) -> Option<PackFile> {
    let file =
        version.files.iter().find(|file| {
            u64::from(file.size) == snapshot.size
                && file.hashes.get("sha1").is_some_and(|hash| {
                    hash.eq_ignore_ascii_case(&snapshot.sha1)
                })
                && file.hashes.get("sha512").is_some_and(|hash| {
                    hash.eq_ignore_ascii_case(&snapshot.sha512)
                })
                && valid_download_url(&file.url)
        })?;
    Some(snapshot_reference(
        snapshot,
        vec![file.url.clone()],
        Some(get_mrpack_environment(version.environment)),
    ))
}

fn curseforge_reference(
    snapshot: &ExportSnapshot,
    origin: &crate::curseforge::SavedFile,
) -> Option<PackFile> {
    if origin.file.file_length != snapshot.size
        || crate::curseforge::sha1(&origin.file)
            .ok()?
            .to_ascii_lowercase()
            != snapshot.sha1
    {
        return None;
    }
    let downloads = crate::curseforge::download_urls(&origin.file).ok()?;
    Some(snapshot_reference(snapshot, downloads, None))
}

fn valid_download_url(raw: &str) -> bool {
    url::Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
    })
}

fn snapshot_reference(
    snapshot: &ExportSnapshot,
    downloads: Vec<String>,
    env: Option<HashMap<EnvType, SideType>>,
) -> PackFile {
    PackFile {
        path: snapshot.path.clone(),
        hashes: HashMap::from([
            (PackFileHash::Sha1, snapshot.sha1.clone()),
            (PackFileHash::Sha512, snapshot.sha512.clone()),
        ]),
        env,
        downloads,
        file_size: snapshot.size as u32,
    }
}

fn write_export_archive<F>(
    destination: &Path,
    files: Vec<ExportSnapshot>,
    index: &[u8],
    icon: Option<&[u8]>,
    progress: F,
) -> crate::Result<()>
where
    F: FnMut(u64) -> crate::Result<()>,
{
    let parent = destination.parent().context("invalid export destination")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write_mrpack_archive(
        temporary.as_file_mut(),
        files,
        index,
        icon,
        progress,
    )?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(destination)
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha512};

    fn snapshot(path: &str, bytes: &[u8]) -> ExportSnapshot {
        let mut temporary = tempfile::NamedTempFile::new().unwrap();
        temporary.write_all(bytes).unwrap();
        ExportSnapshot {
            path: path.to_string().try_into().unwrap(),
            snapshot: temporary.into_temp_path(),
            size: bytes.len() as u64,
            sha1: sha1_smol::Sha1::from(bytes).hexdigest(),
            sha512: format!("{:x}", Sha512::digest(bytes)),
        }
    }

    fn version(file: &ExportSnapshot) -> crate::state::VersionV3 {
        serde_json::from_value(serde_json::json!({
            "id": "online-version",
            "files": [{"hashes":{"sha1":file.sha1,"sha512":file.sha512},
                "url":"https://cdn.modrinth.com/data/project/version/mod.jar",
                "filename":"original.jar","primary":true,"size":file.size}]
        }))
        .unwrap()
    }

    #[test]
    fn references_require_current_bytes_and_preserve_local_disabled_name() {
        let file = snapshot("mods/renamed.jar.disabled", b"original bytes");
        let remote = version(&file);
        let reference = modrinth_reference(&file, &remote).unwrap();
        assert_eq!(reference.path.as_str(), "mods/renamed.jar.disabled");
        let edited = snapshot("mods/renamed.jar.disabled", b"modified bytes");
        assert!(modrinth_reference(&edited, &remote).is_none());
        for field in ["sha1", "sha512"] {
            let mut invalid = remote.clone();
            invalid.files[0].hashes.remove(field);
            assert!(modrinth_reference(&file, &invalid).is_none());
        }
        let mut invalid = remote.clone();
        invalid.files[0].size += 1;
        assert!(modrinth_reference(&file, &invalid).is_none());
        for url in [
            "http://example.com/mod.jar",
            "https://user:password@example.com/mod.jar",
        ] {
            invalid = remote.clone();
            invalid.files[0].url = url.into();
            assert!(modrinth_reference(&file, &invalid).is_none());
        }
    }

    #[test]
    fn curseforge_reference_checks_actual_sha1_and_size() {
        let file = snapshot("mods/custom-name.jar", b"original bytes");
        let mut origin: crate::curseforge::SavedFile = serde_json::from_value(serde_json::json!({
            "project":{"id":1450249,"gameId":432,"name":"Test","slug":"test","summary":"", "downloadCount":0,"links":{"websiteUrl":"https://www.curseforge.com/minecraft/mc-mods/test"},"dateModified":"2026-01-01","dateCreated":"2026-01-01"},
            "file":{"id":8097991,"modId":1450249,"fileName":"original.jar","displayName":"1.0","fileLength":file.size,"fileDate":"2026-01-01","downloadCount":0,"releaseType":1,"isAvailable":true,"hashes":[{"algo":1,"value":file.sha1}]}
        })).unwrap();
        assert!(curseforge_reference(&file, &origin).is_some());
        let edited = snapshot("mods/custom-name.jar", b"modified bytes");
        assert!(curseforge_reference(&edited, &origin).is_none());
        origin.file.file_length += 1;
        assert!(curseforge_reference(&file, &origin).is_none());
    }

    #[test]
    fn archive_embeds_snapshots_and_keeps_reference_only_in_manifest() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("pack.mrpack");
        let embedded = snapshot("mods/edited.jar", b"edited content");
        let snapshot_path = embedded.snapshot.to_path_buf();
        let config = snapshot("config/test.cfg", b"custom config");
        let online = snapshot("mods/online.jar", b"online content");
        let reference = modrinth_reference(&online, &version(&online)).unwrap();
        let index =
            serde_json::to_vec(&serde_json::json!({"files":[reference]}))
                .unwrap();
        write_export_archive(
            &destination,
            vec![embedded, config],
            &index,
            Some(b"test artwork"),
            |_| Ok(()),
        )
        .unwrap();
        let mut archive =
            zip::ZipArchive::new(std::fs::File::open(&destination).unwrap())
                .unwrap();
        assert_eq!(archive.len(), 4);
        assert_eq!(archive.by_name("icon.png").unwrap().size(), 12);
        let mut actual = Vec::new();
        archive
            .by_name("overrides/mods/edited.jar")
            .unwrap()
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(actual, b"edited content");
        assert!(archive.by_name("overrides/mods/online.jar").is_err());
        let mut actual_index = Vec::new();
        archive
            .by_name("modrinth.index.json")
            .unwrap()
            .read_to_end(&mut actual_index)
            .unwrap();
        assert_eq!(actual_index, index);
        assert!(!snapshot_path.exists());
    }

    #[test]
    fn failed_export_preserves_existing_destination_and_removes_temporaries() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("pack.mrpack");
        std::fs::write(&destination, b"previous export").unwrap();
        let file = snapshot("mods/custom.jar", b"custom mod");
        let snapshot_path = file.snapshot.to_path_buf();
        assert!(
            write_export_archive(&destination, vec![file], b"{}", None, |_| {
                bail!("controlled write failure")
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous export");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        assert!(!snapshot_path.exists());
    }

    #[test]
    fn export_selection_retains_exclusions_and_disabled_file_rules() {
        let selection = ExportSelection::new(
            vec!["mods".into(), "config/test.cfg".into()],
            vec!["mods/private.jar".into()],
        );
        assert!(
            selection.is_included(
                &logical_content_path(
                    &"mods/test.jar.disabled".to_string().try_into().unwrap()
                )
                .unwrap()
            )
        );
        assert!(
            !selection.is_included(
                &"mods/private.jar".to_string().try_into().unwrap()
            )
        );
        assert!(
            selection.should_visit_directory(
                &"config".to_string().try_into().unwrap()
            )
        );
        assert!(!is_path_exportable(
            &"profile.json".to_string().try_into().unwrap()
        ));
    }
}
