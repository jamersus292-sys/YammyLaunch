use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use tauri::AppHandle;

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::mojang::{self, Arguments, Library, VersionJson};
use crate::state::AppState;

const BMCL_VERSION_LIST: &str = "https://bmclapi2.bangbang93.com/optifine/versionList";
const BMCL_DOWNLOAD: &str = "https://bmclapi2.bangbang93.com/optifine";
/// OptiFabric (поддерживаемый форк) — нужен для OptiFine на Fabric
pub const OPTIFABRIC_PROJECT_ID: &str = "bHUKuhiA";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptifineEntry {
    pub mcversion: String,
    pub patch: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub filename: String,
    #[serde(default)]
    pub forge: Option<String>,
}

impl OptifineEntry {
    pub fn label(&self) -> String {
        format!("{}_{}", self.kind, self.patch)
    }
}

/// Числовой ранг патча для сортировки: буква (A=1..Z=26) * 1000 + номер,
/// pre-версии ниже стабильных того же патча. Пример: I6=9000, I6_pre6=8994, I5=9000-1000.
fn patch_rank(e: &OptifineEntry) -> i64 {
    let p = &e.patch;
    let (base, pre) = match p.split_once("_pre") {
        Some((b, n)) => (b, n.parse::<i64>().unwrap_or(0)),
        None => (p.as_str(), 0),
    };
    let mut chars = base.chars();
    let letter = chars.next().unwrap_or('A');
    let num: i64 = chars.as_str().parse().unwrap_or(0);
    let letter_val = (letter.to_ascii_uppercase() as i64) - ('A' as i64) + 1;
    let mut rank = letter_val * 1000 + num * 10;
    if pre > 0 {
        rank -= 10 - pre.min(9); // pre1..pre9 ниже стабильного, pre9 ближе всех
    }
    rank
}

/// Список OptiFine для версии игры, новые сверху.
pub async fn versions_for(state: &AppState, game_version: &str) -> Result<Vec<OptifineEntry>> {
    let list: Vec<OptifineEntry> = state
        .http
        .get(BMCL_VERSION_LIST)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut filtered: Vec<OptifineEntry> = list
        .into_iter()
        .filter(|e| e.mcversion == game_version)
        .collect();
    // Сортируем: новые патчи сверху (I6 > I5 > G7), стабильные раньше pre-версий.
    filtered.sort_by(|a, b| patch_rank(b).cmp(&patch_rank(a)));
    if filtered.is_empty() {
        return Err(Error::msg(format!("OptiFine не найден для версии {game_version}")));
    }
    Ok(filtered)
}

