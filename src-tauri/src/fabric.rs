use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::mojang::{self, VersionJson};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricGameVersion {
    pub version: String,
    pub stable: bool,
}

/// Список версий игры, поддерживаемых Fabric (кэшируется в памяти процесса не нужен — дёшево).
pub async fn supported_game_versions(state: &AppState) -> Result<Vec<FabricGameVersion>> {
    let list: Vec<FabricGameVersion> = state
        .http
        .get("https://meta.fabricmc.net/v2/versions/game")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(list)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderEntry {
    pub loader: FabricLoaderInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderInfo {
    pub version: String,
    pub stable: bool,
}

/// Список доступных loader-версий для версии игры (новые сверху).
pub async fn loader_versions(state: &AppState, game_version: &str) -> Result<Vec<FabricLoaderEntry>> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}");
    let resp = state.http.get(&url).send().await?;
    if resp.status().as_u16() == 404 || resp.status().as_u16() == 400 {
        return Err(Error::msg(format!("Fabric не поддерживает версию {game_version}")));
    }
    let list: Vec<FabricLoaderEntry> = resp.error_for_status()?.json().await?;
    if list.is_empty() {
        return Err(Error::msg(format!("Fabric не поддерживает версию {game_version}")));
    }
    Ok(list)
}

/// Скачивает profile JSON лоадера и сохраняет как versions/{id}/{id}.json.
pub async fn install_profile(state: &AppState, game_version: &str, loader_version: &str) -> Result<VersionJson> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/profile/json"
    );
    let v: VersionJson = state.http.get(&url).send().await?.error_for_status()?.json().await?;
    let root = state.root()?;
    let path = mojang::version_json_path(&root, &v.id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&v)?)?;
    Ok(v)
}