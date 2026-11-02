/*!
# Theseus

Theseus is a library which provides utilities for launching minecraft, creating Modrinth mod packs,
and launching Modrinth mod packs
*/
#![warn(unused_import_braces)]
#![deny(unused_must_use)]

#[macro_use]
mod util;

mod api;
pub mod curseforge;
mod error;
mod event;
pub mod install;
mod launcher;
mod logger;
mod state;

pub use api::*;
pub use error::*;
#[cfg(feature = "export-ts")]
pub use event::export_app_event_bindings;
pub use event::{
    AppEvent, EventState, LoadingBar, LoadingBarType, emit::emit_loading,
    emit::emit_warning, emit::init_loading,
};
pub use logger::start_logger;
pub use launcher::progress as launch_progress;
pub use state::State;
pub use state::gecl;
pub use util::download_sources;
pub use util::fetch::DownloadReason;

pub fn launcher_user_agent() -> String {
    const LAUNCHER_BASE_USER_AGENT: &str =
        concat!("gecl/", env!("CARGO_PKG_VERSION"),);

    format!("{} ({})", LAUNCHER_BASE_USER_AGENT, std::env::consts::OS)
}
