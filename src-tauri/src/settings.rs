use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Error, Result};

/// Client_id приложения PrismLauncher (прошёл ревью Mojang).
/// Старый дефолт 00000000402b5328 Microsoft удалил (AADSTS700016).
pub const DEFAULT_MSA_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";
const DEAD_CLIENT_IDS: &[&str] = &["00000000402b5328"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub java_path_override: Option<String>,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub fullscreen: bool,
    pub show_snapshots: bool,
    pub close_launcher_on_game_start: bool,
    pub msa_client_id: String,
    pub data_dir: Option<String>,
    pub extra_jvm_args: String,
    /// Свой ключ ИИ-сервиса (пусто = встроенный)
    pub ai_api_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory_max_mb: 4096,
            memory_min_mb: 512,
            java_path_override: None,
            resolution_width: 1280,
            resolution_height: 720,
            fullscreen: false,
            show_snapshots: false,
            close_launcher_on_game_start: false,
            msa_client_id: DEFAULT_MSA_CLIENT_ID.into(),
            data_dir: None,
            extra_jvm_args: String::new(),
            ai_api_key: None,
        }
    }
}

pub fn root_dir(settings: &Settings) -> Result<PathBuf> {
    if let Some(d) = &settings.data_dir {
        if !d.is_empty() {
            return Ok(PathBuf::from(d));
        }
    }
    // Игра весит много (ассеты ~1 ГБ на версию) — если есть диск D, храним данные там.
    let base = if PathBuf::from("D:\\").exists() {
        PathBuf::from("D:\\")
    } else {
        dirs::config_dir().ok_or_else(|| Error::msg("Не найден %APPDATA%"))?
    };
    let new_dir = base.join("YammyLaunch");
    // миграция данных со старых названий
    for old in ["CraftLaunch", "YammyLanc"] {
        let old_dir = base.join(old);
        if old_dir.exists() && !new_dir.exists() {
            std::fs::rename(&old_dir, &new_dir).ok();
        }
    }
    Ok(new_dir)
}

pub fn load(settings_path: &PathBuf) -> Settings {
    let mut s: Settings = std::fs::read_to_string(settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if DEAD_CLIENT_IDS.contains(&s.msa_client_id.as_str()) || s.msa_client_id.is_empty() {
        s.msa_client_id = DEFAULT_MSA_CLIENT_ID.into();
    }
    s
}

pub fn save(path: &PathBuf, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}