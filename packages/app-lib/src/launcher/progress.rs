//! Launch observations only: never choose Java, alter arguments, or infer success from a timer.
use dashmap::DashMap;
use serde::Serialize;
use std::sync::LazyLock;
use uuid::Uuid;

static LAUNCHES: LazyLock<DashMap<String, LaunchProgress>> =
    LazyLock::new(DashMap::new);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchStage {
    Preparing,
    CheckingJava,
    CheckingFiles,
    StartingJava,
    LoadingMods,
    LoadingResources,
    WindowReady,
}

impl LaunchStage {
    fn percent(self) -> u8 {
        match self {
            Self::Preparing => 5,
            Self::CheckingJava => 15,
            Self::CheckingFiles => 25,
            Self::StartingJava => 35,
            Self::LoadingMods => 55,
            Self::LoadingResources => 80,
            Self::WindowReady => 100,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct LaunchProgress {
    pub id: Uuid,
    pub instance_id: String,
    pub name: String,
    pub stage: LaunchStage,
    pub percent: u8,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub failed: bool,
}

impl LaunchProgress {
    fn advance(&mut self, stage: LaunchStage) {
        if self.finished_at.is_none() && stage > self.stage {
            self.stage = stage;
            self.percent = stage.percent();
            if stage == LaunchStage::WindowReady {
                self.finished_at = Some(chrono::Utc::now().timestamp_millis());
            }
        }
    }

    fn fail(&mut self) {
        if self.finished_at.is_none() {
            self.failed = true;
            self.finished_at = Some(chrono::Utc::now().timestamp_millis());
        }
    }
}

/// A failed/canceled preparation must never leave a perpetual "starting" row.
pub(crate) struct LaunchAttempt {
    instance_id: String,
    id: Uuid,
    handed_off: bool,
}

impl LaunchAttempt {
    pub fn begin(instance_id: &str) -> crate::Result<Self> {
        use dashmap::mapref::entry::Entry;
        let id = Uuid::new_v4();
        let progress = LaunchProgress {
            id,
            instance_id: instance_id.to_owned(),
            name: instance_id.to_owned(),
            stage: LaunchStage::Preparing,
            percent: LaunchStage::Preparing.percent(),
            started_at: chrono::Utc::now().timestamp_millis(),
            finished_at: None,
            failed: false,
        };
        match LAUNCHES.entry(instance_id.to_owned()) {
            Entry::Occupied(mut entry) => {
                if entry.get().finished_at.is_none() {
                    return Err(crate::ErrorKind::LauncherError(
                        "该游戏正在启动".into(),
                    )
                    .into());
                }
                entry.insert(progress);
            }
            Entry::Vacant(entry) => {
                entry.insert(progress);
            }
        }
        Ok(Self {
            instance_id: instance_id.to_owned(),
            id,
            handed_off: false,
        })
    }

    pub fn hand_off(&mut self) {
        self.handed_off = true;
    }
}

impl Drop for LaunchAttempt {
    fn drop(&mut self) {
        if !self.handed_off {
            fail_attempt(&self.instance_id, self.id);
        }
    }
}

fn fail_attempt(instance_id: &str, id: Uuid) {
    if let Some(mut progress) = LAUNCHES.get_mut(instance_id) {
        if progress.id == id {
            progress.fail();
        }
    }
}

pub(crate) fn set_name(instance_id: &str, name: &str) {
    if let Some(mut progress) = LAUNCHES.get_mut(instance_id) {
        progress.name = name.to_owned();
    }
}

pub(crate) fn advance(instance_id: &str, stage: LaunchStage) {
    if let Some(mut progress) = LAUNCHES.get_mut(instance_id) {
        progress.advance(stage);
    }
}

/// Whitelisted milestones only. Raw mod log text (which may contain secrets) never enters the snapshot.
pub(crate) fn observe_log(instance_id: &str, message: &str) {
    if let Some(stage) = log_stage(message) {
        advance(instance_id, stage);
    }
}

fn log_stage(message: &str) -> Option<LaunchStage> {
    let mut message = message.trim_start();
    // Legacy consoles prefix the same milestones with time/thread/level fields.
    for _ in 0..4 {
        if message.starts_with('[') {
            if let Some((_, rest)) = message.split_once(']') {
                message = rest.trim_start_matches([' ', ':']);
                continue;
            }
        }
        break;
    }
    if message.starts_with("Reloading ResourceManager:")
        || message.starts_with("Reloading resource manager:")
        || message.starts_with("Created: ")
            && message.contains("textures/atlas/")
    {
        Some(LaunchStage::LoadingResources)
    } else if message.starts_with("Loading ") && message.contains(" mods:")
        || message.starts_with("ModLauncher running:")
        || message.starts_with("Loading Minecraft ")
        || message.starts_with("SpongePowered MIXIN Subsystem")
    {
        Some(LaunchStage::LoadingMods)
    } else {
        None
    }
}

pub fn list() -> Vec<LaunchProgress> {
    let now = chrono::Utc::now().timestamp_millis();
    LAUNCHES.retain(|_, p| {
        p.finished_at.is_none_or(|at| p.failed || now - at < 5000)
    });
    let mut launches: Vec<_> =
        LAUNCHES.iter().map(|p| p.value().clone()).collect();
    launches.sort_by_key(|p| p.started_at);
    launches
}

/// Dismissing hides a terminal report; it never terminates a game or interrupts installation.
pub fn dismiss(id: Uuid) {
    LAUNCHES.retain(|_, p| p.id != id || p.finished_at.is_none());
}

pub(crate) fn watch_process(
    instance_id: &str,
    process_id: Uuid,
    pid: Option<u32>,
    state: std::sync::Arc<crate::State>,
) {
    let Some(progress) = LAUNCHES.get(instance_id).map(|p| p.clone()) else {
        return;
    };
    let instance_id = instance_id.to_owned();
    tokio::spawn(async move {
        loop {
            let active = LAUNCHES.get(&instance_id).is_some_and(|p| {
                p.id == progress.id && p.finished_at.is_none()
            });
            if !active {
                break;
            }
            if state.process_manager.get(process_id).is_none() {
                fail_attempt(&instance_id, progress.id);
                break;
            }
            #[cfg(windows)]
            if let Some(pid) = pid {
                // A bounded native window enumeration: no process-wide scans and no title matching.
                if visible_window(pid) {
                    advance(&instance_id, LaunchStage::WindowReady);
                    #[cfg(feature = "tauri")]
                    if crate::state::Settings::get(&state.pool)
                        .await
                        .is_ok_and(|s| s.hide_on_process_start)
                    {
                        if let Ok(Some(window)) =
                            crate::EventState::get_main_window().await
                        {
                            let _ = window.minimize();
                        }
                    }
                    break;
                }
            }
            #[cfg(not(windows))]
            let _ = pid;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });
}

#[cfg(windows)]
fn visible_window(pid: u32) -> bool {
    use windows::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
        },
    };
    use windows::core::BOOL;
    struct Search {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> BOOL {
        let search = unsafe { &mut *(data.0 as *mut Search) };
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut owner));
        }
        if owner == search.pid && unsafe { IsWindowVisible(window) }.as_bool() {
            search.found = true;
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut search = Search { pid, found: false };
    unsafe {
        let _ = EnumWindows(
            Some(visit),
            LPARAM(&mut search as *mut Search as isize),
        );
    }
    search.found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestones_never_move_back_or_finish_on_a_log_line() {
        let instance = Uuid::new_v4().to_string();
        let mut attempt = LaunchAttempt::begin(&instance).unwrap();
        observe_log(&instance, "Loading 340 mods:");
        assert_eq!(
            list()
                .into_iter()
                .find(|p| p.instance_id == instance)
                .unwrap()
                .percent,
            55
        );
        observe_log(
            &instance,
            "Reloading ResourceManager: vanilla, mod resources",
        );
        advance(&instance, LaunchStage::CheckingJava);
        let p = LAUNCHES.get(&instance).unwrap();
        assert_eq!(p.percent, 80);
        assert!(p.finished_at.is_none());
        drop(p);
        attempt.hand_off();
        LAUNCHES.remove(&instance);
        assert!(
            log_stage("ERROR: Loading failed: 3 mods: incompatible").is_none()
        );
        assert!(log_stage("Started game, waiting for window").is_none());
        assert_eq!(
            log_stage("[12:00:00] [main/INFO]: Loading 340 mods:"),
            Some(LaunchStage::LoadingMods)
        );
    }

    #[test]
    fn preparation_failure_retry_and_old_attempt_are_separate() {
        let instance = Uuid::new_v4().to_string();
        let first = LaunchAttempt::begin(&instance).unwrap();
        assert!(LaunchAttempt::begin(&instance).is_err());
        let old = first.id;
        drop(first);
        assert!(LAUNCHES.get(&instance).unwrap().failed);
        let mut second = LaunchAttempt::begin(&instance).unwrap();
        fail_attempt(&instance, old);
        assert!(!LAUNCHES.get(&instance).unwrap().failed);
        dismiss(second.id);
        assert!(LAUNCHES.contains_key(&instance));
        advance(&instance, LaunchStage::WindowReady);
        advance(&instance, LaunchStage::LoadingMods);
        fail_attempt(&instance, second.id);
        assert_eq!(LAUNCHES.get(&instance).unwrap().percent, 100);
        assert!(!LAUNCHES.get(&instance).unwrap().failed);
        second.hand_off();
        dismiss(second.id);
        assert!(!LAUNCHES.contains_key(&instance));
    }

    #[cfg(windows)]
    #[test]
    fn unknown_process_has_no_visible_window() {
        assert!(!visible_window(u32::MAX));
    }

    #[cfg(windows)]
    #[test]
    fn native_window_is_detected_by_its_own_pid() {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP,
            WS_VISIBLE,
        };
        use windows::core::w;
        // Owned off-screen Win32 window; never interact with the user's desktop.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("GeCL isolated window probe"),
                WS_POPUP | WS_VISIBLE,
                -32000,
                -32000,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        };
        let detected = visible_window(std::process::id());
        unsafe {
            DestroyWindow(window).unwrap();
        }
        assert!(detected);
    }
}
