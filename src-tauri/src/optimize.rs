use serde::Serialize;
use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::instances::InstalledMod;
use crate::modrinth;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeModInfo {
    pub slug: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizePack {
    pub loader: String,
    pub mods: Vec<OptimizeModInfo>,
}

/// Моды-оптимизаторы для Fabric (актуальны для новых версий, включая 1.21.x / 26.x).
const FABRIC_MODS: &[(&str, &str, &str)] = &[
    ("sodium", "Sodium", "Мощный рендер-движок: +FPS в 2-5 раз (аналог OptiFine)"),
    ("lithium", "Lithium", "Оптимизация игровой логики: мобы, физика, тики"),
    ("ferrite-core", "FerriteCore", "Снижает потребление оперативной памяти"),
    ("entityculling", "EntityCulling", "Не рендерит невидимые сущности за стенами"),
    ("immediatelyfast", "ImmediatelyFast", "Ускоряет отрисовку интерфейса и частиц"),
    ("iris", "Iris Shaders", "Поддержка шейдеров (совместим с Sodium)"),
];

/// Моды-оптимизаторы для Forge (версии подбираются автоматически под версию игры).
const FORGE_MODS: &[(&str, &str, &str)] = &[
    ("embeddium", "Embeddium", "Порт Sodium на Forge: главный буст FPS"),
    ("modernfix", "ModernFix", "Ускоряет запуск и снижает потребление памяти"),
    ("ferrite-core", "FerriteCore", "Снижает потребление оперативной памяти"),
    ("saturn", "Saturn", "Оптимизация использования памяти"),
    ("oculus", "Oculus", "Поддержка шейдеров на Forge (порт Iris)"),
];

/// NeoForge — современный наследник Forge, сюда переехали оптимизаторы для новых версий.
const NEOFORGE_MODS: &[(&str, &str, &str)] = &[
    ("sodium", "Sodium", "Мощный рендер-движок: +FPS в 2-5 раз (есть и под NeoForge)"),
    ("embeddium", "Embeddium", "Альтернативный рендер-движок (порт Sodium)"),
    ("lithium", "Lithium", "Оптимизация игровой логики: мобы, физика, тики"),
    ("modernfix", "ModernFix", "Ускоряет запуск и снижает потребление памяти"),
    ("ferrite-core", "FerriteCore", "Снижает потребление оперативной памяти"),
    ("iris", "Iris Shaders", "Поддержка шейдеров (совместим с Sodium)"),
];

pub fn pack_info(loader: &str) -> OptimizePack {
    let mods = match loader {
        "fabric" => FABRIC_MODS,
        "forge" => FORGE_MODS,
        "neoforge" => NEOFORGE_MODS,
        _ => &[],
    };
    OptimizePack {
        loader: loader.to_string(),
        mods: mods
            .iter()
            .map(|(slug, title, desc)| OptimizeModInfo {
                slug: slug.to_string(),
                title: title.to_string(),
                description: desc.to_string(),
            })
            .collect(),
    }
}

/// Ставит пак оптимизации в сборку. Пропускает моды, у которых нет версии под игру.
/// Возвращает список установленных и пропущенных.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeResult {
    pub installed: Vec<InstalledMod>,
    pub skipped: Vec<String>,
}

pub async fn install_pack(
    app: &AppHandle,
    state: &AppState,
    instance_id: &str,
    with_shaders: bool,
) -> Result<OptimizeResult> {
    let inst = crate::instances::get(state, instance_id)?;
    if inst.loader == "vanilla" {
        return Err(Error::msg("Оптимизация модами доступна для Fabric, Forge и NeoForge сборок"));
    }
    let mods: &[(&str, &str, &str)] = match inst.loader.as_str() {
        "fabric" => FABRIC_MODS,
        "neoforge" => NEOFORGE_MODS,
        _ => FORGE_MODS,
    };
    let mut installed = Vec::new();
    let mut skipped = Vec::new();
    for (slug, title, _) in mods {
        // шейдеры — по желанию
        if !with_shaders && (*slug == "iris" || *slug == "oculus") {
            continue;
        }
        // Sodium и Embeddium — альтернативы, ставим только первый доступный
        if *slug == "embeddium" && installed.iter().any(|m: &InstalledMod| m.file_name.to_lowercase().contains("sodium")) {
            continue;
        }
        // узнаём project_id по slug
        let project: serde_json::Value = match state
            .http
            .get(format!("https://api.modrinth.com/v2/project/{slug}"))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => match r.json().await {
                Ok(v) => v,
                Err(_) => { skipped.push(title.to_string()); continue; }
            },
            _ => { skipped.push(title.to_string()); continue; }
        };
        let Some(project_id) = project.get("id").and_then(|i| i.as_str()) else {
            skipped.push(title.to_string());
            continue;
        };
        match modrinth::install_mod(app, state, instance_id, project_id, &inst.game_version, &inst.loader).await {
            Ok(list) => installed.extend(list),
            Err(_) => skipped.push(title.to_string()),
        }
    }
    Ok(OptimizeResult { installed, skipped })
}