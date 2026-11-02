use crate::api::Result;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use theseus::prelude::*;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("auth")
        .invoke_handler(tauri::generate_handler![
            check_reachable,
            begin_device_login,
            poll_device_login,
            cancel_device_login,
            add_offline_user,
            remove_user,
            get_default_user,
            set_default_user,
            get_users,
        ])
        .build()
}

#[tauri::command]
pub async fn begin_device_login() -> Result<minecraft_auth::GeclDeviceLogin> {
    Ok(minecraft_auth::begin_gecl_device_login().await?)
}

#[tauri::command]
pub async fn poll_device_login(
    id: uuid::Uuid,
) -> Result<minecraft_auth::GeclDevicePoll> {
    Ok(minecraft_auth::poll_device_login(id).await?)
}

#[tauri::command]
pub async fn cancel_device_login(id: uuid::Uuid) -> Result<()> {
    minecraft_auth::cancel_gecl_device_login(id).await;
    Ok(())
}

#[tauri::command]
pub async fn add_offline_user(username: String) -> Result<Credentials> {
    Ok(minecraft_auth::add_offline_user(username).await?)
}

/// Checks if the authentication servers are reachable.
#[tauri::command]
pub async fn check_reachable() -> Result<()> {
    minecraft_auth::check_reachable().await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::remove_user(user).await?)
}

#[tauri::command]
pub async fn get_default_user() -> Result<Option<uuid::Uuid>> {
    Ok(minecraft_auth::get_default_user().await?)
}

#[tauri::command]
pub async fn set_default_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::set_default_user(user).await?)
}

/// Get a copy of the list of all user credentials
#[tauri::command]
pub async fn get_users() -> Result<Vec<Credentials>> {
    Ok(minecraft_auth::users().await?)
}
