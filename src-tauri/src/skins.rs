use serde::Serialize;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::state::AppState;

fn skins_dir(state: &AppState) -> Result<PathBuf> {
    let dir = state.root()?.join("skins");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn skin_path(state: &AppState, account_id: &str) -> Result<PathBuf> {
    Ok(skins_dir(state)?.join(format!("{account_id}.png")))
}

pub fn cape_path(state: &AppState, account_id: &str) -> Result<PathBuf> {
    Ok(skins_dir(state)?.join(format!("{account_id}-cape.png")))
}

/// Валидация PNG скина: 64x64 или 64x32. Плащ: 64x32.
fn validate_png(bytes: &[u8], allowed: &[(u32, u32)]) -> Result<()> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(Error::msg("Файл не является PNG"));
    }
    // IHDR: ширина/высота в байтах 16..24
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if !allowed.contains(&(w, h)) {
        let dims: Vec<String> = allowed.iter().map(|(a, b)| format!("{a}x{b}")).collect();
        return Err(Error::msg(format!(
            "Неверный размер скина: {w}x{h}. Нужен {}",
            dims.join(" или ")
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinInfo {
    pub has_skin: bool,
    pub has_cape: bool,
    pub skin_model: String,
}

pub fn set_skin(state: &AppState, account_id: &str, bytes: &[u8], model: &str) -> Result<SkinInfo> {
    validate_png(bytes, &[(64, 64), (64, 32)])?;
    let path = skin_path(state, account_id)?;
    std::fs::write(&path, bytes)?;
    let mut file = crate::auth::load_accounts(state)?;
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account_id) {
        a.skin_model = Some(if model == "slim" { "slim".into() } else { "classic".into() });
    }
    crate::auth::save_accounts(state, &file)?;
    Ok(info(state, account_id))
}

pub fn set_cape(state: &AppState, account_id: &str, bytes: &[u8]) -> Result<SkinInfo> {
    validate_png(bytes, &[(64, 32), (64, 64)])?;
    let path = cape_path(state, account_id)?;
    std::fs::write(&path, bytes)?;
    let mut file = crate::auth::load_accounts(state)?;
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account_id) {
        a.has_cape = true;
    }
    crate::auth::save_accounts(state, &file)?;
    Ok(info(state, account_id))
}

pub fn remove_skin(state: &AppState, account_id: &str) -> Result<SkinInfo> {
    std::fs::remove_file(skin_path(state, account_id)?).ok();
    let mut file = crate::auth::load_accounts(state)?;
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account_id) {
        a.skin_model = None;
    }
    crate::auth::save_accounts(state, &file)?;
    Ok(info(state, account_id))
}

pub fn remove_cape(state: &AppState, account_id: &str) -> Result<SkinInfo> {
    std::fs::remove_file(cape_path(state, account_id)?).ok();
    let mut file = crate::auth::load_accounts(state)?;
    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account_id) {
        a.has_cape = false;
    }
    crate::auth::save_accounts(state, &file)?;
    Ok(info(state, account_id))
}

pub fn info(state: &AppState, account_id: &str) -> SkinInfo {
    let has_skin = skin_path(state, account_id).map(|p| p.exists()).unwrap_or(false);
    let has_cape = cape_path(state, account_id).map(|p| p.exists()).unwrap_or(false);
    let model = crate::auth::load_accounts(state)
        .ok()
        .and_then(|f| f.accounts.into_iter().find(|a| a.id == account_id))
        .and_then(|a| a.skin_model)
        .unwrap_or_else(|| "classic".into());
    SkinInfo { has_skin, has_cape, skin_model: model }
}

/// data-URL превью скина для UI
pub fn skin_preview(state: &AppState, account_id: &str) -> Result<Option<String>> {
    use base64::Engine;
    let path = skin_path(state, account_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    Ok(Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    )))
}

pub fn cape_preview(state: &AppState, account_id: &str) -> Result<Option<String>> {
    use base64::Engine;
    let path = cape_path(state, account_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    Ok(Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    )))
}

// ---------- локальный скин-сервер для authlib-injector ----------

use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static SERVER_PORT: OnceLock<u16> = OnceLock::new();
static SIGNING_KEY: OnceLock<rsa::RsaPrivateKey> = OnceLock::new();

/// Генерирует (или загружает) RSA-ключ для подписи текстур.
fn signing_key(root: &std::path::Path) -> &'static rsa::RsaPrivateKey {
    SIGNING_KEY.get_or_init(|| {
        use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
        let key_path = root.join("skins").join("signing-key.pem");
        if let Ok(pem) = std::fs::read_to_string(&key_path) {
            if let Ok(k) = rsa::RsaPrivateKey::from_pkcs8_pem(&pem) {
                return k;
            }
        }
        let mut rng = getrandom::SysRng;
        let key = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("RSA keygen");
        if let Ok(pem) = key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF) {
            let _ = std::fs::create_dir_all(root.join("skins"));
            let _ = std::fs::write(&key_path, pem.as_bytes());
        }
        key
    })
}

fn public_key_pem(root: &std::path::Path) -> String {
    use rsa::pkcs8::EncodePublicKey;
    let pub_key = signing_key(root).to_public_key();
    pub_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap_or_default()
}

