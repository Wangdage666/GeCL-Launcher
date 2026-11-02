//! Public Minecraft download routing. Authentication and arbitrary hosts never enter this path.
use anyhow::bail;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use url::Url;

const SETTINGS_KEY: &str = "gecl_download_sources";
const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    OfficialOnly,
    #[default]
    OfficialPreferred,
    MirrorPreferred,
    MirrorOnly,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadSourceSettings {
    pub policy: SourcePolicy,
    pub slow_kib_per_second: u64,
    pub observation_seconds: u64,
}
impl Default for DownloadSourceSettings {
    fn default() -> Self {
        Self {
            policy: SourcePolicy::OfficialPreferred,
            slow_kib_per_second: 128,
            observation_seconds: 8,
        }
    }
}
static CURRENT: LazyLock<RwLock<DownloadSourceSettings>> =
    LazyLock::new(|| RwLock::new(DownloadSourceSettings::default()));
static UNHEALTHY: LazyLock<RwLock<HashMap<String, Instant>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
impl DownloadSourceSettings {
    pub fn validate(&self) -> crate::Result<()> {
        if !(16..=8192).contains(&self.slow_kib_per_second)
            || !(3..=60).contains(&self.observation_seconds)
        {
            bail!("慢速阈值须为 16–8192 KiB/s，观察时间须为 3–60 秒");
        }
        Ok(())
    }
    pub async fn load(pool: &SqlitePool) -> crate::Result<Self> {
        let json: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
                .bind(SETTINGS_KEY)
                .fetch_optional(pool)
                .await?;
        let settings: Self = match json {
            Some(json) => serde_json::from_str(&json)?,
            None => Self::default(),
        };
        settings.validate()?;
        Ok(settings)
    }
    pub async fn save(&self, pool: &SqlitePool) -> crate::Result<()> {
        self.validate()?;
        sqlx::query("INSERT INTO app_metadata (key, value, updated_at) VALUES (?, ?, unixepoch()) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at")
            .bind(SETTINGS_KEY).bind(serde_json::to_string(self)?).execute(pool).await?;
        apply(self.clone());
        Ok(())
    }
}
pub(crate) fn apply(settings: DownloadSourceSettings) {
    *CURRENT.write() = settings;
    UNHEALTHY.write().clear();
}

pub(crate) fn mirror_url(original: &str) -> Option<String> {
    let url = Url::parse(original).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let path = url.path();
    let mirrored = match url.host_str()? {
        "launchermeta.mojang.com"
        | "launcher.mojang.com"
        | "piston-meta.mojang.com"
        | "piston-data.mojang.com" => path.to_owned(),
        "resources.download.minecraft.net" => format!("/assets{path}"),
        "libraries.minecraft.net" | "maven.fabricmc.net" => {
            format!("/maven{path}")
        }
        "maven.minecraftforge.net" => format!("/maven{path}"),
        "files.minecraftforge.net" if path.starts_with("/maven/") => {
            path.to_owned()
        }
        "meta.fabricmc.net" => format!("/fabric-meta{path}"),
        "maven.neoforged.net"
            if path.starts_with("/releases/net/neoforged/") =>
        {
            format!("/maven{}", path.strip_prefix("/releases")?)
        }
        _ => return None,
    };
    Some(format!("{BMCLAPI}{mirrored}"))
}

#[derive(Debug, thiserror::Error)]
#[error("下载源持续低速，切换备用源")]
pub(crate) struct SlowSource;

#[derive(Clone, Copy)]
pub(crate) struct SlowMonitor {
    pub window: Duration,
    pub minimum_bytes_per_second: u64,
}
impl SlowMonitor {
    pub fn check(
        &self,
        bytes: u64,
        total: u64,
        elapsed: Duration,
    ) -> crate::Result<()> {
        if total > 0 && bytes >= total {
            return Ok(());
        }
        if elapsed >= self.window
            && (bytes as f64 / elapsed.as_secs_f64())
                < self.minimum_bytes_per_second as f64
        {
            bail!(SlowSource);
        }
        Ok(())
    }
}

