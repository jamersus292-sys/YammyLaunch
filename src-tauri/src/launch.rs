use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::download::{download_all, DownloadTask};
use crate::error::{Error, Result};
use crate::mojang::{self, Argument, Library, VersionJson, RESOURCES_URL};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub instance_id: String,
    pub line: String,
    pub stream: String,
}

/// Собирает задачи загрузки для версии: client jar, библиотеки, ассеты, logging.
pub fn collect_downloads(root: &Path, v: &VersionJson) -> Result<Vec<DownloadTask>> {
    let mut tasks = Vec::new();
    let base_version_id = v.jar.clone().unwrap_or_else(|| v.id.clone());

    // client jar
    if let Some(client) = v.downloads.get("client") {
        tasks.push(DownloadTask {
            url: client.url.clone(),
            dest: root.join("versions").join(&base_version_id).join(format!("{base_version_id}.jar")),
            sha1: client.sha1.clone(),
            size: client.size,
        });
    }

    // библиотеки
    let libs_dir = root.join("libraries");
    for lib in &v.libraries {
        if !mojang::library_allowed(lib) {
            continue;
        }
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                if !artifact.url.is_empty() {
                    let rel = artifact
                        .path
                        .clone()
                        .unwrap_or_else(|| mojang::maven_path(&lib.name).unwrap_or_default());
                    if !rel.is_empty() {
                        tasks.push(DownloadTask {
                            url: artifact.url.clone(),
                            dest: libs_dir.join(&rel),
                            sha1: artifact.sha1.clone(),
                            size: artifact.size,
                        });
                    }
                }
            }
            // старый формат natives через classifiers
            if let Some(natives) = &lib.natives {
                if let Some(key) = natives.get("windows") {
                    let key = key.replace("${arch}", "64");
                    if let Some(native) = downloads.classifiers.get(&key) {
                        let rel = native
                            .path
                            .clone()
                            .unwrap_or_else(|| mojang::maven_path(&format!("{}:{}", lib.name, key)).unwrap_or_default());
                        tasks.push(DownloadTask {
                            url: native.url.clone(),
                            dest: libs_dir.join(&rel),
                            sha1: native.sha1.clone(),
                            size: native.size,
                        });
                    }
                }
            }
        } else if let Some(repo) = &lib.url {
            // Fabric/Forge: maven-координаты + репозиторий
            let rel = mojang::maven_path(&lib.name)?;
            let url = format!("{}/{}", repo.trim_end_matches('/'), rel);
            tasks.push(DownloadTask {
                url,
                dest: libs_dir.join(&rel),
                sha1: None,
                size: None,
            });
        } else if lib.name.contains("net.fabricmc") || lib.name.contains("org.ow2.asm") {
            let rel = mojang::maven_path(&lib.name)?;
            let repo = if lib.name.starts_with("net.fabricmc") {
                "https://maven.fabricmc.net"
            } else {
                "https://libraries.minecraft.net"
            };
            tasks.push(DownloadTask {
                url: format!("{repo}/{rel}"),
                dest: libs_dir.join(&rel),
                sha1: None,
                size: None,
            });
        }
    }

    // logging config
    if let Some(logging) = &v.logging {
        if let Some(client) = &logging.client {
            tasks.push(DownloadTask {
                url: client.file.url.clone(),
                dest: root.join("logging").join(&client.file.id),
                sha1: client.file.sha1.clone(),
                size: client.file.size,
            });
        }
    }

    // дедуп по пути
    let mut seen = std::collections::HashSet::new();
    tasks.retain(|t| seen.insert(t.dest.clone()));
    Ok(tasks)
}

/// Ассеты: индекс + объекты. Возвращает (tasks, index_id, virtual, map_to_resources).
pub async fn collect_assets(
    state: &AppState,
    root: &Path,
    v: &VersionJson,
) -> Result<(Vec<DownloadTask>, String, bool, bool)> {
    let Some(ai) = &v.asset_index else {
        return Ok((Vec::new(), String::new(), false, false));
    };
    let index_path = root.join("assets").join("indexes").join(format!("{}.json", ai.id));
    if !index_path.exists() {
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = state.http.get(&ai.url).send().await?.error_for_status()?.bytes().await?;
        std::fs::write(&index_path, &bytes)?;
    }
    let index: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;
    let virtual_legacy = index.get("virtual").and_then(|x| x.as_bool()).unwrap_or(false);
    let map_to_resources = index.get("map_to_resources").and_then(|x| x.as_bool()).unwrap_or(false);
    let mut tasks = Vec::new();
    if let Some(objects) = index.get("objects").and_then(|o| o.as_object()) {
        for (_name, obj) in objects {
            let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or("");
            let size = obj.get("size").and_then(|s| s.as_u64());
            if hash.len() < 2 {
                continue;
            }
            let prefix = &hash[0..2];
            tasks.push(DownloadTask {
                url: format!("{RESOURCES_URL}/{prefix}/{hash}"),
                dest: root.join("assets").join("objects").join(prefix).join(hash),
                sha1: Some(hash.to_string()),
                size,
            });
        }
    }
    Ok((tasks, ai.id.clone(), virtual_legacy, map_to_resources))
}

