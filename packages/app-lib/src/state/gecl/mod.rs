pub mod pack_artwork;
pub mod world_backups;
pub mod appearance;
mod local_games;
pub use local_games::{
    LocalGame, discover, finalize_import, import_folder, preview_folder,
};

use super::{DirectoryInfo, Settings, State};
use anyhow::{Context, bail};
use serde::Serialize;
use sqlx::sqlite::SqliteConnectOptions;
use std::path::PathBuf;

const SETUP_KEY: &str = "gecl_setup_pending";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub needs_directory: bool,
    pub needs_account: bool,
    pub data_directory: Option<String>,
}

pub async fn setup_status(identifier: &str) -> anyhow::Result<SetupStatus> {
    let directory = DirectoryInfo::initial_settings_dir_path(identifier)
        .context("无法确定设置目录")?;
    let database = directory.join("app.db");
    if !tokio::fs::try_exists(&database).await? {
        return Ok(SetupStatus {
            needs_directory: true,
            needs_account: true,
            data_directory: Some(
                std::env::current_exe()?
                    .parent()
                    .context("无法确定安装目录")?
                    .to_string_lossy()
                    .into_owned(),
            ),
        });
    }
    let pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(database)
            .read_only(true),
    )
    .await?;
    let pending = account_setup_required(&pool).await?;
    let selected: Option<String> =
        sqlx::query_scalar("SELECT custom_dir FROM settings LIMIT 1")
            .fetch_one(&pool)
            .await?;
    pool.close().await;
    Ok(SetupStatus {
        needs_directory: false,
        needs_account: pending,
        data_directory: selected,
    })
}

pub(super) async fn setup_pending(
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<bool> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
            .bind(SETUP_KEY)
            .fetch_optional(pool)
            .await?;
    Ok(value.as_deref() == Some("true"))
}

async fn account_setup_required(
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<bool> {
    if setup_pending(pool).await? {
        return Ok(true);
    }
    for table in ["minecraft_users", "gecl_offline_accounts"] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
        ).bind(table).fetch_one(pool).await?;
        if exists {
            let query = match table {
                "minecraft_users" => {
                    "SELECT EXISTS(SELECT 1 FROM minecraft_users)"
                }
                _ => "SELECT EXISTS(SELECT 1 FROM gecl_offline_accounts)",
            };
            if sqlx::query_scalar::<_, bool>(query).fetch_one(pool).await? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub async fn choose_directory(
    identifier: &str,
    path: PathBuf,
) -> anyhow::Result<()> {
    if State::initialized() {
        bail!("程序已经初始化，请在设置中更改游戏目录");
    }
    if !path.is_absolute() || path.parent().is_none() {
        bail!("请选择磁盘中的专用文件夹");
    }
    let status = setup_status(identifier).await?;
    if !status.needs_directory {
        bail!("已有数据目录，不能通过首次设置覆盖");
    }
    tokio::fs::create_dir_all(&path)
        .await
        .context("无法创建所选目录")?;
    let path = tokio::fs::canonicalize(path).await?;
    let path = dunce::simplified(&path).to_path_buf();
    for name in [
        "profiles",
        "meta",
        "caches",
        "store",
        "icons",
        "synced-options",
    ] {
        let folder = path.join(name);
        tokio::fs::create_dir_all(&folder)
            .await
            .context("所选位置无法创建游戏数据目录，请选择其他位置")?;
        let probe =
            folder.join(format!(".gecl-write-test-{}", uuid::Uuid::new_v4()));
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .await
            .context("所选游戏数据目录不可写，请选择其他位置")?;
        drop(file);
        tokio::fs::remove_file(probe).await?;
    }
    let lock = super::content_store::ContentStore::lock_process(&path)
        .await
        .context("无法打开所选目录的 store.lock；请重新安装以修复权限，或选择其他目录")?;
    drop(lock);
    let pool = super::db::connect(identifier).await?;
    let mut settings = Settings::get(&pool).await?;
    settings.custom_dir = Some(path.to_string_lossy().into_owned());
    settings.prev_custom_dir = settings.custom_dir.clone();
    let mut tx = pool.begin().await?;
    settings.update(&mut *tx).await?;
    sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?, 'true') ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(SETUP_KEY).execute(&mut *tx).await?;
    tx.commit().await?;
    pool.close().await;
    Ok(())
}

pub async fn finish_setup() -> anyhow::Result<()> {
    let state = State::get().await?;
    let count: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM minecraft_users) + (SELECT COUNT(*) FROM gecl_offline_accounts)")
        .fetch_one(&state.pool).await?;
    if count == 0 {
        bail!("请先添加一个离线或微软账户");
    }
    sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?, 'false') ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(SETUP_KEY).execute(&state.pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_accounts_require_setup_after_real_database_restart() {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("app.db"))
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(options.clone())
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?, 'false')")
            .bind(SETUP_KEY)
            .execute(&pool)
            .await
            .unwrap();
        assert!(account_setup_required(&pool).await.unwrap());
        let id = super::super::offline_accounts::offline_identity("TestUser")
            .unwrap();
        sqlx::query("INSERT INTO gecl_offline_accounts(uuid,username,active) VALUES (?, 'TestUser', TRUE)")
            .bind(id.to_string()).execute(&pool).await.unwrap();
        assert!(!account_setup_required(&pool).await.unwrap());
        pool.close().await;
        let pool = sqlx::SqlitePool::connect_with(options.clone())
            .await
            .unwrap();
        assert!(!account_setup_required(&pool).await.unwrap());
        super::super::Credentials::remove(id, &pool).await.unwrap();
        pool.close().await;
        let pool = sqlx::SqlitePool::connect_with(options).await.unwrap();
        assert!(account_setup_required(&pool).await.unwrap());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM gecl_offline_accounts"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        assert!(!setup_pending(&pool).await.unwrap());
        pool.close().await;
    }

    #[tokio::test]
    async fn legacy_microsoft_account_does_not_require_new_offline_account() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql("CREATE TABLE app_metadata(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE minecraft_users(uuid TEXT PRIMARY KEY); INSERT INTO minecraft_users VALUES ('existing');")
            .execute(&pool).await.unwrap();
        assert!(!account_setup_required(&pool).await.unwrap());
        sqlx::query("INSERT INTO app_metadata VALUES (?, 'true')")
            .bind(SETUP_KEY)
            .execute(&pool)
            .await
            .unwrap();
        assert!(account_setup_required(&pool).await.unwrap());
    }
}
