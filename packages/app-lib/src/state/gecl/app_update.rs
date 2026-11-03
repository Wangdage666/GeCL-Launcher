//! Finds newer GeCL releases on GitHub and downloads their Windows installer with digest checks.

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const REPOSITORY: &str = "Wangdage666/GeCL-Launcher";
const MAX_INSTALLER_BYTES: u64 = 512 * 1024 * 1024;
const MAX_RELEASE_BYTES: usize = 2 * 1024 * 1024;
const MAX_NOTES_CHARS: usize = 4000;
const CHUNK_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub notes: String,
    pub page_url: String,
    pub asset_name: String,
    pub asset_url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Copy, Serialize)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

/// Numeric `major.minor.patch` of a tag or package version; suffixes such as `-local` are ignored.
pub fn version_triple(value: &str) -> Option<(u64, u64, u64)> {
    let core = value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn safe_asset_name(name: &str) -> bool {
    name.ends_with("_x64-setup.exe")
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn release_update(
    release: Release,
    current: &str,
) -> anyhow::Result<Option<UpdateInfo>> {
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let latest =
        version_triple(&release.tag_name).context("发布版本号无法识别")?;
    let installed = version_triple(current).context("当前版本号无法识别")?;
    if latest <= installed {
        return Ok(None);
    }
    let prefix = format!("https://github.com/{REPOSITORY}/releases/download/");
    let asset = release
        .assets
        .into_iter()
        .find(|asset| safe_asset_name(&asset.name))
        .context("发布中没有 Windows 安装包")?;
    if !asset.browser_download_url.starts_with(&prefix) {
        bail!("安装包地址不属于 GeCL 发布页");
    }
    if asset.size == 0 || asset.size > MAX_INSTALLER_BYTES {
        bail!("安装包大小异常");
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|hash| {
            hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
        })
        .context("发布缺少安装包校验值，无法安全更新")?
        .to_ascii_lowercase();
    let notes = release
        .body
        .unwrap_or_default()
        .chars()
        .take(MAX_NOTES_CHARS)
        .collect();
    Ok(Some(UpdateInfo {
        version: release.tag_name.trim().trim_start_matches(['v', 'V']).to_string(),
        current: current.to_string(),
        notes,
        page_url: release.html_url,
        asset_name: asset.name,
        asset_url: asset.browser_download_url,
        size: asset.size,
        sha256,
    }))
}

fn client(timeout: Option<Duration>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .https_only(true)
        .connect_timeout(Duration::from_secs(10));
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    Ok(builder.build()?)
}

async fn bounded_body(
    mut response: reqwest::Response,
    limit: usize,
) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > limit {
            bail!("GitHub 响应过大");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn check_api(current: &str) -> anyhow::Result<Option<UpdateInfo>> {
    let response = client(Some(Duration::from_secs(15)))?
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .context("无法连接 GitHub")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response =
        response.error_for_status().context("GitHub 发布查询失败")?;
    release_update(
        serde_json::from_slice(
            &bounded_body(response, MAX_RELEASE_BYTES).await?,
        )
        .context("GitHub 发布信息格式错误")?,
        current,
    )
}

/// Tag named by the `/releases/latest` redirect, e.g. `v1.0.1`.
fn tag_from_location(location: &str) -> Option<String> {
    let url = url::Url::parse("https://github.com")
        .ok()?
        .join(location)
        .ok()?;
    if url.host_str() != Some("github.com") {
        return None;
    }
    let prefix = format!("/{REPOSITORY}/releases/tag/");
    let tag = url.path().strip_prefix(&prefix)?;
    (!tag.contains('/') && version_triple(tag).is_some())
        .then(|| tag.to_string())
}

/// Installer name and SHA-256 from a `sha256sum`-style listing.
fn installer_from_sums(sums: &str) -> Option<(String, String)> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (parts.next().is_none()
            && safe_asset_name(name)
            && hash.len() == 64
            && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| (name.to_string(), hash.to_ascii_lowercase()))
    })
}