/// Полная подготовка версии: скачивание всего необходимого.
pub async fn prepare_version(app: &AppHandle, state: &AppState, version_id: &str) -> Result<VersionJson> {
    let root = state.root()?;
    let raw = mojang::load_version_json(&root, version_id)?;
    let v = mojang::resolve_inheritance(&root, raw)?;

    let tasks = collect_downloads(&root, &v)?;
    download_all(app, &state.http, version_id, "Файлы игры", tasks).await?;

    let (asset_tasks, index_id, virtual_legacy, map_to_resources) = collect_assets(state, &root, &v).await?;
    if !asset_tasks.is_empty() {
        download_all(app, &state.http, version_id, "Ассеты (звуки, текстуры)", asset_tasks).await?;
    }

    // legacy-раскладка ассетов
    if virtual_legacy && !index_id.is_empty() {
        rebuild_virtual_assets(&root, &index_id)?;
    }
    let _ = map_to_resources; // обрабатывается при запуске (копия в instance/resources)

    Ok(v)
}

fn rebuild_virtual_assets(root: &Path, index_id: &str) -> Result<()> {
    let index_path = root.join("assets").join("indexes").join(format!("{index_id}.json"));
    let index: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;
    let virtual_dir = root.join("assets").join("virtual").join(index_id);
    if let Some(objects) = index.get("objects").and_then(|o| o.as_object()) {
        for (name, obj) in objects {
            let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or("");
            if hash.len() < 2 {
                continue;
            }
            let src = root.join("assets").join("objects").join(&hash[0..2]).join(hash);
            let dst = virtual_dir.join(name);
            if !dst.exists() && src.exists() {
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dst)?;
            }
        }
    }
    Ok(())
}

fn copy_resources_assets(root: &Path, index_id: &str, instance_dir: &Path) -> Result<()> {
    let index_path = root.join("assets").join("indexes").join(format!("{index_id}.json"));
    let index: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;
    let res_dir = instance_dir.join("resources");
    if let Some(objects) = index.get("objects").and_then(|o| o.as_object()) {
        for (name, obj) in objects {
            let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or("");
            if hash.len() < 2 {
                continue;
            }
            let src = root.join("assets").join("objects").join(&hash[0..2]).join(hash);
            let dst = res_dir.join(name);
            if !dst.exists() && src.exists() {
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dst)?;
            }
        }
    }
    Ok(())
}

