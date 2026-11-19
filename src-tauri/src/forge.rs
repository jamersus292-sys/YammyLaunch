use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::java;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePromos {
    pub promos: HashMap<String, String>,
}

/// Рекомендуемая версия Forge для версии игры (или latest).
pub async fn recommended_version(state: &AppState, game_version: &str) -> Result<String> {
    let promos: ForgePromos = state
        .http
        .get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    promos
        .promos
        .get(&format!("{game_version}-recommended"))
        .or_else(|| promos.promos.get(&format!("{game_version}-latest")))
        .cloned()
        .ok_or_else(|| Error::msg(format!("Forge не поддерживает версию {game_version}")))
}

/// Запускает официальный Forge-инсталлятор headless в папку инстанса.
/// Возвращает id созданной версии (например "1.20.1-forge-47.4.28").
pub async fn install(
    app: &AppHandle,
    state: &AppState,
    instance_dir: &Path,
    game_version: &str,
    forge_version: &str,
    java_major: u32,
) -> Result<String> {
    let url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}/forge-{game_version}-{forge_version}-installer.jar"
    );
    install_from_url(app, state, instance_dir, game_version, forge_version, java_major, &url, "forge").await
}

/// Общий механизм для Forge и NeoForge (инсталляторы совместимы).
pub async fn install_from_url(
    app: &AppHandle,
    state: &AppState,
    instance_dir: &Path,
    game_version: &str,
    loader_version: &str,
    java_major: u32,
    url: &str,
    kind: &str,
) -> Result<String> {
    std::fs::create_dir_all(instance_dir)?;
    // Инсталлятор требует наличия launcher_profiles.json
    let profiles = instance_dir.join("launcher_profiles.json");
    if !profiles.exists() {
        std::fs::write(&profiles, r#"{"profiles":{}}"#)?;
    }
    let installer_path = instance_dir.join(format!("{kind}-installer.jar"));
    download_all(
        app,
        &state.http,
        &format!("{kind}-{game_version}"),
        &format!("Скачивание {}", if kind == "neoforge" { "NeoForge" } else { "Forge" }),
        vec![DownloadTask {
            url: url.to_string(),
            dest: installer_path.clone(),
            sha1: None,
            size: None,
        }],
    )
    .await?;

    let java_path = java::ensure_java(app, state, java_major, None).await?;
    let _ = app.emit("launch://log", serde_json::json!({
        "instanceId": "", "line": format!("[{kind}] Запуск инсталлятора {game_version}-{loader_version}..."), "stream": "launcher"
    }));

    let mut cmd = tokio::process::Command::new(&java_path);
    cmd.arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .current_dir(instance_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| Error::msg(format!("Не удалось запустить {kind}-инсталлятор: {e}")))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let app2 = app.clone();
    let log_task = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app2.emit("forge://log", line);
            }
        }
        if let Some(err) = stderr {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app2.emit("forge://log", line);
            }
        }
    });

    let status = child.wait().await?;
    let _ = log_task.await;
    std::fs::remove_file(&installer_path).ok();
    if !status.success() {
        return Err(Error::msg(format!("{kind}-инсталлятор завершился с ошибкой. Подробности в логе.")));
    }

    // Инсталлятор кладёт версию в <instance>/versions/<id>/<id>.json — переносим в общий стор.
    let versions_dir = instance_dir.join("versions");
    let mut found: Option<(String, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            if id.contains("forge") {
                let json = dir.join(format!("{id}.json"));
                if json.exists() {
                    found = Some((id, json));
                    break;
                }
            }
        }
    }
    let (id, json_path) = found.ok_or_else(|| Error::msg(format!("{kind}-инсталлятор не создал версию")))?;
    let root = state.root()?;
    let dest_dir = root.join("versions").join(&id);
    std::fs::create_dir_all(&dest_dir)?;
    std::fs::copy(&json_path, dest_dir.join(format!("{id}.json")))?;
    // Библиотеки инсталлятора могли лечь в instance/libraries — переносим в общий стор.
    let inst_libs = instance_dir.join("libraries");
    if inst_libs.exists() {
        merge_dirs(&inst_libs, &root.join("libraries"))?;
        std::fs::remove_dir_all(&inst_libs).ok();
    }
    std::fs::remove_dir_all(&versions_dir).ok();
    Ok(id)
}

fn merge_dirs(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            merge_dirs(&from, &to)?;
        } else if !to.exists() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}