fn sign_textures(root: &std::path::Path, value_b64: &str) -> Option<String> {
    use base64::Engine;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    let key = signing_key(root);
    let signing = SigningKey::<sha2::Sha256>::new(key.clone());
    let sig = signing.sign(value_b64.as_bytes());
    Some(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()))
}

/// Запускает локальный HTTP-сервер, отдающий скины/плащи по протоколу authlib-injector.
/// Возвращает базовый URL (http://127.0.0.1:PORT).
pub async fn ensure_server(state: &AppState) -> Result<String> {
    if let Some(port) = SERVER_PORT.get() {
        return Ok(format!("http://127.0.0.1:{port}"));
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let _ = SERVER_PORT.set(port);
    let root = state.root()?;
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { break };
            let root = root.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let Ok(n) = sock.read(&mut buf).await else { return };
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, ctype, body) = handle_request(&root, &path);
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.write_all(&body).await;
            });
        }
    });
    Ok(format!("http://127.0.0.1:{port}"))
}

fn handle_request(root: &std::path::Path, path: &str) -> (&'static str, &'static str, Vec<u8>) {
    let skins = root.join("skins");
    // /textures/skin/<accountId> и /textures/cape/<accountId>
    if let Some(id) = path.strip_prefix("/textures/skin/") {
        let p = skins.join(format!("{id}.png"));
        if let Ok(bytes) = std::fs::read(&p) {
            return ("200 OK", "image/png", bytes);
        }
        return ("404 Not Found", "text/plain", b"no skin".to_vec());
    }
    if let Some(id) = path.strip_prefix("/textures/cape/") {
        let p = skins.join(format!("{id}-cape.png"));
        if let Ok(bytes) = std::fs::read(&p) {
            return ("200 OK", "image/png", bytes);
        }
        return ("404 Not Found", "text/plain", b"no cape".to_vec());
    }
    // /sessionserver/session/minecraft/profile/<uuid>?unsigned=false
    if path.contains("/session/minecraft/profile/") {
        let uuid = path
            .split("/profile/")
            .nth(1)
            .and_then(|s| s.split('?').next())
            .unwrap_or("");
        if let Ok(Some(body)) = profile_json(root, uuid) {
            return ("200 OK", "application/json", body.into_bytes());
        }
        return ("404 Not Found", "text/plain", b"no profile".to_vec());
    }
    // корень — metadata для authlib-injector
    if path == "/" || path.is_empty() {
        let pubkey = public_key_pem(root);
        let meta = serde_json::json!({
            "meta": {
                "serverName": "YammyLaunch",
                "implementationName": "yammy-skin-server",
                "implementationVersion": "1.0.0"
            },
            "skinDomains": ["127.0.0.1"],
            "signaturePublickey": pubkey
        });
        return ("200 OK", "application/json", meta.to_string().into_bytes());
    }
    ("404 Not Found", "text/plain", b"not found".to_vec())
}

fn profile_json(root: &std::path::Path, uuid: &str) -> Result<Option<String>> {
    use base64::Engine;
    let accounts = crate::auth::load_accounts_by_root(root)?;
    let Some(acc) = accounts.accounts.iter().find(|a| a.uuid == uuid) else {
        return Ok(None);
    };
    let skins = root.join("skins");
    let has_skin = skins.join(format!("{}.png", acc.id)).exists();
    let has_cape = skins.join(format!("{}-cape.png", acc.id)).exists();
    if !has_skin && !has_cape {
        return Ok(None);
    }
    let mut textures = serde_json::Map::new();
    if has_skin {
        let mut skin = serde_json::Map::new();
        skin.insert("url".into(), serde_json::json!(format!("http://127.0.0.1:{}/textures/skin/{}", SERVER_PORT.get().copied().unwrap_or(0), acc.id)));
        if acc.skin_model.as_deref() == Some("slim") {
            skin.insert("metadata".into(), serde_json::json!({"model": "slim"}));
        }
        textures.insert("SKIN".into(), serde_json::Value::Object(skin));
    }
    if has_cape {
        textures.insert("CAPE".into(), serde_json::json!({
            "url": format!("http://127.0.0.1:{}/textures/cape/{}", SERVER_PORT.get().copied().unwrap_or(0), acc.id)
        }));
    }
    let tex_value = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "profileId": acc.uuid,
        "profileName": acc.username,
        "textures": textures
    });
    let encoded = base64::engine::general_purpose::STANDARD.encode(tex_value.to_string());
    let signature = sign_textures(root, &encoded);
    let mut prop = serde_json::json!({ "name": "textures", "value": encoded });
    if let Some(sig) = signature {
        prop["signature"] = serde_json::json!(sig);
    }
    let profile = serde_json::json!({
        "id": acc.uuid,
        "name": acc.username,
        "properties": [prop]
    });
    Ok(Some(profile.to_string()))
}

/// Скачивает authlib-injector.jar (один раз).
pub async fn ensure_authlib_injector(state: &AppState) -> Result<PathBuf> {
    let root = state.root()?;
    let path = root.join("authlib-injector.jar");
    if path.exists() {
        return Ok(path);
    }
    let url = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";
    let bytes = state.http.get(url).send().await?.error_for_status()?.bytes().await?;
    std::fs::write(&path, &bytes)?;
    Ok(path)
}