/// Used when the API is unavailable (e.g. rate limited): the latest-release redirect plus the release's `SHA256SUMS.txt`.
async fn check_release_page(
    current: &str,
) -> anyhow::Result<Option<UpdateInfo>> {
    let response = reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build()?
        .get(format!("https://github.com/{REPOSITORY}/releases/latest"))
        .send()
        .await
        .context("无法连接 GitHub")?;
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .context("GitHub 没有返回最新版本")?;
    let tag = tag_from_location(location).context("发布版本号无法识别")?;
    let latest = version_triple(&tag).context("发布版本号无法识别")?;
    if latest <= version_triple(current).context("当前版本号无法识别")? {
        return Ok(None);
    }
    let download =
        format!("https://github.com/{REPOSITORY}/releases/download/{tag}/");
    let response = client(Some(Duration::from_secs(20)))?
        .get(format!("{download}SHA256SUMS.txt"))
        .send()
        .await
        .context("无法连接 GitHub")?
        .error_for_status()
        .context("发布缺少安装包校验值，无法安全更新")?;
    let sums = String::from_utf8(bounded_body(response, 64 * 1024).await?)
        .context("发布校验文件格式错误")?;
    let (asset_name, sha256) =
        installer_from_sums(sums.trim_start_matches('\u{feff}'))
            .context("发布缺少安装包校验值，无法安全更新")?;
    Ok(Some(UpdateInfo {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        current: current.to_string(),
        notes: String::new(),
        page_url: format!(
            "https://github.com/{REPOSITORY}/releases/tag/{tag}"
        ),
        asset_url: format!("{download}{asset_name}"),
        asset_name,
        size: 0,
        sha256,
    }))
}

/// Returns the latest published release when it is newer than `current`.
pub async fn check(current: &str) -> anyhow::Result<Option<UpdateInfo>> {
    match check_api(current).await {
        Ok(info) => Ok(info),
        Err(error) => {
            tracing::warn!(
                "GitHub release API failed, trying release page: {error:#}"
            );
            check_release_page(current).await.map_err(|fallback| {
                tracing::warn!("GitHub release page check failed: {fallback:#}");
                error
            })
        }
    }
}

pub fn download_dir() -> PathBuf {
    std::env::temp_dir().join("gecl-update")
}