/// Распаковка natives в папку инстанса.
fn extract_natives(root: &Path, v: &VersionJson, natives_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(natives_dir)?;
    let libs_dir = root.join("libraries");
    for lib in &v.libraries {
        if !mojang::library_allowed(lib) {
            continue;
        }
        let Some(downloads) = &lib.downloads else { continue };
        let native = if let Some(natives) = &lib.natives {
            natives
                .get("windows")
                .map(|k| k.replace("${arch}", "64"))
                .and_then(|k| downloads.classifiers.get(&k).cloned())
        } else {
            None
        };
        // новый формат: отдельная запись :natives-windows
        let native = native.or_else(|| {
            if lib.name.ends_with(":natives-windows") {
                downloads.artifact.clone()
            } else {
                None
            }
        });
        let Some(native) = native else { continue };
        let rel = native
            .path
            .clone()
            .unwrap_or_else(|| mojang::maven_path(&lib.name).unwrap_or_default());
        let jar_path = libs_dir.join(&rel);
        if !jar_path.exists() {
            continue;
        }
        let excludes: Vec<String> = lib
            .extract
            .as_ref()
            .map(|e| e.exclude.clone())
            .unwrap_or_else(|| vec!["META-INF/".into()]);
        let file = std::fs::File::open(&jar_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let Ok(mut entry) = archive.by_index(i) else { continue };
            let Some(name) = entry.enclosed_name().map(|p| p.to_path_buf()) else { continue };
            let name_str = name.to_string_lossy().replace('\\', "/");
            if entry.is_dir() || excludes.iter().any(|ex| name_str.starts_with(ex.as_str())) {
                continue;
            }
            let out = natives_dir.join(&name);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    Ok(())
}

fn substitute(s: &str, map: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    for (k, val) in map {
        out = out.replace(&format!("${{{k}}}"), val);
    }
    out
}

fn arg_values(arg: &Argument, features: &HashMap<String, bool>, map: &HashMap<String, String>) -> Vec<String> {
    match arg {
        Argument::Plain(s) => vec![substitute(s, map)],
        Argument::Ruled { rules, value } => {
            if !mojang::rules_allow(rules, features) {
                return Vec::new();
            }
            match value {
                serde_json::Value::String(s) => vec![substitute(s, map)],
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| substitute(s, map))
                    .collect(),
                _ => Vec::new(),
            }
        }
    }
}

/// Classpath: все разрешённые artifact-библиотеки + client jar.
fn build_classpath(root: &Path, v: &VersionJson) -> Result<String> {
    let libs_dir = root.join("libraries");
    let mut paths: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for lib in &v.libraries {
        if !mojang::library_allowed(lib) {
            continue;
        }
        // natives-записи не идут в classpath (старый формат), новый формат — идут
        if lib.natives.is_some() {
            continue;
        }
        let rel = if let Some(d) = &lib.downloads {
            match &d.artifact {
                Some(a) if !a.url.is_empty() || a.path.is_some() => a
                    .path
                    .clone()
                    .unwrap_or_else(|| mojang::maven_path(&lib.name).unwrap_or_default()),
                _ => continue,
            }
        } else {
            mojang::maven_path(&lib.name)?
        };
        if rel.is_empty() || !seen.insert(rel.clone()) {
            continue;
        }
        let p = libs_dir.join(&rel);
        if p.exists() {
            paths.push(p.to_string_lossy().to_string());
        }
    }
    let base_id = v.jar.clone().unwrap_or_else(|| v.id.clone());
    let client_jar = root.join("versions").join(&base_id).join(format!("{base_id}.jar"));
    paths.push(client_jar.to_string_lossy().to_string());
    Ok(paths.join(";"))
}

pub struct LaunchParams {
    pub instance_id: String,
    pub instance_dir: PathBuf,
    pub version_id: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub xuid: String,
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub java_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub extra_jvm_args: String,
}

pub async fn launch_game(app: &AppHandle, state: &AppState, p: LaunchParams) -> Result<u32> {
    let root = state.root()?;
    let raw = mojang::load_version_json(&root, &p.version_id)?;
    let v = mojang::resolve_inheritance(&root, raw)?;

    let natives_dir = p.instance_dir.join("natives");
    extract_natives(&root, &v, &natives_dir)?;

    // map_to_resources для версий 1.6–1.7
    if let Some(ai) = &v.asset_index {
        let index_path = root.join("assets").join("indexes").join(format!("{}.json", ai.id));
        if let Ok(s) = std::fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<serde_json::Value>(&s) {
                if index.get("map_to_resources").and_then(|x| x.as_bool()).unwrap_or(false) {
                    copy_resources_assets(&root, &ai.id, &p.instance_dir)?;
                }
            }
        }
    }

    let classpath = build_classpath(&root, &v)?;
    let assets_root = root.join("assets");
    let assets_index = v.assets.clone().unwrap_or_else(|| "legacy".into());
    // virtual-ассеты для старых версий
    let assets_root_arg = {
        let virtual_dir = assets_root.join("virtual").join(&assets_index);
        if virtual_dir.exists() {
            virtual_dir.to_string_lossy().to_string()
        } else {
            assets_root.to_string_lossy().to_string()
        }
    };

    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("auth_player_name".into(), p.username.clone());
    map.insert("version_name".into(), v.id.clone());
    map.insert("game_directory".into(), p.instance_dir.to_string_lossy().to_string());
    map.insert("assets_root".into(), assets_root_arg);
    map.insert("game_assets".into(), assets_root.to_string_lossy().to_string());
    map.insert("assets_index_name".into(), assets_index);
    map.insert("auth_uuid".into(), p.uuid.clone());
    map.insert("auth_access_token".into(), p.access_token.clone());
    map.insert("auth_session".into(), format!("token:{}", p.access_token));
    map.insert("clientid".into(), "0".into());
    map.insert("auth_xuid".into(), p.xuid.clone());
    map.insert("user_type".into(), p.user_type.clone());
    map.insert("version_type".into(), v.kind.clone().unwrap_or_else(|| "release".into()));
    map.insert("natives_directory".into(), natives_dir.to_string_lossy().to_string());
    map.insert("launcher_name".into(), "YammyLaunch".into());
    map.insert("launcher_version".into(), "0.1.0".into());
    map.insert("classpath".into(), classpath.clone());
    map.insert("resolution_width".into(), p.width.to_string());
    map.insert("resolution_height".into(), p.height.to_string());
    map.insert("user_properties".into(), "{}".into());

    let mut features: HashMap<String, bool> = HashMap::new();
    features.insert("has_custom_resolution".into(), p.width > 0 && p.height > 0);
    features.insert("is_demo_user".into(), false);

    let mut jvm_args: Vec<String> = vec![
        format!("-Xmx{}M", p.memory_max_mb),
        format!("-Xms{}M", p.memory_min_mb),
    ];
    if let Some(args) = &v.arguments {
        for arg in &args.jvm {
            jvm_args.extend(arg_values(arg, &features, &map));
        }
    } else {
        // legacy-версии без arguments.jvm
        jvm_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        jvm_args.push("-Dminecraft.launcher.brand=YammyLaunch".into());
        jvm_args.push("-Dminecraft.launcher.version=0.1.0".into());
    }
    if let Some(logging) = &v.logging {
        if let Some(client) = &logging.client {
            let log_path = root.join("logging").join(&client.file.id);
            jvm_args.push(substitute(&client.argument, &{
                let mut m = map.clone();
                m.insert("path".into(), log_path.to_string_lossy().to_string());
                m
            }));
        }
    }
    for arg in p.extra_jvm_args.split_whitespace() {
        jvm_args.push(arg.to_string());
    }
    if p.fullscreen {
        jvm_args.push("-Dorg.lwjgl.opengl.Window.undecorated=true".into());
    }

    let mut game_args: Vec<String> = Vec::new();
    if let Some(args) = &v.arguments {
        for arg in &args.game {
            game_args.extend(arg_values(arg, &features, &map));
        }
    } else if let Some(legacy) = &v.minecraft_arguments {
        for token in legacy.split_whitespace() {
            game_args.push(substitute(token, &map));
        }
    }
    if p.width > 0 && p.height > 0 {
        // если arguments.game уже содержит --width (правило has_custom_resolution), не дублируем
        if !game_args.iter().any(|a| a == "--width") {
            game_args.push("--width".into());
            game_args.push(p.width.to_string());
            game_args.push("--height".into());
            game_args.push(p.height.to_string());
        }
    }
    if p.fullscreen && !game_args.iter().any(|a| a == "--fullscreen") {
        game_args.push("--fullscreen".into());
    }

    let mut cmd = tokio::process::Command::new(&p.java_path);
    cmd.args(&jvm_args)
        .arg("-cp")
        .arg(&classpath)
        .arg(&v.main_class)
        .args(&game_args)
        .current_dir(&p.instance_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::msg(format!("Не удалось запустить игру: {e}")))?;
    let pid = child.id().unwrap_or(0);
    state.running.lock().unwrap().insert(p.instance_id.clone(), pid);
    state.owned_runs.lock().unwrap().insert(p.instance_id.clone());

    let emit_line = |app: &AppHandle, instance_id: &str, line: String, stream: &str| {
        let _ = app.emit(
            "launch://log",
            LogLine {
                instance_id: instance_id.to_string(),
                line,
                stream: stream.to_string(),
            },
        );
    };
    emit_line(app, &p.instance_id, format!("[YammyLaunch] Игра запущена (PID {pid})"), "launcher");

    let instance_id = p.instance_id.clone();
    let app2 = app.clone();
    if let Some(stdout) = child.stdout.take() {
        let iid = instance_id.clone();
        let app3 = app2.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app3.emit("launch://log", LogLine { instance_id: iid.clone(), line, stream: "stdout".into() });
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let iid = instance_id.clone();
        let app3 = app2.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app3.emit("launch://log", LogLine { instance_id: iid.clone(), line, stream: "stderr".into() });
            }
        });
    }

    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
        let owned = if let Some(state) = app2.try_state::<AppState>() {
            state.running.lock().unwrap().remove(&instance_id);
            state.owned_runs.lock().unwrap().remove(&instance_id)
        } else {
            false
        };
        // Сообщаем о выходе только тех игр, что запустили сами в этой сессии.
        if owned {
            let _ = app2.emit("launch://exit", serde_json::json!({ "instanceId": instance_id, "code": code }));
        }
    });

    Ok(pid)
}

pub fn kill_game(state: &AppState, instance_id: &str) -> Result<()> {
    let pid = state.running.lock().unwrap().get(instance_id).copied();
    if let Some(pid) = pid {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags_no_window()
                .output();
        }
        state.running.lock().unwrap().remove(instance_id);
    }
    Ok(())
}

#[cfg(windows)]
trait CreationFlagsExt {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}

#[cfg(windows)]
impl CreationFlagsExt for std::process::Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(0x08000000)
    }
}