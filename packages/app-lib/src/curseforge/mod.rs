//! CurseForge metadata is kept separate from Modrinth identifiers and caches.
mod install;
mod metadata;
pub use install::{InstallProgress, cancel, install};
pub(crate) use install::{PreparedMod, checkpoint, sha1};
pub(crate) use metadata::key as metadata_key;
pub use metadata::{ContentPlatform, installed_projects};
pub(crate) use metadata::{
    SavedFile, known_sha1, overlay_content, saved_files,
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

const MIRROR: &str = "https://mod.mcimirror.top/curseforge/v1/";
const OFFICIAL: &str = "https://api.curseforge.com/v1/";
const WIDGET: &str = "https://api.cfwidget.com/";
static SEARCH_PROJECTS: LazyLock<Mutex<HashMap<u64, (Instant, Project)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CACHE: LazyLock<Mutex<HashMap<String, (Instant, serde_json::Value)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub id: u64,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub website_url: String,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub slug: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: u64,
    pub game_id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: f64,
    pub links: Links,
    #[serde(default)]
    pub authors: Vec<Author>,
    pub logo: Option<Asset>,
    #[serde(default)]
    pub screenshots: Vec<Asset>,
    #[serde(default)]
    pub categories: Vec<Category>,
    pub class_id: Option<u64>,
    pub date_modified: String,
    pub date_created: String,
    #[serde(default)]
    pub allow_mod_distribution: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub value: String,
    pub algo: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub mod_id: u64,
    pub relation_type: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: u64,
    pub mod_id: u64,
    pub file_name: String,
    pub display_name: String,
    pub file_length: u64,
    pub file_date: String,
    pub download_count: f64,
    pub download_url: Option<String>,
    pub release_type: u8,
    #[serde(default)]
    pub is_available: bool,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub hashes: Vec<Hash>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}
#[derive(Deserialize)]
struct Data<T> {
    data: T,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    #[serde(default)]
    pub query: String,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    #[serde(default)]
    pub index: u32,
    pub page_size: Option<u32>,
    pub sort: Option<String>,
    pub category: Option<u64>,
    pub slug: Option<String>,
}

pub fn loader_type(loader: &str) -> Option<u8> {
    match loader.to_ascii_lowercase().as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}