/// Скачивает OptiFine jar, извлекает патч и создаёт version JSON (наследник ванильной версии).
/// Возвращает id созданной версии.
pub async fn install(
    app: &AppHandle,
    state: &AppState,
    game_version: &str,
    entry: &OptifineEntry,
    instance_dir: &Path,
    mode: &str, // "vanilla" | "fabric" | "forge"
) -> Result<String> {
    let root = state.root()?;
    let url = format!("{BMCL_DOWNLOAD}/{}/{}/{}", entry.mcversion, entry.kind, entry.patch);
    let jar_dest = root.join("optifine").join(&entry.filename);
    download_all(
        app,
        &state.http,
        &format!("optifine-{game_version}"),
        "Скачивание OptiFine",
        vec![DownloadTask {
            url,
            dest: jar_dest.clone(),
            sha1: None,
            size: None,
        }],
    )
    .await?;

    // Forge (1.13+): OptiFine — обычный мод, кладём jar в mods/, version JSON не нужен.
    if mode == "forge" {
        let mods_dir = instance_dir.join("mods");
        std::fs::create_dir_all(&mods_dir)?;
        std::fs::copy(&jar_dest, mods_dir.join(&entry.filename))?;
        let mut meta = crate::instances::load_mods_meta(instance_dir);
        meta.mods.retain(|m| m.file_name != entry.filename);
        meta.mods.push(crate::instances::InstalledMod {
            file_name: entry.filename.clone(),
            size: std::fs::metadata(&jar_dest).map(|m| m.len()).unwrap_or(0),
            project_id: None,
            version_id: None,
            title: Some(format!("OptiFine {}", entry.label())),
            icon_url: None,
        });
        crate::instances::save_mods_meta(instance_dir, &meta)?;
        return Ok(String::new());
    }

    // Извлекаем OptiFinePatch из jar (лежит в корне архива)
    let patch_bytes = {
        let file = std::fs::File::open(&jar_dest)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut patch_file = archive
            .by_name("OptiFinePatch")
            .map_err(|_| Error::msg("В jar OptiFine не найден OptiFinePatch"))?;
        let mut buf = Vec::new();
        patch_file.read_to_end(&mut buf)?;
        buf
    };
    let patch_str = String::from_utf8(patch_bytes)
        .map_err(|_| Error::msg("OptiFinePatch не является текстом"))?;
    let patch: serde_json::Value = serde_json::from_str(&patch_str)
        .map_err(|e| Error::msg(format!("Не удалось разобрать OptiFinePatch: {e}")))?;

    // Копируем сам OptiFine jar как библиотеку
    let of_lib_rel = format!("optifine/OptiFine/{game_version}_{}/OptiFine-{game_version}_{}.jar", entry.label(), entry.label());
    let of_lib_dest = root.join("libraries").join(&of_lib_rel);
    if let Some(parent) = of_lib_dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&jar_dest, &of_lib_dest)?;

    // launchwrapper для старых версий (<= 1.12.2)
    let mut libraries: Vec<Library> = Vec::new();
    let needs_wrapper = patch.get("mainClass").and_then(|m| m.as_str()) == Some("net.minecraft.launchwrapper.Launch");
    if needs_wrapper {
        libraries.push(Library {
            name: "net.minecraft:launchwrapper:1.12".into(),
            downloads: None,
            rules: vec![],
            extract: None,
            url: Some("https://libraries.minecraft.net/".into()),
            natives: None,
        });
    }
    libraries.push(Library {
        name: format!("optifine:OptiFine:{game_version}_{}", entry.label()),
        downloads: None,
        rules: vec![],
        extract: None,
        url: None,
        natives: None,
    });

    let mut arguments = Arguments::default();
    if let Some(tweaks) = patch.get("--tweakClass").and_then(|t| t.as_str()) {
        arguments.game.push(mojang::Argument::Plain("--tweakClass".into()));
        arguments.game.push(mojang::Argument::Plain(tweaks.to_string()));
    }

    let with_fabric = mode == "fabric";
    let version_id = if with_fabric {
        format!("fabric-optifine-{game_version}-{}", entry.label())
    } else {
        format!("{game_version}-OptiFine_{}", entry.label())
    };
    let inherits_from = if with_fabric {
        // наследуемся от fabric-версии; её id найдём в versions/
        find_fabric_version_id(&root, game_version)?
    } else {
        game_version.to_string()
    };
    let main_class = patch
        .get("mainClass")
        .and_then(|m| m.as_str())
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();
    // Для Fabric mainClass остаётся KnotClient (OptiFabric сам подключает OptiFine)
    let main_class = if with_fabric {
        "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()
    } else {
        main_class
    };

    let v = VersionJson {
        id: version_id.clone(),
        kind: Some("release".into()),
        main_class,
        assets: None,
        asset_index: None,
        downloads: std::collections::HashMap::new(),
        libraries,
        arguments: Some(arguments),
        minecraft_arguments: None,
        java_version: None,
        logging: None,
        inherits_from: Some(inherits_from),
        jar: Some(game_version.to_string()),
    };
    let path = mojang::version_json_path(&root, &version_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&v)?)?;

    // Для Fabric кладём сам OptiFine в mods/ (OptiFabric подхватит его оттуда)
    if with_fabric {
        let mods_dir = instance_dir.join("mods");
        std::fs::create_dir_all(&mods_dir)?;
        std::fs::copy(&jar_dest, mods_dir.join(&entry.filename))?;
    }
    Ok(version_id)
}

fn find_fabric_version_id(root: &Path, game_version: &str) -> Result<String> {
    let versions_dir = root.join("versions");
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("fabric-loader-") && name.ends_with(&format!("-{game_version}")) {
                candidates.push(name);
            }
        }
    }
    candidates.sort();
    candidates
        .pop()
        .ok_or_else(|| Error::msg("Сначала создайте Fabric-версию для этой версии игры"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(patch: &str) -> OptifineEntry {
        OptifineEntry {
            mcversion: "1.20.1".into(),
            patch: patch.into(),
            kind: "HD_U".into(),
            filename: format!("OptiFine_1.20.1_HD_U_{patch}.jar"),
            forge: None,
        }
    }

    #[test]
    fn sort_newest_first() {
        let mut list = vec![
            entry("I5_pre4"), entry("I5"), entry("I6_pre1"), entry("I6"), entry("I6_pre6"), entry("G7"),
        ];
        list.sort_by(|a, b| patch_rank(b).cmp(&patch_rank(a)));
        let order: Vec<&str> = list.iter().map(|e| e.patch.as_str()).collect();
        assert_eq!(order, vec!["I6", "I6_pre6", "I6_pre1", "I5", "I5_pre4", "G7"]);
    }
}