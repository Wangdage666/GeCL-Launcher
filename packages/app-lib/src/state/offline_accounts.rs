use super::{AccountType, Credentials, MinecraftProfile};
use chrono::Utc;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct OfflineAccount {
    pub uuid: String,
    pub username: String,
    pub active: bool,
}

impl OfflineAccount {
    pub fn into_credentials(self) -> crate::Result<Credentials> {
        Ok(Credentials {
            account_type: AccountType::Offline,
            offline_profile: MinecraftProfile {
                id: Uuid::parse_str(&self.uuid)?,
                name: self.username,
                ..MinecraftProfile::default()
            },
            access_token: String::new(),
            refresh_token: String::new(),
            expires: Utc::now(),
            active: self.active,
        })
    }
}

pub fn offline_identity(username: &str) -> crate::Result<Uuid> {
    if !(3..=16).contains(&username.len())
        || !username
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        anyhow::bail!("离线用户名须为 3–16 位英文字母、数字或下划线");
    }
    let mut bytes = md5::compute(format!("OfflinePlayer:{username}")).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gecl_java_compatible_identity() {
        assert_eq!(
            offline_identity("Notch").unwrap().to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_ne!(
            offline_identity("Player").unwrap(),
            offline_identity("player").unwrap()
        );
    }

    #[test]
    fn gecl_rejects_invalid_names() {
        for name in ["", "ab", "../player", "玩家", "a b", "abcdefghijklmnopq"]
        {
            assert!(offline_identity(name).is_err());
        }
        assert!(offline_identity("Player_123").is_ok());
    }

    #[tokio::test]
    async fn gecl_offline_persistence_and_exclusive_selection() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql("CREATE TABLE minecraft_users (uuid TEXT PRIMARY KEY, active INTEGER NOT NULL);")
            .execute(&pool).await.unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/20261020000000_gecl_offline_accounts.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for name in ["PlayerOne", "PlayerTwo", "PlayerTwo"] {
            OfflineAccount {
                uuid: offline_identity(name).unwrap().to_string(),
                username: name.into(),
                active: true,
            }
            .into_credentials()
            .unwrap()
            .upsert(&pool)
            .await
            .unwrap();
        }
        let accounts = sqlx::query_as::<_, OfflineAccount>(
            "SELECT uuid, username, active FROM gecl_offline_accounts",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts.iter().filter(|account| account.active).count(), 1);
        let credentials =
            Credentials::get_active(&pool).await.unwrap().unwrap();
        assert_eq!(credentials.offline_profile.name, "PlayerTwo");
        assert_eq!(credentials.account_type, AccountType::Offline);
        assert!(credentials.access_token.is_empty());
        assert!(credentials.refresh_token.is_empty());
        sqlx::query("INSERT INTO minecraft_users (uuid, active) VALUES ('microsoft-test', TRUE)")
            .execute(&pool).await.unwrap();
        let selected: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM gecl_offline_accounts WHERE active = TRUE",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(selected, 0);
        credentials.upsert(&pool).await.unwrap();
        let selected: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM minecraft_users WHERE active = TRUE",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(selected, 0);
        Credentials::remove(credentials.offline_profile.id, &pool)
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM gecl_offline_accounts")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }
}
