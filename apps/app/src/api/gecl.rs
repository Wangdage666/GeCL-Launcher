use crate::api::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};

static IMPORTS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("gecl")
        .invoke_handler(tauri::generate_handler![
            setup_status,
            choose_directory,
            finish_setup,
            preview_folder,
            import_folder,
            cancel_import,
            launch_progress,
            dismiss_launch,
            pack_artwork,
            cache_artwork,
            world_backup_directory,
            world_backup,
            cancel_world_backup,
            appearance_get,
            appearance_set,
            appearance_import_background,
            appearance_discard_background,
            curseforge_search,
            curseforge_installed,
            curseforge_project,
            curseforge_files,
            curseforge_file,
            curseforge_changelog,
            curseforge_description,
            curseforge_install,
            curseforge_cancel,
            update_check,
            update_download,
            update_cancel,
            update_install
        ])
        .build()
}

#[tauri::command]
async fn pack_artwork<R: tauri::Runtime>(app: tauri::AppHandle<R>, source: PathBuf) -> Result<theseus::gecl::pack_artwork::PackArtwork> {
    use tauri::Manager;
    let value = theseus::gecl::pack_artwork::inspect(source).await?;
    if let Some(icon) = &value.icon { app.asset_protocol_scope().allow_file(icon)?; }
    Ok(value)
}

#[tauri::command]
async fn cache_artwork<R: tauri::Runtime>(app: tauri::AppHandle<R>, source: PathBuf) -> Result<PathBuf> {
    use tauri::Manager;
    let path = theseus::gecl::pack_artwork::cache(source).await?;
    app.asset_protocol_scope().allow_file(&path)?;
    Ok(path)
}

static WORLD_BACKUPS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
async fn world_backup_directory() -> Result<Option<String>> { Ok(theseus::gecl::world_backups::directory().await?) }

#[tauri::command]
async fn world_backup(job_id: String, instance_id: String, world: String, archive: PathBuf, restore: bool,
    on_progress: tauri::ipc::Channel<theseus::gecl::world_backups::Progress>) -> Result<theseus::gecl::world_backups::BackupResult> {
    if job_id.len() > 64 { return Err(anyhow::anyhow!("无效任务编号").into()); }
    let cancelled = Arc::new(AtomicBool::new(false));
    { let mut jobs = WORLD_BACKUPS.lock().map_err(|_| anyhow::anyhow!("无法创建备份任务"))?;
      if jobs.contains_key(&job_id) { return Err(anyhow::anyhow!("任务已经存在").into()); }
      jobs.insert(job_id.clone(), cancelled.clone()); }
    let result = theseus::gecl::world_backups::run(instance_id, world, archive, restore, cancelled,
        move |progress| { let _ = on_progress.send(progress); }).await;
    if let Ok(mut jobs) = WORLD_BACKUPS.lock() { jobs.remove(&job_id); }
    Ok(result?)
}

#[tauri::command]
async fn cancel_world_backup(job_id: String) { if let Ok(jobs) = WORLD_BACKUPS.lock() { if let Some(cancelled) = jobs.get(&job_id) { cancelled.store(true, Ordering::Relaxed); } } }

fn allow_background<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    background: Option<&theseus::gecl::appearance::Background>,
) -> Result<()> {
    use tauri::Manager;
    if let Some(background) = background {
        app.asset_protocol_scope().allow_file(&background.path)?;
    }
    Ok(())
}

#[tauri::command]
async fn appearance_get<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<theseus::gecl::appearance::Appearance> {
    let value = theseus::gecl::appearance::get().await?;
    allow_background(&app, value.background.as_ref())?;
    Ok(value)
}

#[tauri::command]
async fn appearance_set<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    preferences: theseus::gecl::appearance::Preferences,
    window: Option<theseus::gecl::appearance::WindowAppearance>,
) -> Result<theseus::gecl::appearance::Appearance> {
    let value = theseus::gecl::appearance::set(preferences, window).await?;
    allow_background(&app, value.background.as_ref())?;
    Ok(value)
}

