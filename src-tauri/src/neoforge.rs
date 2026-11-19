use std::path::Path;
use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::forge;
use crate::state::AppState;

const MAVEN_META: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

/// NeoForge-версия для версии игры: 1.21.11 -> 21.11.x, 1.20.4 -> 20.4.x, 26.3 -> 26.3.x
fn version_prefix(game_version: &str) -> Option<String> {
    let parts: Vec<&str> = game_version.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    // год-версии (26.x) уже совпадают с префиксом NeoForge
    if parts[0].parse::<u32>().ok()? >= 26 {
        return Some(format!("{}.{}", parts[0], parts[1]));
    }
    // классика 1.X.Y -> X.Y
    if parts[0] == "1" {
        Some(format!("{}.{}", parts[1], parts.get(2).copied().unwrap_or("0")))
    } else {
        None
    }
}

/// Последняя стабильная NeoForge-версия для игры.
pub async fn recommended_version(state: &AppState, game_version: &str) -> Result<String> {
    let prefix = version_prefix(game_version)
        .ok_or_else(|| Error::msg(format!("NeoForge не поддерживает версию {game_version}")))?;
    let xml = state.http.get(MAVEN_META).send().await?.error_for_status()?.text().await?;
    let mut best: Option<String> = None;
    for line in xml.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("<version>").and_then(|s| s.strip_suffix("</version>")) {
            if v.starts_with(&format!("{prefix}.")) && !v.contains("beta") && !v.contains("alpha") {
                // сравниваем по числовым компонентам
                let better = match &best {
                    None => true,
                    Some(b) => cmp_versions(v, b),
                };
                if better {
                    best = Some(v.to_string());
                }
            }
        }
    }
    best.ok_or_else(|| Error::msg(format!("NeoForge не найден для версии {game_version}")))
}

fn cmp_versions(a: &str, b: &str) -> bool {
    let pa: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let pb: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    pa > pb
}

/// Установка NeoForge тем же инсталлятором, что и Forge (они совместимы).
pub async fn install(
    app: &AppHandle,
    state: &AppState,
    instance_dir: &Path,
    game_version: &str,
    neoforge_version: &str,
    java_major: u32,
) -> Result<String> {
    let url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
    );
    forge::install_from_url(app, state, instance_dir, game_version, neoforge_version, java_major, &url, "neoforge").await
}