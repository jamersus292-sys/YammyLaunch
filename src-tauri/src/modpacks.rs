use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tauri::AppHandle;

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::instances::{self, Instance};
use crate::modrinth::{self, ModVersion};
use crate::state::AppState;

const API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSearchResponse {
    pub hits: Vec<PackHit>,
    pub total_hits: u64,
}

pub async fn search(state: &AppState, query: &str, offset: u64) -> Result<PackSearchResponse> {
    let facets = serde_json::to_string(&[["project_type:modpack"]])?;
    let resp: PackSearchResponse = state
        .http
        .get(format!("{API}/search"))
        .query(&[
            ("query", query),
            ("facets", &facets),
            ("limit", "20"),
            ("offset", &offset.to_string()),
            ("index", if query.is_empty() { "downloads" } else { "relevance" }),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MrpackIndex {
    #[serde(rename = "game")]
    game: String,
    #[serde(rename = "versionId")]
    version_id: String,
    name: String,
    #[serde(default)]
    dependencies: MrpackDeps,
    files: Vec<MrpackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MrpackDeps {
    minecraft: String,
    #[serde(default, rename = "fabric-loader")]
    fabric_loader: Option<String>,
    #[serde(default)]
    forge: Option<String>,
    #[serde(default)]
    neoforge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MrpackFile {
    path: String,
    hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    env: Option<MrpackEnv>,
    downloads: Vec<String>,
    #[serde(default)]
    #[serde(rename = "fileSize")]
    file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MrpackEnv {
    client: String,
    server: String,
}

/// Устанавливает готовую сборку (modpack) с Modrinth. Возвращает созданный инстанс.
pub async fn install_pack(
    app: &AppHandle,
    state: &AppState,
    project_id: &str,
    name_override: Option<String>,
) -> Result<Instance> {
    // 1. Последняя версия проекта
    let versions: Vec<ModVersion> = state
        .http
        .get(format!("{API}/project/{project_id}/version"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let version = versions
        .first()
        .ok_or_else(|| Error::msg("У сборки нет доступных версий"))?;
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| Error::msg("У сборки нет файлов"))?;
    if !file.filename.ends_with(".mrpack") {
        return Err(Error::msg("Эта сборка не в формате .mrpack — установка не поддерживается"));
    }

    // 2. Скачиваем .mrpack
    let root = state.root()?;
    let pack_path = root.join("packs").join(&file.filename);
    download_all(
        app,
        &state.http,
        &format!("pack-{project_id}"),
        "Скачивание сборки",
        vec![DownloadTask {
            url: file.url.clone(),
            dest: pack_path.clone(),
            sha1: file.hashes.sha1.clone(),
            size: Some(file.size),
        }],
    )
    .await?;

    // 3. Читаем индекс
    let index: MrpackIndex = {
        let f = std::fs::File::open(&pack_path)?;
        let mut archive = zip::ZipArchive::new(f)?;
        let mut entry = archive
            .by_name("modrinth.index.json")
            .map_err(|_| Error::msg("В .mrpack нет modrinth.index.json"))?;
        serde_json::from_reader(&mut entry)?
    };
    let game_version = index.dependencies.minecraft.clone();
    let loader = if index.dependencies.fabric_loader.is_some() {
        "fabric"
    } else if index.dependencies.forge.is_some() {
        "forge"
    } else if index.dependencies.neoforge.is_some() {
        "neoforge"
    } else {
        "vanilla"
    };

    // 4. Создаём инстанс с нужным лоадером
    let mut inst = Instance {
        name: name_override.unwrap_or_else(|| index.name.clone()),
        game_version: game_version.clone(),
        loader: loader.to_string(),
        ..Default::default()
    };
    let dir = instances::create_dirs(state, &inst)?;
    match loader {
        "fabric" => {
            let lv = index.dependencies.fabric_loader.clone().unwrap();
            let profile = crate::fabric::install_profile(state, &game_version, &lv).await?;
            inst.loader_version = Some(lv);
            inst.version_id = profile.id;
        }
        "forge" => {
            let fv = index.dependencies.forge.clone().unwrap();
            let manifest = crate::mojang::fetch_manifest(state).await?;
            let mv = manifest
                .versions
                .iter()
                .find(|v| v.id == game_version)
                .ok_or_else(|| Error::msg("Версия игры не найдена"))?;
            let vj = crate::mojang::fetch_version_json(state, mv).await?;
            let vid = crate::forge::install(app, state, &dir, &game_version, &fv, vj.java_major()).await?;
            inst.loader_version = Some(fv);
            inst.version_id = vid;
        }
        "neoforge" => {
            let nv = index.dependencies.neoforge.clone().unwrap();
            let manifest = crate::mojang::fetch_manifest(state).await?;
            let mv = manifest
                .versions
                .iter()
                .find(|v| v.id == game_version)
                .ok_or_else(|| Error::msg("Версия игры не найдена"))?;
            let vj = crate::mojang::fetch_version_json(state, mv).await?;
            let vid = crate::neoforge::install(app, state, &dir, &game_version, &nv, vj.java_major()).await?;
            inst.loader_version = Some(nv);
            inst.version_id = vid;
        }
        _ => inst.version_id = game_version.clone(),
    }

    // 5. Скачиваем файлы сборки (моды, конфиги, ресурспаки)
    let mut tasks = Vec::new();
    for f in &index.files {
        if let Some(env) = &f.env {
            if env.client == "unsupported" {
                continue;
            }
        }
        // защита от выхода за пределы папки инстанса
        if f.path.contains("..") || f.path.starts_with('/') || f.path.contains(':') {
            continue;
        }
        let Some(url) = f.downloads.first() else { continue };
        tasks.push(DownloadTask {
            url: url.clone(),
            dest: dir.join(&f.path),
            sha1: f.hashes.get("sha1").cloned(),
            size: Some(f.file_size),
        });
    }
    download_all(app, &state.http, &format!("pack-{project_id}"), "Файлы сборки", tasks).await?;

    // 6. Overrides из архива
    {
        let f = std::fs::File::open(&pack_path)?;
        let mut archive = zip::ZipArchive::new(f)?;
        for i in 0..archive.len() {
            let Ok(mut entry) = archive.by_index(i) else { continue };
            let Some(enc) = entry.enclosed_name().map(|p| p.to_path_buf()) else { continue };
            let name = enc.to_string_lossy().replace('\\', "/");
            let rel = if let Some(r) = name.strip_prefix("overrides/") {
                r.to_string()
            } else if let Some(r) = name.strip_prefix("client-overrides/") {
                r.to_string()
            } else {
                continue;
            };
            if rel.is_empty() || entry.is_dir() {
                continue;
            }
            let out = dir.join(&rel);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    std::fs::remove_file(&pack_path).ok();

    // 7. Метаданные модов для списка установленного
    let mut meta = instances::load_mods_meta(&dir);
    let mods_dir = dir.join("mods");
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        let known: HashSet<String> = meta.mods.iter().map(|m| m.file_name.clone()).collect();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") && !known.contains(&name) {
                meta.mods.push(instances::InstalledMod {
                    file_name: name,
                    size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                    project_id: None,
                    version_id: None,
                    title: None,
                    icon_url: None,
                });
            }
        }
    }
    instances::save_mods_meta(&dir, &meta)?;

    instances::upsert(state, inst.clone())?;
    Ok(inst)
}

/// Список файлов сборки для отображения (без скачивания).
pub async fn pack_files(state: &AppState, project_id: &str) -> Result<Vec<String>> {
    let versions: Vec<ModVersion> = state
        .http
        .get(format!("{API}/project/{project_id}/version"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(versions
        .first()
        .map(|v| v.game_versions.clone())
        .unwrap_or_default())
}

#[allow(dead_code)]
fn unused(_: &Path) {}