#[tauri::command]
async fn appearance_import_background<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    source: PathBuf,
) -> Result<theseus::gecl::appearance::Background> {
    let value = theseus::gecl::appearance::import_background(source).await?;
    allow_background(&app, Some(&value))?;
    Ok(value)
}

#[tauri::command]
async fn appearance_discard_background(id: String) -> Result<()> {
    Ok(theseus::gecl::appearance::discard_background(id).await?)
}

#[tauri::command]
async fn launch_progress() -> Vec<theseus::launch_progress::LaunchProgress> {
    theseus::launch_progress::list()
}

#[tauri::command]
async fn dismiss_launch(id: uuid::Uuid) {
    theseus::launch_progress::dismiss(id);
}

#[tauri::command]
async fn setup_status<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<theseus::gecl::SetupStatus> {
    Ok(theseus::gecl::setup_status(&app.config().identifier).await?)
}

#[tauri::command]
async fn choose_directory<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: PathBuf,
) -> Result<()> {
    Ok(theseus::gecl::choose_directory(&app.config().identifier, path).await?)
}

#[tauri::command]
async fn finish_setup() -> Result<()> {
    Ok(theseus::gecl::finish_setup().await?)
}

#[tauri::command]
async fn preview_folder(
    path: PathBuf,
) -> Result<Vec<theseus::gecl::LocalGame>> {
    Ok(theseus::gecl::preview_folder(path).await?)
}

#[tauri::command]
async fn import_folder(
    job_id: String,
    source: PathBuf,
    version_file: PathBuf,
    on_progress: tauri::ipc::Channel<(u64, u64)>,
) -> Result<String> {
    if job_id.len() > 64 {
        return Err(anyhow::anyhow!("无效任务编号").into());
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = IMPORTS
            .lock()
            .map_err(|_| anyhow::anyhow!("无法创建导入任务"))?;
        if jobs.contains_key(&job_id) {
            return Err(anyhow::anyhow!("任务已经存在").into());
        }
        jobs.insert(job_id.clone(), cancelled.clone());
    }
    let last = Mutex::new(
        std::time::Instant::now() - std::time::Duration::from_secs(1),
    );
    let result = theseus::gecl::import_folder(
        source,
        version_file,
        &|| cancelled.load(Ordering::Relaxed),
        &|done, total| {
            if let Ok(mut at) = last.lock() {
                if done == 0
                    || done >= total
                    || at.elapsed() >= std::time::Duration::from_millis(150)
                {
                    let _ = on_progress.send((done, total));
                    *at = std::time::Instant::now();
                }
            }
        },
    )
    .await;
    if let Ok(mut jobs) = IMPORTS.lock() {
        jobs.remove(&job_id);
    }
    let id = result?;
    theseus::gecl::finalize_import(&id).await?;
    Ok(id)
}

