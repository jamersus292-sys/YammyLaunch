use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::state::AppState;

const KEYRING_SERVICE: &str = "YammyLaunch";
const KEYRING_OLD_SERVICES: &[&str] = &["CraftLaunch", "YammyLanc"];

fn keyring_entry(id: &str) -> Result<keyring::Entry> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, id)?;
    if entry.get_password().is_err() {
        // миграция токенов со старых названий сервиса
        for old_name in KEYRING_OLD_SERVICES {
            if let Ok(old) = keyring::Entry::new(old_name, id) {
                if let Ok(secret) = old.get_password() {
                    entry.set_password(&secret)?;
                    old.delete_credential().ok();
                    break;
                }
            }
        }
    }
    Ok(entry)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    /// "offline" | "msa"
    pub kind: String,
    pub username: String,
    /// UUID без дефисов
    pub uuid: String,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountsFile {
    pub accounts: Vec<Account>,
    pub active_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MsaSecret {
    refresh_token: String,
    mc_token: Option<String>,
    /// unix timestamp
    mc_expires_at: Option<i64>,
}

pub fn offline_uuid(name: &str) -> String {
    let digest = Md5::digest(format!("OfflinePlayer:{name}").as_bytes());
    let mut bytes: [u8; 16] = digest.into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC4122 variant
    Uuid::from_bytes(bytes).simple().to_string()
}

fn accounts_path(state: &AppState) -> Result<PathBuf> {
    Ok(state.root()?.join("accounts.json"))
}

pub fn load_accounts(state: &AppState) -> Result<AccountsFile> {
    let path = accounts_path(state)?;
    if !path.exists() {
        return Ok(AccountsFile::default());
    }
    let s = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&s).unwrap_or_default())
}

pub fn save_accounts(state: &AppState, file: &AccountsFile) -> Result<()> {
    let path = accounts_path(state)?;
    std::fs::write(path, serde_json::to_string_pretty(file)?)?;
    Ok(())
}

pub fn add_offline_account(state: &AppState, username: &str) -> Result<Account> {
    let username = username.trim();
    if username.len() < 3 || username.len() > 16 {
        return Err(Error::msg("Ник должен быть от 3 до 16 символов"));
    }
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::msg("Ник может содержать только латинские буквы, цифры и _"));
    }
    let mut file = load_accounts(state)?;
    if let Some(existing) = file.accounts.iter().find(|a| a.kind == "offline" && a.username.eq_ignore_ascii_case(username)) {
        file.active_id = Some(existing.id.clone());
        save_accounts(state, &file)?;
        return Ok(existing.clone());
    }
    let account = Account {
        id: Uuid::new_v4().simple().to_string(),
        kind: "offline".into(),
        username: username.to_string(),
        uuid: offline_uuid(username),
        skin_url: None,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    file.accounts.push(account.clone());
    file.active_id = Some(account.id.clone());
    save_accounts(state, &file)?;
    Ok(account)
}

pub fn remove_account(state: &AppState, id: &str) -> Result<()> {
    let mut file = load_accounts(state)?;
    file.accounts.retain(|a| a.id != id);
    if file.active_id.as_deref() == Some(id) {
        file.active_id = file.accounts.first().map(|a| a.id.clone());
    }
    save_accounts(state, &file)?;
    keyring::Entry::new(KEYRING_SERVICE, id).and_then(|e| e.delete_credential()).ok();
    Ok(())
}

pub fn set_active(state: &AppState, id: &str) -> Result<()> {
    let mut file = load_accounts(state)?;
    if !file.accounts.iter().any(|a| a.id == id) {
        return Err(Error::msg("Аккаунт не найден"));
    }
    file.active_id = Some(id.to_string());
    save_accounts(state, &file)
}

pub fn active_account(state: &AppState) -> Result<Account> {
    let file = load_accounts(state)?;
    let id = file.active_id.ok_or_else(|| Error::msg("Нет активного аккаунта. Добавьте аккаунт на странице «Аккаунты»."))?;
    file.accounts
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| Error::msg("Активный аккаунт не найден"))
}

// ---------- Microsoft device code flow ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeInfo {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblClaims,
}

#[derive(Debug, Deserialize)]
struct XblClaims {
    xui: Vec<XblUhs>,
}

