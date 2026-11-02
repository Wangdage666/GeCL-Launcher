use super::{File, Project};
use crate::state::{
    ContentItem, ContentItemOwner, ContentItemProject, ContentItemVersion,
    License, OwnerType,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

const PREFIX: &str = "gecl_curseforge_file:";
pub(crate) fn key(instance: &str, sha512: &str) -> String {
    format!("{PREFIX}{instance}:{sha512}")
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPlatform {
    pub name: String,
    pub project_id: u64,
    pub file_id: u64,
    pub project_url: String,
    pub author_url: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SavedFile {
    pub project: Project,
    pub file: File,
}
impl SavedFile {
    pub(crate) async fn save<
        'e,
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    >(
        &self,
        instance: &str,
        hash: &str,
        exec: E,
    ) -> crate::Result<()> {
        sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
            .bind(key(instance,hash)).bind(serde_json::to_string(self)?).execute(exec).await?;
        Ok(())
    }
    pub(crate) fn apply(&self, item: &mut ContentItem) {
        let project = &self.project;
        let file = &self.file;
        item.platform = Some(ContentPlatform {
            name: "curseforge".into(),
            project_id: project.id,
            file_id: file.id,
            project_url: project.links.website_url.clone(),
            author_url: project.authors.first().map(|a| a.url.clone()),
        });
        item.project = Some(ContentItemProject {
            id: project.id.to_string(),
            slug: Some(project.slug.clone()),
            title: project.name.clone(),
            icon_url: project
                .logo
                .as_ref()
                .map(|l| l.thumbnail_url.as_ref().unwrap_or(&l.url).clone()),
            license: License {
                id: "unknown".into(),
                name: "".into(),
                url: None,
            },
            categories: vec![],
            additional_categories: vec![],
        });
        item.version = Some(ContentItemVersion {
            id: file.id.to_string(),
            version_number: item
                .embedded_metadata
                .as_ref()
                .and_then(|metadata| metadata.version.as_deref())
                .filter(|version| !version.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    let label = file.display_name.trim();
                    if label == file.file_name
                        || label.to_ascii_lowercase().ends_with(".jar")
                    {
                        "未知".to_string()
                    } else {
                        label.to_string()
                    }
                }),
            file_name: item.file_name.clone(),
            date_published: Some(file.file_date.clone()),
        });
        item.owner = project.authors.first().map(|author| ContentItemOwner {
            id: author.id.to_string(),
            name: project
                .authors
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join("、"),
            avatar_url: author.avatar_url.clone(),
            owner_type: OwnerType::User,
        });
        item.has_update = false;
        item.update_version_id = None;
    }
}
pub async fn installed_projects()
-> crate::Result<std::collections::HashMap<String, Vec<u64>>> {
    let state = crate::State::get().await?;
    installed_projects_in(&state.pool).await
}
async fn installed_projects_in(
    pool: &SqlitePool,
) -> crate::Result<std::collections::HashMap<String, Vec<u64>>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT file.instance_id,file.sha1,origin.value FROM instance_files file JOIN store_instance_files binding ON binding.file_id=file.id JOIN app_metadata origin ON origin.key=? || file.instance_id || ':' || binding.blob_sha512 WHERE file.enabled=1 AND file.missing=0")
        .bind(PREFIX).fetch_all(pool).await?;
    let mut result: std::collections::HashMap<String, Vec<u64>> =
        std::collections::HashMap::new();
    for (instance, hash, value) in rows {
        if let Ok(saved) = serde_json::from_str::<SavedFile>(&value)
            && super::sha1(&saved.file)
                .is_ok_and(|expected| expected.eq_ignore_ascii_case(&hash))
        {
            let ids = result.entry(instance).or_default();
            if !ids.contains(&saved.project.id) {
                ids.push(saved.project.id);
            }
        }
    }
    Ok(result)
}

pub(crate) async fn saved_files(
    instance: &str,
    pool: &SqlitePool,
) -> crate::Result<std::collections::HashMap<String, SavedFile>> {
    let prefix = format!("{PREFIX}{instance}:");
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key,value FROM app_metadata WHERE substr(key,1,?) = ?",
    )
    .bind(prefix.len() as i64)
    .bind(&prefix)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(key, value)| {
            Some((
                key.strip_prefix(&prefix)?.to_string(),
                serde_json::from_str(&value).ok()?,
            ))
        })
        .collect())
}
pub(crate) async fn known_sha1(
    instance: &str,
    pool: &SqlitePool,
) -> crate::Result<std::collections::HashSet<String>> {
    let hashes: Vec<String> = sqlx::query_scalar("SELECT file.sha1 FROM instance_files file JOIN store_instance_files binding ON binding.file_id=file.id JOIN app_metadata origin ON origin.key=? || binding.blob_sha512 WHERE file.instance_id=?")
        .bind(format!("{PREFIX}{instance}:")).bind(instance).fetch_all(pool).await?;
    Ok(hashes.into_iter().collect())
}
pub(crate) async fn overlay_content(
    instance: &str,
    items: &mut [ContentItem],
    pool: &SqlitePool,
) -> crate::Result<()> {
    let saved = saved_files(instance, pool).await?;
    if saved.is_empty() {
        return Ok(());
    }
    let bindings: Vec<(String,String,String)> = sqlx::query_as("SELECT file.relative_path,file.sha1,binding.blob_sha512 FROM instance_files file JOIN store_instance_files binding ON binding.file_id=file.id WHERE file.instance_id=? AND file.missing=0")
        .bind(instance).fetch_all(pool).await?;
    let bindings = bindings
        .into_iter()
        .map(|(path, sha1, sha512)| {
            (
                (path.trim_end_matches(".disabled").to_string(), sha1),
                sha512,
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    for item in items {
        let key = (
            item.file_path.trim_end_matches(".disabled").to_string(),
            item.id.clone(),
        );
        if let Some(origin) =
            bindings.get(&key).and_then(|sha512| saved.get(sha512))
        {
            origin.apply(item);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SavedFile {
        serde_json::from_value(serde_json::json!({
            "project":{"id":1450249,"gameId":432,"classId":6,"name":"Easy Peasy Gunpowder","slug":"easypeasygunpowder","summary":"Two recipes","downloadCount":33510,"links":{"websiteUrl":"https://www.curseforge.com/minecraft/mc-mods/easypeasygunpowder"},"authors":[{"id":42,"name":"Author","url":"https://www.curseforge.com/members/author"}],"logo":{"url":"https://media.forgecdn.net/icon.png"},"dateModified":"2026-05-16","dateCreated":"2026-02-01"},
            "file":{"id":8097991,"modId":1450249,"fileName":"mod.jar","displayName":"Forge 1.1.0","fileLength":4997,"fileDate":"2026-05-16","downloadCount":63,"releaseType":1,"isAvailable":true,"gameVersions":["1.20.1","Forge"]}
        })).unwrap()
    }
    fn item() -> ContentItem {
        serde_json::from_value(serde_json::json!({"file_name":"renamed.jar.disabled","file_path":"mods/renamed.jar.disabled","id":"testhash","size":4997,"enabled":false,"locked":false,"project_type":"mod","has_update":true,"update_version_id":"mr-version"})).unwrap()
    }
    #[test]
    fn embedded_version_is_separate_from_archive_filename() {
        let mut origin = fixture();
        origin.file.display_name =
            "easypeasygunpowder-1.20.1-forge-1.1.0.jar".into();
        origin.file.file_name = origin.file.display_name.clone();
        let mut content = item();
        content.embedded_metadata = Some(
            serde_json::from_value(serde_json::json!({"version":"1.1.0"}))
                .unwrap(),
        );
        origin.apply(&mut content);
        assert_eq!(content.version.as_ref().unwrap().version_number, "1.1.0");
        assert_eq!(
            content.version.as_ref().unwrap().file_name,
            "renamed.jar.disabled"
        );
        content.embedded_metadata = None;
        origin.apply(&mut content);
        assert_eq!(content.version.as_ref().unwrap().version_number, "未知");
        origin.file.display_name = "Release 1.1.0".into();
        origin.apply(&mut content);
        assert_eq!(
            content.version.as_ref().unwrap().version_number,
            "Release 1.1.0"
        );
    }

    #[tokio::test]
    async fn installed_index_requires_enabled_present_matching_content() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE app_metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE instance_files(id TEXT,instance_id TEXT,sha1 TEXT,enabled INTEGER,missing INTEGER)").execute(&pool).await.unwrap();
        sqlx::query(
            "CREATE TABLE store_instance_files(file_id TEXT,blob_sha512 TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let hash = "a".repeat(40);
        let mut origin = fixture();
        origin.file.hashes = serde_json::from_value(
            serde_json::json!([{"algo":1,"value":hash}]),
        )
        .unwrap();
        origin.save("game", "blob", &pool).await.unwrap();
        sqlx::query("INSERT INTO instance_files VALUES ('file','game',?,1,0)")
            .bind(&hash)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO store_instance_files VALUES ('file','blob')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            installed_projects_in(&pool).await.unwrap()["game"],
            vec![1450249]
        );
        sqlx::query("UPDATE instance_files SET enabled=0")
            .execute(&pool)
            .await
            .unwrap();
        assert!(installed_projects_in(&pool).await.unwrap().is_empty());
        sqlx::query("UPDATE instance_files SET enabled=1,missing=1")
            .execute(&pool)
            .await
            .unwrap();
        assert!(installed_projects_in(&pool).await.unwrap().is_empty());
        sqlx::query("UPDATE instance_files SET missing=0,sha1='changed'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(installed_projects_in(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn provenance_survives_reload_and_disabled_rename_and_is_instance_scoped()
     {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE app_metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE instance_files(id TEXT,instance_id TEXT,relative_path TEXT,sha1 TEXT,missing INTEGER)").execute(&pool).await.unwrap();
        sqlx::query(
            "CREATE TABLE store_instance_files(file_id TEXT,blob_sha512 TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO instance_files VALUES ('file','game-a','mods/renamed.jar','testhash',0)").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO store_instance_files VALUES ('file','different-sha512')").execute(&pool).await.unwrap();
        fixture()
            .save("game-a", "different-sha512", &pool)
            .await
            .unwrap();
        assert!(
            known_sha1("game-a", &pool)
                .await
                .unwrap()
                .contains("testhash")
        );
        assert!(known_sha1("game-b", &pool).await.unwrap().is_empty());
        let mut a = vec![item()];
        overlay_content("game-a", &mut a, &pool).await.unwrap();
        assert_eq!(
            a[0].project.as_ref().unwrap().title,
            "Easy Peasy Gunpowder"
        );
        assert_eq!(a[0].owner.as_ref().unwrap().name, "Author");
        assert_eq!(a[0].owner.as_ref().unwrap().avatar_url, None);
        assert!(a[0].project.as_ref().unwrap().icon_url.is_some());
        assert_eq!(
            a[0].version.as_ref().unwrap().file_name,
            "renamed.jar.disabled"
        );
        assert_eq!(a[0].platform.as_ref().unwrap().file_id, 8097991);
        assert!(!a[0].has_update);
        assert!(a[0].update_version_id.is_none());
        let mut b = vec![item()];
        overlay_content("game-b", &mut b, &pool).await.unwrap();
        assert!(b[0].platform.is_none());
        a[0].id = "changed-binary".into();
        a[0].project = None;
        overlay_content("game-a", &mut a, &pool).await.unwrap();
        assert!(a[0].project.is_none());
    }
    #[tokio::test]
    async fn metadata_transaction_rolls_back_and_corrupt_optional_record_does_not_break_list()
     {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE app_metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL)").execute(&pool).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        fixture().save("game", "hash", &mut *tx).await.unwrap();
        tx.rollback().await.unwrap();
        assert!(saved_files("game", &pool).await.unwrap().is_empty());
        sqlx::query("INSERT INTO app_metadata VALUES ('gecl_curseforge_file:game:broken','not json')").execute(&pool).await.unwrap();
        assert!(saved_files("game", &pool).await.unwrap().is_empty());
    }
}