fn parameters(request: &Search) -> crate::Result<Vec<(String, String)>> {
    if request.query.chars().count() > 200 || request.index >= 10_000 {
        bail!("搜索范围超出 CurseForge 限制");
    }
    let size = request
        .page_size
        .unwrap_or(20)
        .clamp(1, 50)
        .min(10_000 - request.index);
    let mut params = vec![
        ("index".into(), request.index.to_string()),
        ("pageSize".into(), size.to_string()),
    ];
    if let Some(version) = &request.game_version {
        params.push(("gameVersion".into(), version.clone()));
    }
    if let Some(loader) = &request.loader {
        let id = loader_type(loader).context("CurseForge 不支持所选加载器")?;
        params.push(("modLoaderType".into(), id.to_string()));
    }
    Ok(params)
}
async fn request<T: DeserializeOwned>(
    path: &str,
    params: &[(String, String)],
) -> crate::Result<T> {
    // A developer-owned key is sent only to the official API, never to the mirror or frontend.
    let key = std::env::var("GECL_CURSEFORGE_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let url = url::Url::parse(if key.is_some() { OFFICIAL } else { MIRROR })?
        .join(path)?;
    request_url(url, params, key).await
}
async fn request_url<T: DeserializeOwned>(
    mut url: url::Url,
    params: &[(String, String)],
    key: Option<String>,
) -> crate::Result<T> {
    url.query_pairs_mut()
        .extend_pairs(params.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let cache_key = url.to_string();
    let cached = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&cache_key)
        .filter(|(at, _)| at.elapsed() < Duration::from_secs(60))
        .map(|(_, v)| v.clone());
    if let Some(value) = cached {
        return Ok(serde_json::from_value(value)?);
    }
    let client = reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .https_only(true)
        .connect_timeout(Duration::from_secs(6))
        .timeout(Duration::from_secs(20))
        .build()?;
    let mut req = client.get(url);
    if let Some(key) = key {
        req = req.header("x-api-key", key);
    }
    let mut response = req.send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
            bail!("CurseForge 响应过大");
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let output = serde_json::from_value(value.clone())?;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= 128 {
        cache.clear();
    }
    cache.insert(cache_key, (Instant::now(), value));
    Ok(output)
}
pub async fn search(request_args: Search) -> crate::Result<Page<Project>> {
    let mut params = parameters(&request_args)?;
    params.extend([
        ("gameId".into(), "432".into()),
        ("classId".into(), "6".into()),
        ("searchFilter".into(), request_args.query),
        ("sortOrder".into(), "desc".into()),
    ]);
    let sort = match request_args.sort.as_deref() {
        Some("downloads") => "6",
        Some("newest") => "11",
        Some("updated") => "3",
        _ => "2",
    };
    params.push(("sortField".into(), sort.into()));
    if let Some(category) = request_args.category {
        params.push(("categoryId".into(), category.to_string()));
    }
    if let Some(slug) = request_args.slug {
        params.push(("slug".into(), slug));
    }
    let mut result: Page<Project> = request("mods/search", &params).await?;
    for project in &mut result.data {
        normalize_project(project)?;
    }
    {
        let mut cache =
            SEARCH_PROJECTS.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() + result.data.len() > 128 {
            cache.clear();
        }
        for project in &result.data {
            cache.insert(project.id, (Instant::now(), project.clone()));
        }
    }
    Ok(result)
}
pub async fn project(id: u64) -> crate::Result<Project> {
    let cached = SEARCH_PROJECTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&id)
        .filter(|(at, _)| at.elapsed() < Duration::from_secs(60))
        .map(|(_, p)| p.clone());
    if let Some(project) = cached {
        return Ok(project);
    }
    let mut result: Data<Project> = request(&format!("mods/{id}"), &[]).await?;
    if result.data.id != id
        || result.data.game_id != 432
        || result.data.class_id != Some(6)
    {
        bail!("项目并非 Minecraft 模组");
    }
    normalize_project(&mut result.data)?;
    Ok(result.data)
}
fn public_https(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
    })
}
fn normalize_project(project: &mut Project) -> crate::Result<()> {
    if project.game_id != 432
        || project.class_id != Some(6)
        || project.slug.is_empty()
        || !project
            .slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        bail!("信息源返回了无效的 Minecraft 模组");
    }
    project.links.website_url = format!(
        "https://www.curseforge.com/minecraft/mc-mods/{}",
        project.slug
    );
    for link in [
        &mut project.links.source_url,
        &mut project.links.wiki_url,
        &mut project.links.issues_url,
    ] {
        if link.as_ref().is_some_and(|value| !public_https(value)) {
            *link = None;
        }
    }
    for author in &mut project.authors {
        if !public_https(&author.url) {
            author.url = project.links.website_url.clone();
        }
        if author
            .avatar_url
            .as_ref()
            .is_some_and(|value| !public_https(value))
        {
            author.avatar_url = None;
        }
    }
    if project
        .logo
        .as_ref()
        .is_some_and(|logo| !public_https(&logo.url))
    {
        project.logo = None;
    }
    if let Some(logo) = &mut project.logo
        && logo
            .thumbnail_url
            .as_ref()
            .is_some_and(|value| !public_https(value))
    {
        logo.thumbnail_url = None;
    }
    project.screenshots.retain(|image| public_https(&image.url));
    Ok(())
}
pub async fn files(id: u64, args: Search) -> crate::Result<Page<File>> {
    request(&format!("mods/{id}/files"), &parameters(&args)?).await
}
pub async fn file(project: u64, id: u64) -> crate::Result<File> {
    let result: Data<File> =
        request(&format!("mods/{project}/files/{id}"), &[]).await?;
    if result.data.id != id || result.data.mod_id != project {
        bail!("文件与项目不匹配");
    }
    Ok(result.data)
}
pub async fn description(id: u64) -> crate::Result<String> {
    if id == 0 {
        bail!("无效 CurseForge 项目 ID");
    }
    let primary = tokio::time::timeout(
        Duration::from_secs(8),
        request::<Data<String>>(&format!("mods/{id}/description"), &[]),
    )
    .await;
    if let Ok(Ok(value)) = primary
        && !value.data.trim().is_empty()
    {
        return Ok(value.data);
    }
    let url = url::Url::parse(WIDGET)?.join(&id.to_string())?;
    let value: WidgetDescription = request_url(url, &[], None)
        .await
        .context("无法读取 CurseForge 介绍")?;
    validate_widget_description(id, value)
}

