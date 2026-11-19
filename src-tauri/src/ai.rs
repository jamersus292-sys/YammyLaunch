use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::state::AppState;

const AI_BASE: &str = "https://api.reformboss.com/v1";
const AI_MODEL: &str = "deepseek-v4.1-flash";

// Встроенный ключ хранится в обфусцированном виде (XOR + base64),
// чтобы его нельзя было вытащить из репозитория/бинарника поиском по строке "sk-".
// Это НЕ криптографическая защита: для приватного ключа используйте своё значение в настройках.
const KEY_BLOB: &str = "KgpADBp4UUANB1AcBwhXBR9fR1NcAV0ZFQECQQMQO1ZfWEt1WRRbU1lPBlUGBBlZFgIL";
const KEY_MASK: &[u8] = b"YammyLaunch-2026-orange-secret";

fn builtin_key() -> Option<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(KEY_BLOB).ok()?;
    let unmasked: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ KEY_MASK[i % KEY_MASK.len()])
        .collect();
    String::from_utf8(unmasked).ok()
}

fn api_key(state: &AppState) -> Result<String> {
    let s = state.settings();
    if let Some(custom) = s.ai_api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        return Ok(custom.trim().to_string());
    }
    builtin_key().ok_or_else(|| Error::msg("ИИ-ключ не найден. Введите свой ключ в настройках."))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

const SYSTEM_PROMPT: &str = "Ты — Ямми, милый и весёлый ИИ-помощник лаунчера Minecraft «YammyLaunch». \
Отвечай по-русски, кратко и по делу, с лёгкой игривостью (можно 1-2 эмодзи 🍊✨). \
Ты помогаешь с: настройкой Minecraft и лаунчера, выбором версий и модов (Fabric/Forge/NeoForge/OptiFine), \
решением ошибок и крашей, оптимизацией FPS, установкой шейдеров и текстур. \
Если пользователь прислал лог ошибки — найди причину и дай конкретные шаги исправления. \
Не выдумывай факты о версиях; если не уверен — скажи честно.";

pub async fn chat(state: &AppState, history: Vec<ChatMessage>) -> Result<String> {
    let key = api_key(state)?;
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: SYSTEM_PROMPT.into(),
    }];
    // последние 20 сообщений, чтобы не раздувать контекст
    let start = history.len().saturating_sub(20);
    messages.extend(history[start..].iter().cloned());

    let resp = state
        .http
        .post(format!("{AI_BASE}/chat/completions"))
        .bearer_auth(&key)
        .json(&serde_json::json!({
            "model": AI_MODEL,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 1500
        }))
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::msg(format!("ИИ-сервис ответил ошибкой {status}: {}", &body[..body.len().min(300)])));
    }
    let parsed: ChatResponse = resp.json().await?;
    parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| Error::msg("ИИ не вернул ответ"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtin_key_decodes() {
        let k = super::builtin_key().expect("key must decode");
        assert!(k.starts_with("sk-"));
        assert_eq!(k.len(), 51);
    }
}