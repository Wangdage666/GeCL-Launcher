use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};

struct Agent {
    root: PathBuf,
    started: AtomicBool,
    session: String,
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let mut temp = tempfile::NamedTempFile::new_in(
        path.parent().context("invalid agent directory")?,
    )?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[tauri::command]
async fn ready(app: tauri::AppHandle) -> crate::api::Result<()> {
    let agent = app.state::<Arc<Agent>>().inner().clone();
    if agent.started.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut heartbeat_at =
            std::time::Instant::now() - std::time::Duration::from_secs(2);
        loop {
            if handle.get_webview_window("main").is_none() {
                break;
            }
            if heartbeat_at.elapsed() >= std::time::Duration::from_secs(1) {
                let heartbeat = format!(
                    "{}\n{}\n{}",
                    std::process::id(),
                    agent.session,
                    chrono::Utc::now().timestamp()
                );
                if let Err(error) = atomic_write(
                    &agent.root.join("ready"),
                    heartbeat.as_bytes(),
                ) {
                    tracing::debug!(%error, "Agent heartbeat unavailable");
                }
                heartbeat_at = std::time::Instant::now();
            }
            if let Ok(entries) = std::fs::read_dir(&agent.root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Some(filename) =
                        path.file_name().and_then(|name| name.to_str())
                    else {
                        continue;
                    };
                    let Some(id) = filename.strip_suffix(".request.json")
                    else {
                        continue;
                    };
                    if uuid::Uuid::parse_str(id).is_err() {
                        continue;
                    }
                    let id = id.to_owned();
                    let request = (|| -> anyhow::Result<Value> {
                        let metadata = std::fs::symlink_metadata(&path)?;
                        ensure!(
                            metadata.is_file()
                                && !metadata.file_type().is_symlink()
                                && metadata.len() <= 1024 * 1024,
                            "invalid agent request"
                        );
                        let request: Value =
                            serde_json::from_slice(&std::fs::read(&path)?)?;
                        ensure!(
                            request.get("session").and_then(Value::as_str)
                                == Some(&agent.session),
                            "调试会话已变化，请重新执行命令"
                        );
                        Ok(request)
                    })();
                    if let Err(error) = std::fs::remove_file(&path) {
                        tracing::debug!(%error, "Unable to consume agent request");
                        continue;
                    }
                    match request {
                        Ok(mut request) => {
                            request["id"] = json!(id);
                            if let Err(error) = handle.emit_to(
                                "main",
                                "gecl-agent-request",
                                request,
                            ) {
                                let _ = complete_inner(
                                    &agent,
                                    &id,
                                    json!({"ok":false,"error":error.to_string()}),
                                );
                            }
                        }
                        Err(error) => {
                            let _ = complete_inner(
                                &agent,
                                &id,
                                json!({"ok":false,"error":error.to_string()}),
                            );
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
    Ok(())
}

fn complete_inner(
    agent: &Agent,
    id: &str,
    result: Value,
) -> anyhow::Result<()> {
    uuid::Uuid::parse_str(id)?;
    atomic_write(
        &agent.root.join(format!("{id}.response.json")),
        &serde_json::to_vec(&result)?,
    )
}

#[tauri::command]
fn complete(
    app: tauri::AppHandle,
    id: String,
    result: Value,
) -> crate::api::Result<()> {
    Ok(complete_inner(
        app.state::<Arc<Agent>>().inner(),
        &id,
        result,
    )?)
}

#[tauri::command]
fn commands() -> Value {
    serde_json::from_str(include_str!("../../../docs/AGENT_COMMANDS.json"))
        .expect("valid compiled command catalog")
}

pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("agent")
        .setup(|app, _| {
            let root = std::env::var_os("THESEUS_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(
                        std::env::var_os("APPDATA").unwrap_or_default(),
                    )
                    .join(app.config().identifier.clone())
                })
                .join("agent");
            std::fs::create_dir_all(&root)?;
            if std::fs::symlink_metadata(&root)?.file_type().is_symlink() {
                return Err(std::io::Error::other(
                    "agent directory cannot be a link",
                )
                .into());
            }
            app.manage(Arc::new(Agent {
                root,
                started: AtomicBool::new(false),
                session: uuid::Uuid::new_v4().to_string(),
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ready, complete, commands])
        .build()
}
