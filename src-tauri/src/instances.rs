use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Instance {
    pub id: String,
    pub name: String,
    /// id ванильной версии, напр. "1.21.1"
    pub game_version: String,
    /// "vanilla" | "fabric" | "forge"
    pub loader: String,
    /// версия лоадера (fabric loader / forge build)
    pub loader_version: Option<String>,
    /// версия OptiFine (например "HD_U_I5"), если установлен
    pub optifine_version: Option<String>,
    /// id итогового version JSON (fabric-loader-... / forge-...)
    pub version_id: String,
    pub memory_max_mb: Option<u32>,
    pub memory_min_mb: Option<u32>,
    pub java_path: Option<String>,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub icon: Option<String>,
    pub created_at: String,
    pub last_played: Option<String>,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().simple().to_string(),
            name: String::new(),
            game_version: String::new(),
            loader: "vanilla".into(),
            loader_version: None,
            optifine_version: None,
            version_id: String::new(),
            memory_max_mb: None,
            memory_min_mb: None,
            java_path: None,
            resolution_width: None,
            resolution_height: None,
            icon: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: None,
        }
    }
}

fn instances_path(state: &AppState) -> Result<PathBuf> {
    Ok(state.root()?.join("instances.json"))
}

pub fn instances_dir(state: &AppState) -> Result<PathBuf> {
    let dir = state.root()?.join("instances");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn instance_dir(state: &AppState, id: &str) -> Result<PathBuf> {
    Ok(instances_dir(state)?.join(id))
}

pub fn load_all(state: &AppState) -> Result<Vec<Instance>> {
    let path = instances_path(state)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let s = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&s).unwrap_or_default())
}

pub fn save_all(state: &AppState, list: &[Instance]) -> Result<()> {
    std::fs::write(instances_path(state)?, serde_json::to_string_pretty(list)?)?;
    Ok(())
}

pub fn get(state: &AppState, id: &str) -> Result<Instance> {
    load_all(state)?
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| Error::msg("Инстанс не найден"))
}

pub fn upsert(state: &AppState, inst: Instance) -> Result<()> {
    let mut list = load_all(state)?;
    if let Some(existing) = list.iter_mut().find(|i| i.id == inst.id) {
        *existing = inst;
    } else {
        list.push(inst);
    }
    save_all(state, &list)
}

pub fn create_dirs(state: &AppState, inst: &Instance) -> Result<PathBuf> {
    let dir = instance_dir(state, &inst.id)?;
    for sub in ["mods", "saves", "config", "resourcepacks", "shaderpacks"] {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    Ok(dir)
}

pub fn delete(state: &AppState, id: &str) -> Result<()> {
    let mut list = load_all(state)?;
    list.retain(|i| i.id != id);
    save_all(state, &list)?;
    let dir = instance_dir(state, id)?;
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

pub fn touch_last_played(state: &AppState, id: &str) -> Result<()> {
    let mut list = load_all(state)?;
    if let Some(inst) = list.iter_mut().find(|i| i.id == id) {
        inst.last_played = Some(chrono::Utc::now().to_rfc3339());
    }
    save_all(state, &list)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub file_name: String,
    pub size: u64,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

/// Метаданные установленных модов храним в mods/.YammyLaunch.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModsMeta {
    #[serde(default)]
    pub mods: Vec<InstalledMod>,
}

pub fn load_mods_meta(dir: &std::path::Path) -> ModsMeta {
    std::fs::read_to_string(dir.join("mods").join(".YammyLaunch.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_mods_meta(dir: &std::path::Path, meta: &ModsMeta) -> Result<()> {
    std::fs::write(
        dir.join("mods").join(".YammyLaunch.json"),
        serde_json::to_string_pretty(meta)?,
    )?;
    Ok(())
}