use super::content_mutation::{ContentChange, InstallContent, InstanceContent};
use crate::curseforge::{PreparedMod, checkpoint, sha1};
use crate::state::content_store::{FileContent, StoredFileHandle};
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::{ContentSourceKind, InstanceFile, ProjectType, State};
use anyhow::{Context, bail};
use std::collections::HashSet;
use tokio::sync::watch;

struct Applied {
    path: String,
    previous: Option<(InstanceFile, StoredFileHandle)>,
}
pub(crate) async fn install_curseforge_files(
    instance_id: &str,
    files: Vec<PreparedMod>,
    cancel: &watch::Receiver<bool>,
    state: &State,
) -> crate::Result<Vec<String>> {
    let content = InstanceContent::lock(instance_id, state).await?;
    crate::instance::ensure_instance_content_unlocked(instance_id, state)
        .await?;
    if state
        .process_manager
        .get_all()
        .iter()
        .any(|p| p.instance_id == instance_id)
    {
        bail!("请先关闭此游戏再安装模组");
    }
    let existing =
        content_rows::get_instance_files(instance_id, &state.pool).await?;
    let origins =
        crate::curseforge::saved_files(instance_id, &state.pool).await?;
    let mut names = HashSet::new();
    let mut prepared = Vec::new();
    for file in files {
        let target = format!("mods/{}", file.origin.file.file_name);
        if !names.insert(target.to_lowercase()) {
            bail!("多个依赖使用相同文件名，请核对项目");
        }
        let previous = existing
            .iter()
            .find(|f| {
                origins.values().any(|o| {
                    o.project.id == file.origin.project.id
                        && sha1(&o.file)
                            .is_ok_and(|hash| f.sha1.eq_ignore_ascii_case(hash))
                })
            })
            .cloned();
        if let Some(other) = existing
            .iter()
            .find(|f| f.relative_path.eq_ignore_ascii_case(&target))
            && previous.as_ref().is_none_or(|p| p.id != other.id)
        {
            bail!("文件 {} 已存在，请先核对现有模组", target);
        }
        let physical = state
            .content_store
            .instance_path(&content.instance().path, &target)
            .await?;
        if previous
            .as_ref()
            .is_none_or(|p| !p.relative_path.eq_ignore_ascii_case(&target))
            && tokio::fs::try_exists(&physical).await?
        {
            bail!("文件 {} 已存在，请刷新内容列表后重试", target);
        }
        let previous = if let Some(previous) = previous {
            if !previous.enabled {
                bail!(
                    "{} 已被禁用，请先启用后再安装或切换版本",
                    previous.file_name
                );
            }
            if content_rows::is_instance_file_locked(
                instance_id,
                &previous.relative_path,
                &state.pool,
            )
            .await?
            {
                bail!("请先解冻 {} 再切换版本", previous.file_name);
            }
            let stored =
                match state.content_store.file_content(&previous).await? {
                    FileContent::Stored { stored_file, .. } => stored_file,
                    _ => {
                        bail!(
                            "现有 {} 尚未进入内容仓库，请刷新并修复后重试",
                            previous.file_name
                        );
                    }
                };
            Some((previous, stored))
        } else {
            None
        };
        prepared.push((target, file, previous));
    }
    let mut applied = Vec::<Applied>::new();
    let result: crate::Result<()> = async {
        for (target, file, previous) in &prepared {
            checkpoint(cancel)?;
            content
                .apply_change(ContentChange::Install(InstallContent {
                    requested_path: target,
                    stored_file: &file.stored,
                    project_type: ProjectType::Mod,
                    source_kind: ContentSourceKind::Local,
                    origin: None,
                    enabled_override: Some(true),
                    previous_path: previous
                        .as_ref()
                        .map(|(f, _)| f.relative_path.as_str()),
                }))
                .await?;
            applied.push(Applied {
                path: target.clone(),
                previous: previous.clone(),
            });
        }
        checkpoint(cancel)?;
        let mut tx = state.pool.begin().await?;
        for (_, file, previous) in &prepared {
            if let Some((_, old)) = previous {
                sqlx::query("DELETE FROM app_metadata WHERE key=?")
                    .bind(crate::curseforge::metadata_key(
                        instance_id,
                        &old.metadata.sha512,
                    ))
                    .execute(&mut *tx)
                    .await?;
            }
            file.origin
                .save(instance_id, &file.stored.metadata.sha512, &mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        for change in applied.iter().rev() {
            let restored = match &change.previous {
                Some((old, stored)) => {
                    content
                        .apply_change(ContentChange::Install(InstallContent {
                            requested_path: &old.relative_path,
                            stored_file: stored,
                            project_type: ProjectType::Mod,
                            source_kind: ContentSourceKind::Local,
                            origin: None,
                            enabled_override: Some(old.enabled),
                            previous_path: Some(&change.path),
                        }))
                        .await
                }
                None => {
                    content
                        .apply_change(ContentChange::Remove {
                            project_path: &change.path,
                        })
                        .await
                }
            };
            restored.with_context(|| {
                format!(
                    "安装失败：{error}；还原 {} 失败，请保留日志和文件",
                    change.path
                )
            })?;
        }
        return Err(error);
    }
    Ok(applied.into_iter().map(|change| change.path).collect())
}