#[derive(Debug, Deserialize)]
struct XblUhs {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct McLoginResponse {
    access_token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct McProfile {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<McSkin>,
}

#[derive(Debug, Deserialize)]
struct McSkin {
    state: String,
    url: String,
}

fn token_url(client_id: &str) -> String {
    let _ = client_id;
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".to_string()
}

pub async fn start_device_flow(app: &AppHandle, state: &AppState) -> Result<String> {
    let settings = state.settings();
    let client_id = settings.msa_client_id.clone();
    let session_id = Uuid::new_v4().simple().to_string();

    let resp = state
        .http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::msg(format!("Microsoft отклонил запрос кода ({status}): {body}")));
    }
    let dc: DeviceCodeResponse = resp.json().await?;

    let _ = app.emit(
        "auth://device-code",
        DeviceCodeInfo {
            session_id: session_id.clone(),
            user_code: dc.user_code.clone(),
            verification_uri: dc.verification_uri.clone(),
            expires_in: dc.expires_in,
            interval: dc.interval,
        },
    );

    let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
    state.auth_cancel.lock().unwrap().insert(session_id.clone(), cancel_tx);

    let app2 = app.clone();
    let http = state.http.clone();
    let sid = session_id.clone();
    let device_code = dc.device_code.clone();
    let mut interval = dc.interval.max(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(dc.expires_in);
    let state_handle = app.clone();

    tokio::spawn(async move {
        let result = poll_and_login(&http, &client_id, &device_code, &mut interval, deadline, &mut cancel_rx).await;
        match result {
            Ok(account) => {
                // сохраняем аккаунт в список и делаем активным
                use tauri::Manager;
                if let Some(state) = state_handle.try_state::<AppState>() {
                    let mut file = load_accounts(&state).unwrap_or_default();
                    // не дублируем один и тот же профиль
                    file.accounts.retain(|a| !(a.kind == "msa" && a.uuid == account.uuid));
                    file.accounts.push(account.clone());
                    file.active_id = Some(account.id.clone());
                    let _ = save_accounts(&state, &file);
                }
                let _ = app2.emit("auth://success", &account);
            }
            Err(e) => {
                let _ = app2.emit("auth://error", serde_json::json!({ "sessionId": sid, "error": e.to_string() }));
            }
        }
    });

    Ok(session_id)
}

pub fn cancel_device_flow(state: &AppState, session_id: &str) {
    if let Some(tx) = state.auth_cancel.lock().unwrap().remove(session_id) {
        let _ = tx.send(true);
    }
}

async fn poll_and_login(
    http: &reqwest::Client,
    client_id: &str,
    device_code: &str,
    interval: &mut u64,
    deadline: std::time::Instant,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<Account> {
    loop {
        if *cancel.borrow() {
            return Err(Error::msg("Вход отменён"));
        }
        if std::time::Instant::now() > deadline {
            return Err(Error::msg("Время ожидания входа истекло. Попробуйте снова."));
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(*interval)) => {}
            _ = cancel.changed() => { return Err(Error::msg("Вход отменён")); }
        }
        let resp = http
            .post(token_url(client_id))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        if status.is_success() {
            let tr: TokenResponse = serde_json::from_value(body)?;
            let refresh = tr
                .refresh_token
                .ok_or_else(|| Error::msg("Microsoft не выдал refresh_token"))?;
            return complete_login(http, &tr.access_token, &refresh).await;
        }
        let err = body.get("error").and_then(|e| e.as_str()).unwrap_or("");
        match err {
            "authorization_pending" => continue,
            "slow_down" => {
                *interval += 5;
                continue;
            }
            "authorization_declined" => return Err(Error::msg("Вход отклонён пользователем")),
            "expired_token" => return Err(Error::msg("Код истёк. Запросите новый.")),
            "bad_verification_code" => return Err(Error::msg("Неверный код устройства")),
            _ => {
                let desc = body.get("error_description").and_then(|d| d.as_str()).unwrap_or("неизвестная ошибка");
                return Err(Error::msg(format!("Ошибка Microsoft: {desc}")));
            }
        }
    }
}

/// Полная цепочка: MSA access_token → XBL → XSTS → MC token → профиль.
async fn complete_login(http: &reqwest::Client, ms_token: &str, refresh_token: &str) -> Result<Account> {
    let (mc_token, mc_expires, profile) = msa_to_mc(http, ms_token).await?;
    let account = Account {
        id: Uuid::new_v4().simple().to_string(),
        kind: "msa".into(),
        username: profile.name,
        uuid: profile.id,
        skin_url: profile
            .skins
            .iter()
            .find(|s| s.state == "ACTIVE")
            .map(|s| s.url.clone()),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    let secret = MsaSecret {
        refresh_token: refresh_token.to_string(),
        mc_token: Some(mc_token),
        mc_expires_at: Some(mc_expires),
    };
    let entry = keyring_entry(&account.id)?;
    entry.set_password(&serde_json::to_string(&secret)?)?;
    Ok(account)
}

async fn msa_to_mc(http: &reqwest::Client, ms_token: &str) -> Result<(String, i64, McProfile)> {
    // 1. Xbox Live
    let xbl: XblResponse = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("x-xbl-contract-version", "1")
        .json(&serde_json::json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_token}") },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::msg(format!("Xbox Live отклонил вход: {e}")))?
        .json()
        .await?;
    let uhs = xbl
        .display_claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or_else(|| Error::msg("Xbox Live не вернул uhs"))?;

    // 2. XSTS
    let xsts_resp = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("x-xbl-contract-version", "1")
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?;
    if !xsts_resp.status().is_success() {
        let body: serde_json::Value = xsts_resp.json().await.unwrap_or_default();
        let xerr = body.get("XErr").and_then(|x| x.as_u64()).unwrap_or(0);
        let msg = match xerr {
            2148916233 => "У этого аккаунта Microsoft нет профиля Xbox. Создайте его на xbox.com и попробуйте снова.",
            2148916235 => "Xbox Live недоступен в вашей стране/регионе.",
            2148916238 => "Это детский аккаунт. Он должен быть добавлен в семью Microsoft, либо используйте взрослый аккаунт.",
            2148916227 => "Аккаунт Xbox заблокирован.",
            _ => "Не удалось получить XSTS-токен Xbox.",
        };
        return Err(Error::msg(msg));
    }
    let xsts: XblResponse = xsts_resp.json().await?;

    // 3. Minecraft login
    let mc: McLoginResponse = http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::msg(format!("Minecraftservices отклонил вход: {e}")))?
        .json()
        .await?;
    let expires_at = chrono::Utc::now().timestamp() + mc.expires_in;

    // 4. Лицензия
    let ent: serde_json::Value = http
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&mc.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let owns = ent
        .get("items")
        .and_then(|i| i.as_array())
        .map(|items| {
            items.iter().any(|it| {
                let name = it.get("name").and_then(|n| n.as_str()).unwrap_or("");
                name == "product_minecraft" || name == "game_minecraft" || name.contains("game_pass")
            })
        })
        .unwrap_or(false);
    if !owns {
        return Err(Error::msg("На этом аккаунте нет лицензии Minecraft Java Edition."));
    }

    // 5. Профиль
    let profile_resp = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc.access_token)
        .send()
        .await?;
    if profile_resp.status().as_u16() == 404 {
        return Err(Error::msg(
            "Лицензия есть, но профиль Minecraft не создан. Запустите один раз официальный лаунчер и создайте профиль.",
        ));
    }
    let profile: McProfile = profile_resp.error_for_status()?.json().await?;
    Ok((mc.access_token, expires_at, profile))
}