pub(crate) struct SourcePlan {
    pub urls: Vec<String>,
    pub monitor: Option<SlowMonitor>,
    pub(super) preferred_host: Option<String>,
}
impl SourcePlan {
    pub fn for_request(
        method: &reqwest::Method,
        url: &str,
        has_credentials_or_body: bool,
    ) -> Self {
        let settings = CURRENT.read().clone();
        Self::from_settings(method, url, has_credentials_or_body, &settings)
    }
    fn from_settings(
        method: &reqwest::Method,
        url: &str,
        private: bool,
        settings: &DownloadSourceSettings,
    ) -> Self {
        let mut plan = Self {
            urls: vec![url.to_owned()],
            monitor: None,
            preferred_host: None,
        };
        if *method != reqwest::Method::GET
            || private
            || settings.policy == SourcePolicy::OfficialOnly
        {
            return plan;
        }
        let Some(mirror) = mirror_url(url) else {
            return plan;
        };
        plan.urls = match settings.policy {
            SourcePolicy::OfficialPreferred => vec![url.to_owned(), mirror],
            SourcePolicy::MirrorPreferred => vec![mirror, url.to_owned()],
            SourcePolicy::MirrorOnly => vec![mirror],
            SourcePolicy::OfficialOnly => unreachable!(),
        };
        if plan.urls.len() > 1 {
            let preferred_host = Url::parse(&plan.urls[0])
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned));
            let unhealthy = preferred_host.as_ref().is_some_and(|host| {
                UNHEALTHY
                    .read()
                    .get(host)
                    .is_some_and(|until| *until > Instant::now())
            });
            if unhealthy {
                plan.urls.swap(0, 1);
            } else {
                plan.preferred_host = preferred_host;
                plan.monitor = Some(SlowMonitor {
                    window: Duration::from_secs(settings.observation_seconds),
                    minimum_bytes_per_second: settings.slow_kib_per_second
                        * 1024,
                });
            }
        }
        plan
    }
    pub fn mark_slow(&self) {
        if let Some(host) = &self.preferred_host {
            UNHEALTHY.write().insert(
                host.clone(),
                Instant::now() + Duration::from_secs(120),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gecl_mirror_maps_only_documented_public_hosts() {
        for (url, expected) in [
            (
                "https://piston-data.mojang.com/v1/objects/abc/client.jar",
                "/v1/objects/abc/client.jar",
            ),
            (
                "https://resources.download.minecraft.net/ab/abcdef",
                "/assets/ab/abcdef",
            ),
            (
                "https://libraries.minecraft.net/org/example/file.jar",
                "/maven/org/example/file.jar",
            ),
            (
                "https://meta.fabricmc.net/v2/versions/loader",
                "/fabric-meta/v2/versions/loader",
            ),
            (
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/a.jar",
                "/maven/net/neoforged/neoforge/a.jar",
            ),
        ] {
            assert_eq!(
                mirror_url(url).unwrap(),
                format!("{BMCLAPI}{expected}")
            );
        }
        for url in [
            "https://login.microsoftonline.com/token",
            "https://api.minecraftservices.com/entitlements/mcstore",
            "https://cdn.modrinth.com/data/a.jar",
            "https://libraries.minecraft.net.evil.test/file",
            "https://user:secret@libraries.minecraft.net/file",
            "https://libraries.minecraft.net/file?token=secret",
            "https://libraries.minecraft.net:8443/file",
        ] {
            assert!(mirror_url(url).is_none(), "{url}");
        }
    }
    #[test]
    fn gecl_download_policy_respects_priority_and_private_requests() {
        let url = "https://libraries.minecraft.net/a.jar";
        for (policy, count, first_is_mirror) in [
            (SourcePolicy::OfficialOnly, 1, false),
            (SourcePolicy::OfficialPreferred, 2, false),
            (SourcePolicy::MirrorPreferred, 2, true),
            (SourcePolicy::MirrorOnly, 1, true),
        ] {
            let settings = DownloadSourceSettings {
                policy,
                ..Default::default()
            };
            let plan = SourcePlan::from_settings(
                &reqwest::Method::GET,
                url,
                false,
                &settings,
            );
            assert_eq!(plan.urls.len(), count);
            assert_eq!(plan.urls[0].starts_with(BMCLAPI), first_is_mirror);
            assert_eq!(
                SourcePlan::from_settings(
                    &reqwest::Method::GET,
                    url,
                    true,
                    &settings
                )
                .urls,
                vec![url]
            );
            assert_eq!(
                SourcePlan::from_settings(
                    &reqwest::Method::POST,
                    url,
                    false,
                    &settings
                )
                .urls,
                vec![url]
            );
        }
    }
    #[test]
    fn gecl_slow_detection_observes_grace_and_complete_files() {
        let monitor = SlowMonitor {
            window: Duration::from_secs(8),
            minimum_bytes_per_second: 128 * 1024,
        };
        assert!(monitor.check(1024, 0, Duration::from_secs(7)).is_ok());
        assert!(monitor.check(1024, 0, Duration::from_secs(8)).is_err());
        assert!(monitor.check(1024, 1024, Duration::from_secs(8)).is_ok());
        assert!(
            monitor
                .check(2 * 1024 * 1024, 0, Duration::from_secs(8))
                .is_ok()
        );
    }
    #[tokio::test]
    async fn gecl_download_settings_validate_and_persist() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql("CREATE TABLE app_metadata (key TEXT PRIMARY KEY,value TEXT,updated_at INTEGER)").execute(&pool).await.unwrap();
        let settings = DownloadSourceSettings {
            policy: SourcePolicy::MirrorPreferred,
            observation_seconds: 12,
            slow_kib_per_second: 256,
        };
        settings.save(&pool).await.unwrap();
        assert_eq!(
            DownloadSourceSettings::load(&pool).await.unwrap(),
            settings
        );
        let invalid = DownloadSourceSettings {
            observation_seconds: 0,
            ..settings.clone()
        };
        assert!(invalid.save(&pool).await.is_err());
        assert_eq!(
            DownloadSourceSettings::load(&pool).await.unwrap(),
            settings
        );
        apply(DownloadSourceSettings::default());
    }
}

pub async fn get_settings() -> crate::Result<DownloadSourceSettings> {
    let state = crate::State::get().await?;
    DownloadSourceSettings::load(&state.pool).await
}
pub async fn set_settings(
    settings: DownloadSourceSettings,
) -> crate::Result<()> {
    let state = crate::State::get().await?;
    settings.save(&state.pool).await
}
