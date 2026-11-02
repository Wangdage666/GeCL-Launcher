use super::{File, Project, SavedFile, Search, compatible, download_urls};
use crate::{State, state::content_store::StoredFileHandle};
use anyhow::{Context, bail};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::{LazyLock, Mutex},
};
use tokio::sync::watch;
use uuid::Uuid;

static JOBS: LazyLock<Mutex<HashMap<Uuid, watch::Sender<bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
struct Registration(Uuid);
impl Drop for Registration {
    fn drop(&mut self) {
        JOBS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.0);
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: String,
    pub name: String,
    pub completed: u64,
    pub total: u64,
}
pub fn cancel(id: Uuid) {
    if let Some(sender) =
        JOBS.lock().unwrap_or_else(|e| e.into_inner()).get(&id)
    {
        sender.send_replace(true);
    }
}
pub(crate) fn checkpoint(
    cancelled: &watch::Receiver<bool>,
) -> crate::Result<()> {
    if *cancelled.borrow() {
        bail!("已取消安装");
    }
    Ok(())
}
async fn network<T>(
    control: &watch::Receiver<bool>,
    future: impl Future<Output = crate::Result<T>>,
) -> crate::Result<T> {
    checkpoint(control)?;
    let mut receiver = control.clone();
    tokio::select! { biased; _ = receiver.changed() => { bail!("已取消安装"); }, result=future => result }
}
pub(crate) struct PreparedMod {
    pub origin: SavedFile,
    pub stored: StoredFileHandle,
}
pub async fn install(
    id: Uuid,
    instance_id: &str,
    project_id: u64,
    file_id: Option<u64>,
    progress: &(dyn Fn(InstallProgress) + Send + Sync),
) -> crate::Result<Vec<String>> {
    let (sender, control) = watch::channel(false);
    {
        let mut jobs = JOBS.lock().unwrap_or_else(|e| e.into_inner());
        if jobs.contains_key(&id) {
            bail!("任务已存在");
        }
        jobs.insert(id, sender);
    }
    let _registration = Registration(id);
    let state = State::get().await?;
    let instance = crate::instance::get(instance_id)
        .await?
        .context("找不到游戏版本")?;
    crate::instance::ensure_instance_content_unlocked(
        &instance.instance.id,
        &state,
    )
    .await?;
    if instance
        .shared_instance
        .as_ref()
        .is_some_and(|s| s.role.is_member())
    {
        bail!("共享版本成员暂不能通过此入口安装模组");
    }
    let version = &instance.applied_content_set.game_version;
    let loader = instance.applied_content_set.loader.as_str();
    super::loader_type(loader)
        .context("请先选择使用 Forge、NeoForge、Fabric 或 Quilt 的游戏版本")?;
    let mut selected: HashMap<u64, (Project, File)> = HashMap::new();
    let root = match file_id {
        Some(file_id) => {
            Some(network(&control, super::file(project_id, file_id)).await?)
        }
        None => None,
    };
    let mut pending = vec![(project_id, root)];
    let saved = super::saved_files(&instance.instance.id, &state.pool).await?;
    let existing = crate::state::instances::commands::sync_content_files(
        &instance.instance.id,
        &state,
    )
    .await?;
    let mut excluded = HashSet::new();
    while let Some((project_id, pinned)) = pending.pop() {
        checkpoint(&control)?;
        if selected.contains_key(&project_id) {
            continue;
        }
        if selected.len() >= 128 {
            bail!("依赖数量超过安全上限，请检查整合包");
        }
        let project = network(&control, super::project(project_id)).await?;
        progress(InstallProgress {
            phase: "解析依赖".into(),
            name: project.name.clone(),
            completed: selected.len() as u64,
            total: 0,
        });
        let file = match pinned {
            Some(file) => file,
            None => {
                let retained = saved.values().find(|s| {
                    s.project.id == project_id
                        && compatible(&s.file, version, loader)
                        && existing.iter().any(|f| {
                            !f.missing
                                && f.enabled
                                && sha1(&s.file)
                                    .is_ok_and(|hash| f.sha1 == hash)
                        })
                });
                if let Some(retained) = retained {
                    retained.file.clone()
                } else {
                    let mut index = 0;
                    let mut found = None;
                    while index < 1000 {
                        let page = network(
                            &control,
                            super::files(
                                project_id,
                                Search {
                                    game_version: Some(version.clone()),
                                    loader: Some(loader.into()),
                                    index,
                                    page_size: Some(50),
                                    ..Default::default()
                                },
                            ),
                        )
                        .await?;
                        let count = page.data.len();
                        let total = page.pagination.total_count;
                        found = page
                            .data
                            .into_iter()
                            .filter(|f| compatible(f, version, loader))
                            .max_by(|a, b| a.file_date.cmp(&b.file_date));
                        if found.is_some()
                            || count == 0
                            || index + count as u32 >= total
                        {
                            break;
                        }
                        index += count as u32;
                    }
                    found.with_context(|| {
                        format!(
                            "{} 没有适用于 {} / {} 的文件",
                            project.name, version, loader
                        )
                    })?
                }
            }
        };
        if !compatible(&file, version, loader) {
            bail!("{} 与 {} / {} 不兼容", project.name, version, loader);
        }
        for dependency in &file.dependencies {
            match dependency.relation_type {
                3 => pending.push((dependency.mod_id, None)),
                5 => {
                    excluded.insert(dependency.mod_id);
                }
                _ => {}
            }
        }
        selected.insert(project_id, (project, file));
    }
    if selected.keys().any(|id| excluded.contains(id)) {
        bail!("所选文件包含互不兼容的模组依赖");
    }
    for existing in saved.values() {
        if excluded.contains(&existing.project.id)
            && existing_file_present(
                &existing.file,
                &instance.instance.id,
                &state,
            )
            .await?
        {
            bail!("已安装的 {} 与所选文件不兼容", existing.project.name);
        }
    }
    let mut prepared = Vec::new();
    let total = selected.len() as u64;
    for (project, file) in selected.into_values() {
        checkpoint(&control)?;
        let hash = sha1(&file)?;
        if existing.iter().any(|f| {
            !f.enabled && !f.missing && f.sha1.eq_ignore_ascii_case(hash)
        }) {
            bail!(
                "{} 已被禁用，请先在内容列表启用后再安装依赖",
                file.file_name
            );
        }
        let same = existing.iter().any(|f| {
            !f.missing && f.enabled && f.sha1.eq_ignore_ascii_case(hash)
        });
        if same {
            continue;
        }
        let urls = download_urls(&file)?;
        let refs = urls.iter().map(String::as_str).collect::<Vec<_>>();
        let done = prepared.len() as u64;
        let mut report = |downloaded, total_bytes| {
            progress(InstallProgress {
                phase: "下载模组".into(),
                name: file.file_name.clone(),
                completed: downloaded,
                total: total_bytes,
            });
            Box::pin(async { Ok(()) })
                as std::pin::Pin<
                    Box<dyn Future<Output = crate::Result<()>> + Send>,
                >
        };
        progress(InstallProgress {
            phase: "下载模组".into(),
            name: project.name.clone(),
            completed: done,
            total,
        });
        let downloaded = network(
            &control,
            crate::util::fetch::fetch_file_mirrors_in(
                &refs,
                Some(hash),
                None,
                None,
                &state.fetch_semaphore,
                &state.pool,
                Some(&mut report),
                Some(&state.content_store.staging),
            ),
        )
        .await?;
        if file.file_length > 0 && downloaded.size != file.file_length {
            bail!("{} 文件长度不符", file.file_name);
        }
        let stored = downloaded.store_file(&state).await?;
        prepared.push(PreparedMod {
            origin: SavedFile { project, file },
            stored,
        });
    }
    checkpoint(&control)?;
    progress(InstallProgress {
        phase: "安装模组".into(),
        name: instance.instance.name.clone(),
        completed: 0,
        total: prepared.len() as u64,
    });
    let paths = crate::state::instances::commands::install_curseforge_files(
        &instance.instance.id,
        prepared,
        &control,
        &state,
    )
    .await?;
    if let Err(error) = crate::event::emit::emit_instance(
        &instance.instance.id,
        crate::event::InstancePayloadType::Edited,
    )
    .await
    {
        tracing::warn!(%error,"CurseForge content installed, but UI notification failed");
    }
    Ok(paths)
}
pub(crate) fn sha1(file: &File) -> crate::Result<&str> {
    let hash = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.as_str())
        .context("缺少 SHA-1 校验值，不能安全安装此文件")?;
    if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("无效文件校验值");
    }
    Ok(hash)
}
async fn existing_file_present(
    file: &File,
    instance: &str,
    state: &State,
) -> crate::Result<bool> {
    let hash = sha1(file)?;
    Ok(
        crate::state::instances::get_instance_files(instance, &state.pool)
            .await?
            .iter()
            .any(|f| {
                f.enabled && !f.missing && f.sha1.eq_ignore_ascii_case(hash)
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancel_interrupts_pending_network_without_waiting_for_request_timeout()
     {
        let (sender, receiver) = watch::channel(false);
        let pending = async {
            std::future::pending::<()>().await;
            Ok(())
        };
        let cancel = async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            sender.send_replace(true);
        };
        let (result, ()) = tokio::join!(network(&receiver, pending), cancel);
        assert!(result.unwrap_err().to_string().contains("取消"));
    }
}