/// Возвращает действующий MC access_token для аккаунта (обновляет цепочку при необходимости).
pub async fn ensure_mc_token(state: &AppState, account: &Account) -> Result<String> {
    let entry = keyring_entry(&account.id)?;
    let secret_str = entry
        .get_password()
        .map_err(|_| Error::msg("Токены аккаунта не найдены. Войдите заново."))?;
    let mut secret: MsaSecret = serde_json::from_str(&secret_str)?;
    let now = chrono::Utc::now().timestamp();
    if let (Some(token), Some(exp)) = (&secret.mc_token, secret.mc_expires_at) {
        if exp > now + 120 {
            return Ok(token.clone());
        }
    }
    let settings = state.settings();
    let resp = state
        .http
        .post(token_url(&settings.msa_client_id))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", settings.msa_client_id.as_str()),
            ("refresh_token", secret.refresh_token.as_str()),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(Error::msg("Сессия Microsoft истекла. Войдите в аккаунт заново."));
    }
    let tr: TokenResponse = resp.json().await?;
    let (mc_token, expires_at, profile) = msa_to_mc(&state.http, &tr.access_token).await?;
    secret.refresh_token = tr.refresh_token.unwrap_or(secret.refresh_token);
    secret.mc_token = Some(mc_token.clone());
    secret.mc_expires_at = Some(expires_at);
    entry.set_password(&serde_json::to_string(&secret)?)?;
    // Обновим ник/скин, если изменились
    let mut file = load_accounts(state)?;
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account.id) {
        a.username = profile.name;
        a.uuid = profile.id;
        a.skin_url = profile.skins.iter().find(|s| s.state == "ACTIVE").map(|s| s.url.clone());
    }
    save_accounts(state, &file)?;
    Ok(mc_token)
}