/// SHA-256 of a file, read in blocks.
pub async fn file_sha256(path: &Path) -> anyhow::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Downloads the installer into the update folder; the file is published only after size and SHA-256 match.
pub async fn download(
    info: &UpdateInfo,
    cancelled: &AtomicBool,
    on_progress: impl Fn(Progress),
) -> anyhow::Result<PathBuf> {
    if !safe_asset_name(&info.asset_name)
        || !info
            .asset_url
            .starts_with(&format!("https://github.com/{REPOSITORY}/releases/download/"))
    {
        bail!("更新信息无效");
    }
    let directory = download_dir();
    tokio::fs::create_dir_all(&directory).await?;
    let target = directory.join(&info.asset_name);
    if tokio::fs::try_exists(&target).await?
        && file_sha256(&target).await? == info.sha256
    {
        let size = tokio::fs::metadata(&target).await?.len();
        on_progress(Progress { done: size, total: size });
        return Ok(target);
    }
    let partial = directory.join(format!("{}.part", info.asset_name));
    let result = async {
        let mut response = client(None)?
            .get(&info.asset_url)
            .send()
            .await
            .context("无法下载安装包")?
            .error_for_status()
            .context("无法下载安装包")?;
        // A release-page fallback has no size; the response length is then the expectation.
        let total = match (info.size, response.content_length()) {
            (0, Some(length)) => length,
            (size, _) => size,
        };
        if total > MAX_INSTALLER_BYTES {
            bail!("安装包大小异常");
        }
        let limit = if total > 0 { total } else { MAX_INSTALLER_BYTES };
        let mut file = tokio::fs::File::create(&partial).await?;
        let mut hasher = Sha256::new();
        let mut done = 0u64;
        let mut reported = 0u64;
        on_progress(Progress { done, total });
        loop {
            if cancelled.load(Ordering::Relaxed) {
                bail!("已取消更新");
            }
            let Some(chunk) = tokio::time::timeout(CHUNK_TIMEOUT, response.chunk())
                .await
                .context("下载超时")??
            else {
                break;
            };
            done += chunk.len() as u64;
            if done > limit {
                bail!("安装包大小与发布信息不一致");
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            if done - reported >= 256 * 1024 || done == total {
                reported = done;
                on_progress(Progress { done, total: total.max(done) });
            }
        }
        if total > 0 && done != total {
            bail!("安装包下载不完整");
        }
        if format!("{:x}", hasher.finalize()) != info.sha256 {
            bail!("安装包校验失败");
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        if tokio::fs::try_exists(&target).await? {
            tokio::fs::remove_file(&target).await?;
        }
        tokio::fs::rename(&partial, &target).await?;
        Ok(target.clone())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&partial).await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, digest: Option<&str>, url: &str) -> Release {
        Release {
            tag_name: tag.into(),
            body: Some("说明".into()),
            html_url: format!("https://github.com/{REPOSITORY}/releases/tag/{tag}"),
            draft: false,
            prerelease: false,
            assets: vec![Asset {
                name: "gecl_1.0.1_x64-setup.exe".into(),
                size: 1024,
                browser_download_url: url.into(),
                digest: digest.map(str::to_string),
            }],
        }
    }

    const URL: &str = "https://github.com/Wangdage666/GeCL-Launcher/releases/download/v1.0.1/gecl_1.0.1_x64-setup.exe";
    const DIGEST: &str = "sha256:728E0ADDE44CDE4C26DD2455DBDF1F7EB55F81E43855D97CFB480A419FC26F50";

    #[test]
    fn version_triples_ignore_local_suffix() {
        assert_eq!(version_triple("v1.0.1"), Some((1, 0, 1)));
        assert_eq!(version_triple("1.0.0-local"), Some((1, 0, 0)));
        assert_eq!(version_triple("2"), Some((2, 0, 0)));
        assert_eq!(version_triple("1.0.0.1"), None);
        assert_eq!(version_triple("latest"), None);
    }

    #[test]
    fn newer_release_returns_verified_installer() {
        let info = release_update(release("v1.0.1", Some(DIGEST), URL), "1.0.0-local")
            .unwrap()
            .unwrap();
        assert_eq!(info.version, "1.0.1");
        assert_eq!(info.sha256, DIGEST[7..].to_ascii_lowercase());
        assert_eq!(info.asset_url, URL);
    }

    #[test]
    fn same_older_and_prerelease_are_not_updates() {
        assert!(release_update(release("v1.0.1", Some(DIGEST), URL), "1.0.1").unwrap().is_none());
        assert!(release_update(release("v1.0.0", Some(DIGEST), URL), "1.0.1").unwrap().is_none());
        let mut pre = release("v2.0.0", Some(DIGEST), URL);
        pre.prerelease = true;
        assert!(release_update(pre, "1.0.1").unwrap().is_none());
    }

    /// Run with `--ignored`: reads the real latest GitHub release and downloads its installer.
    #[tokio::test]
    #[ignore]
    async fn real_release_downloads_and_matches_digest() {
        let info = check("0.0.1").await.unwrap().expect("a published release");
        let path = download(&info, &AtomicBool::new(false), |_| {}).await.unwrap();
        assert_eq!(file_sha256(&path).await.unwrap(), info.sha256);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), info.size);
        let cancelled = AtomicBool::new(true);
        std::fs::remove_file(&path).unwrap();
        assert!(download(&info, &cancelled, |_| {}).await.is_err());
        assert!(!path.exists());
        assert!(!download_dir().join(format!("{}.part", info.asset_name)).exists());
    }

    /// Run with `--ignored`: follows the real latest-release redirect without the API.
    #[tokio::test]
    #[ignore]
    async fn real_release_page_reports_latest_tag() {
        assert!(check_release_page("9999.0.0").await.unwrap().is_none());
        match check_release_page("0.0.1").await {
            Ok(Some(info)) => assert!(version_triple(&info.version).is_some()),
            Ok(None) => panic!("an older version must see the latest release"),
            Err(error) => assert!(error.to_string().contains("校验值"), "{error:#}"),
        }
    }

    #[test]
    fn release_page_fallback_parses_tag_and_checksums() {
        assert_eq!(
            tag_from_location(
                "https://github.com/Wangdage666/GeCL-Launcher/releases/tag/v1.0.1"
            ),
            Some("v1.0.1".into())
        );
        assert_eq!(
            tag_from_location("https://github.com/other/repo/releases/tag/v9.0.0"),
            None
        );
        assert_eq!(
            tag_from_location("https://example.com/Wangdage666/GeCL-Launcher/releases/tag/v9.0.0"),
            None
        );
        let sums = "38609d5b08a9086deb3064e335716d290cb0a0e1f811a458e277d82f5a42e0c1  gecl.exe\r\n\
            728E0ADDE44CDE4C26DD2455DBDF1F7EB55F81E43855D97CFB480A419FC26F50 *gecl_1.0.1_x64-setup.exe\r\n";
        assert_eq!(
            installer_from_sums(sums),
            Some((
                "gecl_1.0.1_x64-setup.exe".into(),
                "728e0adde44cde4c26dd2455dbdf1f7eb55f81e43855d97cfb480a419fc26f50".into()
            ))
        );
        assert_eq!(installer_from_sums("abc  gecl_1.0.1_x64-setup.exe"), None);
    }

    #[test]
    fn missing_digest_or_foreign_url_is_rejected() {
        assert!(release_update(release("v1.0.1", None, URL), "1.0.0").is_err());
        assert!(
            release_update(
                release("v1.0.1", Some(DIGEST), "https://example.com/gecl_1.0.1_x64-setup.exe"),
                "1.0.0"
            )
            .is_err()
        );
        let mut unsafe_name = release("v1.0.1", Some(DIGEST), URL);
        unsafe_name.assets[0].name = "..\\gecl_x64-setup.exe".into();
        assert!(release_update(unsafe_name, "1.0.0").is_err());
    }
}