#[tauri::command]
async fn cancel_import(job_id: String) -> Result<()> {
    if let Ok(jobs) = IMPORTS.lock() {
        if let Some(cancel) = jobs.get(&job_id) {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    Ok(())
}

#[tauri::command]
async fn curseforge_search(
    request: theseus::curseforge::Search,
) -> Result<theseus::curseforge::Page<theseus::curseforge::Project>> {
    Ok(theseus::curseforge::search(request).await?)
}
#[tauri::command]
async fn curseforge_project(id: u64) -> Result<theseus::curseforge::Project> {
    Ok(theseus::curseforge::project(id).await?)
}
#[tauri::command]
async fn curseforge_files(
    id: u64,
    request: theseus::curseforge::Search,
) -> Result<theseus::curseforge::Page<theseus::curseforge::File>> {
    Ok(theseus::curseforge::files(id, request).await?)
}
#[tauri::command]
async fn curseforge_description(id: u64) -> Result<String> {
    Ok(theseus::curseforge::description(id).await?)
}
#[tauri::command]
async fn curseforge_file(
    id: u64,
    file_id: u64,
) -> Result<theseus::curseforge::File> {
    Ok(theseus::curseforge::file(id, file_id).await?)
}
#[tauri::command]
async fn curseforge_changelog(id: u64, file_id: u64) -> Result<String> {
    Ok(theseus::curseforge::changelog(id, file_id).await?)
}
#[tauri::command]
async fn curseforge_install(
    job_id: uuid::Uuid,
    instance_id: String,
    project_id: u64,
    file_id: Option<u64>,
    on_progress: tauri::ipc::Channel<theseus::curseforge::InstallProgress>,
) -> Result<Vec<String>> {
    let last = Mutex::new(
        std::time::Instant::now() - std::time::Duration::from_secs(1),
    );
    Ok(theseus::curseforge::install(
        job_id,
        &instance_id,
        project_id,
        file_id,
        &|event| {
            if let Ok(mut at) = last.lock()
                && (event.completed == 0
                    || event.completed >= event.total
                    || at.elapsed() >= std::time::Duration::from_millis(150))
            {
                let _ = on_progress.send(event);
                *at = std::time::Instant::now();
            }
        },
    )
    .await?)
}
#[tauri::command]
async fn curseforge_cancel(job_id: uuid::Uuid) {
    theseus::curseforge::cancel(job_id);
}

#[tauri::command]
async fn curseforge_installed()
-> Result<std::collections::HashMap<String, Vec<u64>>> {
    Ok(theseus::curseforge::installed_projects().await?)
}

use theseus::gecl::app_update;

#[derive(Default)]
struct UpdateState {
    latest: Option<app_update::UpdateInfo>,
    downloaded: Option<(PathBuf, String)>,
    cancelled: Option<Arc<AtomicBool>>,
}

static UPDATE: LazyLock<Mutex<UpdateState>> = LazyLock::new(Mutex::default);

fn update_state() -> Result<std::sync::MutexGuard<'static, UpdateState>> {
    Ok(UPDATE.lock().map_err(|_| anyhow::anyhow!("更新状态不可用"))?)
}

#[tauri::command]
async fn update_check<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Option<app_update::UpdateInfo>> {
    let info = app_update::check(&app.package_info().version.to_string()).await?;
    let mut state = update_state()?;
    if state.latest != info { state.downloaded = None; }
    state.latest = info.clone();
    Ok(info)
}

#[tauri::command]
async fn update_download(on_progress: tauri::ipc::Channel<app_update::Progress>) -> Result<()> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let info = {
        let mut state = update_state()?;
        if state.cancelled.is_some() { return Err(anyhow::anyhow!("更新正在下载").into()); }
        let info = state.latest.clone().ok_or_else(|| anyhow::anyhow!("没有可用的更新"))?;
        state.cancelled = Some(cancelled.clone());
        info
    };
    let result = app_update::download(&info, &cancelled, |progress| { let _ = on_progress.send(progress); }).await;
    let mut state = update_state()?;
    state.cancelled = None;
    state.downloaded = Some((result?, info.sha256));
    Ok(())
}

#[tauri::command]
fn update_cancel() -> Result<()> {
    if let Some(cancelled) = &update_state()?.cancelled { cancelled.store(true, Ordering::Relaxed); }
    Ok(())
}

/// Runs the verified installer passively with `/UPDATE /R`; it relaunches gecl after replacing the files.
#[tauri::command]
async fn update_install<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<()> {
    let (path, sha256) = update_state()?.downloaded.clone().ok_or_else(|| anyhow::anyhow!("安装包尚未下载"))?;
    if app_update::file_sha256(&path).await? != sha256 {
        update_state()?.downloaded = None;
        return Err(anyhow::anyhow!("安装包校验失败，请重新下载").into());
    }
    launch_installer(&path)?;
    app.exit(0);
    Ok(())
}

#[cfg(windows)]
fn launch_installer(path: &std::path::Path) -> Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, w};
    let file = HSTRING::from(path.as_os_str());
    let result = unsafe { ShellExecuteW(None, w!("open"), &file, w!("/P /UPDATE /R"), None, SW_SHOWNORMAL) };
    if result.0 as isize <= 32 {
        return Err(anyhow::anyhow!("没有启动安装程序（可能已取消管理员授权）").into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn launch_installer(_path: &std::path::Path) -> Result<()> {
    Err(anyhow::anyhow!("当前系统不支持自动安装").into())
}
