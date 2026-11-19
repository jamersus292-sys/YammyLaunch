use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::state::AppState;

/// Путь к javaw.exe для нужной major-версии Java; при отсутствии — скачивает Adoptium JRE.
pub async fn ensure_java(
    app: &AppHandle,
    state: &AppState,
    major: u32,
    override_path: Option<&str>,
) -> Result<PathBuf> {
    if let Some(p) = override_path {
        if !p.is_empty() {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
            return Err(Error::msg(format!("Указанный путь к Java не существует: {p}")));
        }
    }
    let root = state.root()?;
    let runtime_dir = root.join("runtimes").join(format!("java-{major}"));
    if let Some(existing) = find_javaw(&runtime_dir) {
        return Ok(existing);
    }
    std::fs::create_dir_all(&runtime_dir)?;
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/x64/jre/hotspot/normal/eclipse"
    );
    let zip_path = runtime_dir.join("jre.zip");
    download_all(
        app,
        &state.http,
        &format!("java-{major}"),
        &format!("Скачивание Java {major}"),
        vec![DownloadTask {
            url,
            dest: zip_path.clone(),
            sha1: None,
            size: None,
        }],
    )
    .await?;
    let zip_path2 = zip_path.clone();
    let runtime_dir2 = runtime_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::open(&zip_path2)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(&runtime_dir2)?;
        Ok(())
    })
    .await
    .map_err(|e| Error::msg(e.to_string()))??;
    std::fs::remove_file(&zip_path).ok();
    find_javaw(&runtime_dir)
        .ok_or_else(|| Error::msg("В архиве JRE не найден bin/javaw.exe"))
}

fn find_javaw(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("bin").join("javaw.exe");
    if direct.exists() {
        return Some(direct);
    }
    // Adoptium zip распаковывается в подпапку jdk-XX.../
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path().join("bin").join("javaw.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}