#[derive(Deserialize)]
struct WidgetDescription {
    id: u64,
    game: String,
    description: String,
}

fn validate_widget_description(
    id: u64,
    value: WidgetDescription,
) -> crate::Result<String> {
    if value.id != id || value.game != "minecraft" {
        bail!("介绍与 CurseForge 项目不匹配");
    }
    if value.description.trim().is_empty() {
        bail!("项目暂无介绍");
    }
    Ok(value.description)
}
pub async fn changelog(project: u64, id: u64) -> crate::Result<String> {
    let result: crate::Result<Data<String>> =
        request(&format!("mods/{project}/files/{id}/changelog"), &[]).await;
    match result {
        Ok(value) => Ok(value.data),
        Err(error)
            if error.chain().any(|source| {
                source
                    .downcast_ref::<reqwest::Error>()
                    .is_some_and(|source| {
                        source.status() == Some(reqwest::StatusCode::NOT_FOUND)
                    })
            }) =>
        {
            Ok(String::new())
        }
        Err(error) => Err(error),
    }
}
pub fn compatible(file: &File, game_version: &str, loader: &str) -> bool {
    file.is_available
        && file.game_versions.iter().any(|v| v == game_version)
        && file
            .game_versions
            .iter()
            .any(|v| v.eq_ignore_ascii_case(loader))
}
pub fn download_urls(file: &File) -> crate::Result<Vec<String>> {
    if !path_util::is_safe_file_name(&file.file_name)
        || !file.file_name.to_ascii_lowercase().ends_with(".jar")
    {
        bail!("无效模组文件名");
    }
    let mut urls = Vec::new();
    if let Some(raw) = &file.download_url
        && let Ok(url) = url::Url::parse(raw)
        && valid_cdn(&url)
    {
        urls.push(url);
    }
    // Public file IDs address published ForgeCDN objects. Empty API URLs are not proof of unavailability.
    let mut fallback = url::Url::parse("https://edge.forgecdn.net/")?;
    fallback
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("无效下载地址"))?
        .extend([
            "files",
            &(file.id / 1000).to_string(),
            &(file.id % 1000).to_string(),
            &file.file_name,
        ]);
    if !urls.contains(&fallback) {
        urls.push(fallback);
    }
    let mut result = Vec::new();
    for url in urls {
        result.push(url.to_string());
        let mut alternate = url;
        alternate.set_host(Some("mediafilez.forgecdn.net"))?;
        if !result.contains(&alternate.to_string()) {
            result.push(alternate.to_string());
        }
    }
    Ok(result)
}
fn valid_cdn(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some(
                "edge.forgecdn.net"
                    | "media.forgecdn.net"
                    | "mediafilez.forgecdn.net"
            )
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widget_description_preserves_author_html() {
        let html = "<h2>Features</h2><p>介绍 <a href=\"https://example.com\">wiki</a></p>";
        let value = WidgetDescription {
            id: 42,
            game: "minecraft".into(),
            description: html.into(),
        };
        assert_eq!(validate_widget_description(42, value).unwrap(), html);
    }
    #[test]
    fn widget_description_rejects_other_projects_and_games() {
        for (id, game) in [(43, "minecraft"), (42, "wow")] {
            let value = WidgetDescription {
                id,
                game: game.into(),
                description: "<p>Wrong project</p>".into(),
            };
            assert!(validate_widget_description(42, value).is_err());
        }
    }
    #[test]
    fn widget_description_rejects_empty_text() {
        let value = WidgetDescription {
            id: 42,
            game: "minecraft".into(),
            description: " \n\t".into(),
        };
        assert!(validate_widget_description(42, value).is_err());
    }
    fn sample() -> File {
        serde_json::from_value(serde_json::json!({"id":8097991,"modId":1450249,"fileName":"easypeasygunpowder-1.20.1-forge-1.1.0.jar","displayName":"1.1.0","fileLength":4997,"fileDate":"2026-05-16T11:23:15Z","downloadCount":63,"downloadUrl":null,"releaseType":1,"isAvailable":true,"gameVersions":["1.20.1","Forge"],"hashes":[]})).unwrap()
    }
    #[test]
    fn empty_address_uses_verified_public_cdn_scheme() {
        let urls = download_urls(&sample()).unwrap();
        assert_eq!(
            urls[0],
            "https://edge.forgecdn.net/files/8097/991/easypeasygunpowder-1.20.1-forge-1.1.0.jar"
        );
        assert!(urls[1].starts_with("https://mediafilez.forgecdn.net/"));
    }
    #[test]
    fn loader_and_version_must_both_match() {
        let f = sample();
        assert!(compatible(&f, "1.20.1", "forge"));
        assert!(!compatible(&f, "1.20.1", "fabric"));
        assert!(!compatible(&f, "1.21.1", "forge"));
    }
    #[test]
    fn unsafe_names_and_addresses_are_not_used() {
        let mut f = sample();
        f.file_name = "../mod.jar".into();
        assert!(download_urls(&f).is_err());
        f = sample();
        f.download_url = Some("https://unrelated.example/secret".into());
        assert_eq!(download_urls(&f).unwrap().len(), 2);
        f.file_name = "模组 #+.jar".into();
        let url = download_urls(&f).unwrap().remove(0);
        assert!(url.contains("%23+"));
        assert!(!url.contains('#'));
    }
    #[test]
    fn untrusted_links_are_normalized_without_changing_source_statistics() {
        let mut p:Project=serde_json::from_value(serde_json::json!({"id":42,"gameId":432,"classId":6,"name":"Example","slug":"example-mod","summary":"Summary","downloadCount":1234,"links":{"websiteUrl":"javascript:alert(1)","sourceUrl":"javascript:alert(2)"},"authors":[{"id":1,"name":"Creator","url":"javascript:alert(3)","avatarUrl":"http://unsafe.example/image.png"}],"dateModified":"2026-10-24","dateCreated":"2026-10-24"})).unwrap();
        normalize_project(&mut p).unwrap();
        assert_eq!(p.download_count, 1234.0);
        assert_eq!(
            p.links.website_url,
            "https://www.curseforge.com/minecraft/mc-mods/example-mod"
        );
        assert!(p.links.source_url.is_none());
        assert!(p.authors[0].avatar_url.is_none());
        assert_eq!(p.authors[0].url, p.links.website_url);
        p.class_id = Some(4471);
        assert!(normalize_project(&mut p).is_err());
    }
    #[test]
    fn bounded_pagination_and_loader_mapping() {
        assert!(
            parameters(&Search {
                index: 10000,
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            parameters(&Search {
                loader: Some("unknown".into()),
                ..Default::default()
            })
            .is_err()
        );
        assert_eq!(loader_type("neoforge"), Some(6));
        assert_eq!(loader_type("fabric"), Some(4));
    }
}
