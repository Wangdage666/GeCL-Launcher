//! Compatibility reads for public-resource callers; no platform authentication bridge.
use crate::api::Result;
use tauri::plugin::TauriPlugin;
use theseus::prelude::ModrinthCredentials;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("mr-auth")
        .invoke_handler(tauri::generate_handler![get, get_all])
        .build()
}

#[tauri::command]
pub async fn get() -> Result<Option<ModrinthCredentials>> {
    Ok(None)
}

#[tauri::command]
pub async fn get_all() -> Result<Vec<ModrinthCredentials>> {
    Ok(Vec::new())
}
