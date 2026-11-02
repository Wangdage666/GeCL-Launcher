use super::*;
use std::sync::LazyLock;

const OAUTH_BASE: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0";
const SCOPE: &str = "XboxLive.signin offline_access";
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("HTTPS client")
});
static DEVICE_LOGIN: Mutex<Option<DeviceSession>> = Mutex::const_new(None);

struct DeviceSession {
    id: Uuid,
    code: String,
    expires: Instant,
    next_poll: Instant,
    interval: u64,
}

#[derive(Serialize)]
pub struct GeclDeviceLogin {
    pub id: Uuid,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GeclDevicePoll {
    Pending { interval: u64 },
    Complete { user: Credentials },
    Cancelled,
}

#[derive(Deserialize)]
struct DeviceResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

async fn checked_json(
    response: reqwest::Response,
    step: &str,
) -> crate::Result<serde_json::Value> {
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!(
            "{step}失败（HTTP {}）。请检查账户权限、网络及应用授权",
            status.as_u16()
        );
    }
    Ok(response.json().await?)
}

pub async fn begin_gecl_device_login() -> crate::Result<GeclDeviceLogin> {
    let response = CLIENT
        .post(format!("{OAUTH_BASE}/devicecode"))
        .query(&[("mkt", "zh-CN")])
        .form(&[("client_id", MICROSOFT_CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await?;
    let body: DeviceResponse = serde_json::from_value(
        checked_json(response, "获取微软登录代码").await?,
    )?;
    let uri = Url::parse(&body.verification_uri)?;
    if uri.scheme() != "https"
        || !matches!(
            uri.host_str(),
            Some(
                "microsoft.com"
                    | "www.microsoft.com"
                    | "login.microsoftonline.com"
            )
        )
    {
        anyhow::bail!("微软返回了无法识别的登录地址");
    }
    let id = Uuid::new_v4();
    let interval = body.interval.max(5);
    let now = Instant::now();
    *DEVICE_LOGIN.lock().await = Some(DeviceSession {
        id,
        code: body.device_code,
        interval,
        expires: now + std::time::Duration::from_secs(body.expires_in),
        next_poll: now + std::time::Duration::from_secs(interval),
    });
    Ok(GeclDeviceLogin {
        id,
        user_code: body.user_code,
        verification_uri: body.verification_uri,
        expires_in: body.expires_in,
        interval,
    })
}

pub async fn cancel_gecl_device_login(id: Uuid) {
    let mut session = DEVICE_LOGIN.lock().await;
    if session.as_ref().is_some_and(|s| s.id == id) {
        *session = None;
    }
}

pub async fn poll_gecl_device_login(
    id: Uuid,
    pool: &sqlx::SqlitePool,
) -> crate::Result<GeclDevicePoll> {
    let code = {
        let mut guard = DEVICE_LOGIN.lock().await;
        let Some(session) = guard.as_mut().filter(|s| s.id == id) else {
            return Ok(GeclDevicePoll::Cancelled);
        };
        let now = Instant::now();
        if now >= session.expires {
            *guard = None;
            anyhow::bail!("登录代码已过期，请重新登录");
        }
        if now < session.next_poll {
            return Ok(GeclDevicePoll::Pending {
                interval: session.interval,
            });
        }
        session.next_poll =
            now + std::time::Duration::from_secs(session.interval);
        session.code.clone()
    };
    let response = CLIENT
        .post(format!("{OAUTH_BASE}/token"))
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", &code),
        ])
        .send()
        .await?;
    let status = response.status();
    let body: serde_json::Value = response.json().await?;
    if !status.is_success() {
        let mut guard = DEVICE_LOGIN.lock().await;
        let Some(session) = guard.as_mut().filter(|s| s.id == id) else {
            return Ok(GeclDevicePoll::Cancelled);
        };
        match body["error"].as_str() {
            Some("authorization_pending") => {
                return Ok(GeclDevicePoll::Pending {
                    interval: session.interval,
                });
            }
            Some("slow_down") => {
                session.interval += 5;
                session.next_poll = Instant::now()
                    + std::time::Duration::from_secs(session.interval);
                return Ok(GeclDevicePoll::Pending {
                    interval: session.interval,
                });
            }
            Some("authorization_declined" | "access_denied") => {
                *guard = None;
                anyhow::bail!("已取消微软授权");
            }
            Some("expired_token") => {
                *guard = None;
                anyhow::bail!("登录代码已过期，请重新登录");
            }
            _ => {
                *guard = None;
                anyhow::bail!(
                    "微软授权失败（HTTP {}），请检查公共客户端流设置",
                    status.as_u16()
                );
            }
        }
    }
    let token: OAuthToken = serde_json::from_value(body)?;
    let user = exchange_minecraft_session(token).await?;
    let mut guard = DEVICE_LOGIN.lock().await;
    if !guard.as_ref().is_some_and(|s| s.id == id) {
        return Ok(GeclDevicePoll::Cancelled);
    }
    user.upsert(pool).await?;
    *guard = None;
    Ok(GeclDevicePoll::Complete { user })
}

async fn exchange_minecraft_session(
    token: OAuthToken,
) -> crate::Result<Credentials> {
    let xbox = checked_json(CLIENT.post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json!({"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":format!("d={}", token.access_token)},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}))
        .send().await?, "Xbox 登录").await?;
    let xbox_token = xbox["Token"].as_str().context("Xbox 未返回令牌")?;
    let xsts = checked_json(CLIENT.post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({"Properties":{"SandboxId":"RETAIL","UserTokens":[xbox_token]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}))
        .send().await?, "Xbox 账户授权").await?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .context("Xbox 未返回用户标识")?;
    let xsts_token = xsts["Token"].as_str().context("Xbox 未返回会话令牌")?;
    let minecraft = checked_json(CLIENT.post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({"identityToken":format!("XBL3.0 x={uhs};{xsts_token}")}))
        .send().await?, "Minecraft 登录").await?;
    let access_token = minecraft["access_token"]
        .as_str()
        .context("Minecraft 未返回令牌")?
        .to_owned();
    let expires_in = minecraft["expires_in"]
        .as_i64()
        .context("Minecraft 未返回有效期")?;
    let entitlements = checked_json(
        CLIENT
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&access_token)
            .send()
            .await?,
        "Minecraft 拥有权检查",
    )
    .await?;
    if !entitlements["items"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        anyhow::bail!(
            "此微软账户未返回 Minecraft Java 版拥有权，请确认购买状态或订阅"
        );
    }
    let profile: MinecraftProfile = serde_json::from_value(
        checked_json(
            CLIENT
                .get("https://api.minecraftservices.com/minecraft/profile")
                .bearer_auth(&access_token)
                .send()
                .await?,
            "获取 Minecraft 玩家资料",
        )
        .await?,
    )?;
    Ok(Credentials {
        account_type: AccountType::Microsoft,
        offline_profile: profile,
        access_token,
        refresh_token: token.refresh_token,
        expires: Utc::now() + Duration::seconds(expires_in),
        active: true,
    })
}

pub(super) async fn refresh_credentials(
    credentials: &mut Credentials,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<()> {
    let response = CLIENT
        .post(format!("{OAUTH_BASE}/token"))
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &credentials.refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|source| MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::RefreshOAuthToken,
        })?;
    let status = response.status();
    let body: serde_json::Value = response.json().await?;
    if !status.is_success() {
        if body["error"].as_str() == Some("invalid_grant") {
            return Err(SessionExpired.into());
        }
        anyhow::bail!(
            "刷新微软登录失败（HTTP {}），请重新登录",
            status.as_u16()
        );
    }
    let token: OAuthToken = serde_json::from_value(body)?;
    let mut refreshed = exchange_minecraft_session(token).await?;
    refreshed.active = credentials.active;
    refreshed.upsert(exec).await?;
    *credentials = refreshed;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("微软登录已失效，请重新登录")]
pub(super) struct SessionExpired;
