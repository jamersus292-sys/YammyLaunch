use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::instances::{self, InstalledMod, ModsMeta};
use crate::state::AppState;

const API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
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
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_hits: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub dependencies: Vec<ModDependency>,
    pub files: Vec<ModFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: String,
}

pub async fn search(
    state: &AppState,
    query: &str,
    game_version: &str,
    loader: &str,
    offset: u64,
) -> Result<SearchResponse> {
    let mut facets = vec![vec!["project_type:mod".to_string()]];
    if !game_version.is_empty() {
        facets.push(vec![format!("versions:{game_version}")]);
    }
    if loader == "fabric" || loader == "forge" || loader == "neoforge" {
        facets.push(vec![format!("categories:{loader}")]);
    }
    let facets_json = serde_json::to_string(&facets)?;
    let resp: SearchResponse = state
        .http
        .get(format!("{API}/search"))
        .query(&[
            ("query", query),
            ("facets", &facets_json),
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

pub async fn project_versions(
    state: &AppState,
    project_id: &str,
    game_version: &str,
    loader: &str,
) -> Result<Vec<ModVersion>> {
    let gv = serde_json::to_string(&[game_version])?;
    let mut req = state
        .http
        .get(format!("{API}/project/{project_id}/version"))
        .query(&[("game_versions", gv.as_str())]);
    if loader == "fabric" || loader == "forge" || loader == "neoforge" {
        let ld = serde_json::to_string(&[loader])?;
        req = req.query(&[("loaders", ld.as_str())]);
    }
    let list: Vec<ModVersion> = req.send().await?.error_for_status()?.json().await?;
    Ok(list)
}

async fn version_by_id(state: &AppState, version_id: &str) -> Result<ModVersion> {
    Ok(state
        .http
        .get(format!("{API}/version/{version_id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Устанавливает мод (последнюю подходящую версию) + обязательные зависимости рекурсивно.
pub async fn install_mod(
    app: &AppHandle,
    state: &AppState,
    instance_id: &str,
    project_id: &str,
    game_version: &str,
    loader: &str,
) -> Result<Vec<InstalledMod>> {
    let dir = instances::instance_dir(state, instance_id)?;
    let mut installed = Vec::new();
    let mut visited = std::collections::HashSet::new();
    install_recursive(app, state, &dir, project_id, game_version, loader, &mut visited, &mut installed, 0).await?;
    Ok(installed)
}

async fn install_recursive(
    app: &AppHandle,
    state: &AppState,
    dir: &Path,
    project_id: &str,
    game_version: &str,
    loader: &str,
    visited: &mut std::collections::HashSet<String>,
    installed: &mut Vec<InstalledMod>,
    depth: usize,
) -> Result<()> {
    if depth > 6 || !visited.insert(project_id.to_string()) {
        return Ok(());
    }
    let versions = project_versions(state, project_id, game_version, loader).await?;
    let version = versions
        .first()
        .ok_or_else(|| Error::msg(format!("Нет версии мода для {game_version} ({loader})")))?;
    install_version_files(app, state, dir, version, installed).await?;
    for dep in &version.dependencies {
        if dep.dependency_type != "required" {
            continue;
        }
        if let Some(dep_project) = &dep.project_id {
            Box::pin(install_recursive(app, state, dir, dep_project, game_version, loader, visited, installed, depth + 1)).await?;
        } else if let Some(dep_version) = &dep.version_id {
            let v = version_by_id(state, dep_version).await?;
            if visited.insert(v.project_id.clone()) {
                install_version_files(app, state, dir, &v, installed).await?;
            }
        }
    }
    Ok(())
}

async fn install_version_files(
    app: &AppHandle,
    state: &AppState,
    dir: &Path,
    version: &ModVersion,
    installed: &mut Vec<InstalledMod>,
) -> Result<()> {
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| Error::msg("У версии мода нет файлов"))?;
    let dest = dir.join("mods").join(&file.filename);
    download_all(
        app,
        &state.http,
        &format!("mod-{}", version.project_id),
        &format!("Мод: {}", version.name),
        vec![DownloadTask {
            url: file.url.clone(),
            dest: dest.clone(),
            sha1: file.hashes.sha1.clone(),
            size: Some(file.size),
        }],
    )
    .await?;
    let mut meta = instances::load_mods_meta(dir);
    meta.mods.retain(|m| m.file_name != file.filename);
    let entry = InstalledMod {
        file_name: file.filename.clone(),
        size: file.size,
        project_id: Some(version.project_id.clone()),
        version_id: Some(version.id.clone()),
        title: Some(version.name.clone()),
        icon_url: None,
    };
    meta.mods.push(entry.clone());
    instances::save_mods_meta(dir, &meta)?;
    installed.push(entry);
    Ok(())
}

pub fn list_mods(state: &AppState, instance_id: &str) -> Result<Vec<InstalledMod>> {
    let dir = instances::instance_dir(state, instance_id)?;
    let meta = instances::load_mods_meta(&dir);
    let mods_dir = dir.join("mods");
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") || name.ends_with(".jar.disabled") {
                let known = meta.mods.iter().find(|m| m.file_name == name || format!("{}.disabled", m.file_name) == name);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                result.push(InstalledMod {
                    file_name: name,
                    size,
                    project_id: known.and_then(|k| k.project_id.clone()),
                    version_id: known.and_then(|k| k.version_id.clone()),
                    title: known.and_then(|k| k.title.clone()),
                    icon_url: known.and_then(|k| k.icon_url.clone()),
                });
            }
        }
    }
    result.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(result)
}

pub fn remove_mod(state: &AppState, instance_id: &str, file_name: &str) -> Result<()> {
    let dir = instances::instance_dir(state, instance_id)?;
    let path = dir.join("mods").join(file_name);
    // Защита от выхода за пределы mods/
    if path.parent().map(|p| p.file_name().map(|n| n == "mods").unwrap_or(false)).unwrap_or(false) {
        std::fs::remove_file(&path).ok();
    }
    let mut meta = instances::load_mods_meta(&dir);
    meta.mods.retain(|m| m.file_name != file_name);
    instances::save_mods_meta(&dir, &meta)?;
    Ok(())
}

pub fn toggle_mod(state: &AppState, instance_id: &str, file_name: &str) -> Result<String> {
    let dir = instances::instance_dir(state, instance_id)?;
    let mods = dir.join("mods");
    let old = mods.join(file_name);
    let new_name = if file_name.ends_with(".disabled") {
        file_name.trim_end_matches(".disabled").to_string()
    } else {
        format!("{file_name}.disabled")
    };
    std::fs::rename(&old, mods.join(&new_name))?;
    Ok(new_name)
}