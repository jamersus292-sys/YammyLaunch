mod ai;
mod auth;
mod download;
mod error;
mod fabric;
mod forge;
mod instances;
mod java;
mod launch;
mod modpacks;
mod mojang;
mod modrinth;
mod neoforge;
mod optifine;
mod optimize;
mod settings;
mod skins;
mod state;
mod system;

use error::Result;
use serde::Serialize;
use state::AppState;
use tauri::{AppHandle, State};

// ---------- версии ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionList {
    latest_release: String,
    latest_snapshot: String,
    versions: Vec<mojang::ManifestVersion>,
}

#[tauri::command]
async fn get_versions(state: State<'_, AppState>) -> Result<VersionList> {
    let manifest = mojang::fetch_manifest(&state).await?;
    Ok(VersionList {
        latest_release: manifest.latest.release,
        latest_snapshot: manifest.latest.snapshot,
        versions: manifest.versions,
    })
}

#[tauri::command]
async fn prepare_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Result<()> {
    let inst = instances::get(&state, &instance_id)?;
    let root = state.root()?;
    // ванильная версия должна быть скачана (json)
    let manifest = mojang::fetch_manifest(&state).await?;
    let mv = manifest
        .versions
        .iter()
        .find(|v| v.id == inst.game_version)
        .ok_or_else(|| error::Error::msg(format!("Версия {} не найдена в манифесте", inst.game_version)))?;
    mojang::fetch_version_json(&state, mv).await?;
    launch::prepare_version(&app, &state, &inst.version_id).await?;
    Ok(())
}

// ---------- инстансы ----------

#[tauri::command]
fn list_instances(state: State<'_, AppState>) -> Result<Vec<instances::Instance>> {
    instances::load_all(&state)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateInstanceArgs {
    name: String,
    game_version: String,
    loader: String,
    loader_version: Option<String>,
    #[serde(default)]
    optifine: bool,
    #[serde(default)]
    optifine_version: Option<String>,
}

#[tauri::command]
async fn create_instance(app: AppHandle, state: State<'_, AppState>, args: CreateInstanceArgs) -> Result<instances::Instance> {
    let mut inst = instances::Instance {
        name: args.name.trim().to_string(),
        game_version: args.game_version.clone(),
        loader: args.loader.clone(),
        ..Default::default()
    };
    if inst.name.is_empty() {
        inst.name = format!("{} {}", args.loader, args.game_version);
    }
    let dir = instances::create_dirs(&state, &inst)?;

    match args.loader.as_str() {
        "vanilla" => {
            inst.version_id = args.game_version.clone();
        }
        "fabric" => {
            let loaders = fabric::loader_versions(&state, &args.game_version).await?;
            let loader_version = args
                .loader_version
                .clone()
                .unwrap_or_else(|| loaders[0].loader.version.clone());
            let profile = fabric::install_profile(&state, &args.game_version, &loader_version).await?;
            inst.loader_version = Some(loader_version);
            inst.version_id = profile.id;
        }
        "forge" => {
            // Если выбран OptiFine — Forge должен быть совместим с ним,
            // иначе OptiFine патчит классы под старую структуру и игра крашится.
            let forge_version = if args.optifine {
                let entries = optifine::versions_for(&state, &args.game_version).await?;
                let of_entry = match &args.optifine_version {
                    Some(label) => entries
                        .iter()
                        .find(|e| e.label() == *label || e.patch == *label)
                        .cloned()
                        .unwrap_or_else(|| entries[0].clone()),
                    None => entries[0].clone(),
                };
                match optifine::compatible_forge(&of_entry) {
                    Some(v) => v,
                    None => {
                        return Err(error::Error::msg(format!(
                            "OptiFine {} несовместим с Forge для {}. Выбери другую версию OptiFine или поставь Forge без OptiFine.",
                            of_entry.label(),
                            args.game_version
                        )));
                    }
                }
            } else {
                match args.loader_version {
                    Some(v) => v,
                    None => forge::recommended_version(&state, &args.game_version).await?,
                }
            };
            // узнаём нужную Java из ванильного version JSON
            let manifest = mojang::fetch_manifest(&state).await?;
            let mv = manifest
                .versions
                .iter()
                .find(|v| v.id == args.game_version)
                .ok_or_else(|| error::Error::msg("Версия игры не найдена"))?;
            let vj = mojang::fetch_version_json(&state, mv).await?;
            let version_id = forge::install(&app, &state, &dir, &args.game_version, &forge_version, vj.java_major()).await?;
            inst.loader_version = Some(forge_version);
            inst.version_id = version_id;
        }
        "neoforge" => {
            let nf_version = match args.loader_version {
                Some(v) => v,
                None => neoforge::recommended_version(&state, &args.game_version).await?,
            };
            let manifest = mojang::fetch_manifest(&state).await?;
            let mv = manifest
                .versions
                .iter()
                .find(|v| v.id == args.game_version)
                .ok_or_else(|| error::Error::msg("Версия игры не найдена"))?;
            let vj = mojang::fetch_version_json(&state, mv).await?;
            let version_id = neoforge::install(&app, &state, &dir, &args.game_version, &nf_version, vj.java_major()).await?;
            inst.loader_version = Some(nf_version);
            inst.version_id = version_id;
        }
        other => return Err(error::Error::msg(format!("Неизвестный лоадер: {other}"))),
    }

    // OptiFine поверх vanilla/fabric/forge
    if args.optifine {
        let entries = optifine::versions_for(&state, &args.game_version).await?;
        let entry = match &args.optifine_version {
            Some(label) => entries
                .iter()
                .find(|e| e.label() == *label || e.patch == *label)
                .cloned()
                .unwrap_or_else(|| entries[0].clone()),
            None => entries[0].clone(),
        };
        if args.loader == "fabric" {
            // OptiFabric — мост между Fabric и OptiFine
            modrinth::install_mod(
                &app,
                &state,
                &inst.id,
                optifine::OPTIFABRIC_PROJECT_ID,
                &args.game_version,
                "fabric",
            )
            .await?;
        }
        let version_id = optifine::install(&app, &state, &args.game_version, &entry, &dir, &args.loader).await?;
        inst.optifine_version = Some(entry.label());
        // Forge: OptiFine ставится как мод, version JSON не меняется
        if !version_id.is_empty() {
            inst.version_id = version_id;
        }
    }

    instances::upsert(&state, inst.clone())?;
    Ok(inst)
}

#[tauri::command]
fn update_instance(state: State<'_, AppState>, instance: instances::Instance) -> Result<()> {
    instances::upsert(&state, instance)
}

#[tauri::command]
fn delete_instance(state: State<'_, AppState>, id: String) -> Result<()> {
    instances::delete(&state, &id)
}

#[tauri::command]
async fn open_instance_folder(state: State<'_, AppState>, id: String) -> Result<()> {
    let dir = instances::instance_dir(&state, &id)?;
    std::fs::create_dir_all(&dir)?;
    open_path(&dir)
}

fn open_path(path: &std::path::Path) -> Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    Ok(())
}

// ---------- запуск ----------

#[tauri::command]
async fn launch_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Result<u32> {
    let inst = instances::get(&state, &instance_id)?;
    let account = auth::active_account(&state)?;
    let settings = state.settings();
    let dir = instances::create_dirs(&state, &inst)?;

    let (access_token, user_type, xuid) = if account.kind == "msa" {
        let token = auth::ensure_mc_token(&state, &account).await?;
        (token, "msa".to_string(), "0".to_string())
    } else {
        ("0".to_string(), "legacy".to_string(), "0".to_string())
    };

    // Java: override инстанса > глобальный override > авто
    let root = state.root()?;
    let raw = mojang::load_version_json(&root, &inst.version_id)?;
    let v = mojang::resolve_inheritance(&root, raw)?;
    let override_path = inst
        .java_path
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(settings.java_path_override.as_deref().filter(|s| !s.is_empty()));
    let java_path = java::ensure_java(&app, &state, v.java_major(), override_path).await?;

    // Оффлайн-скин: поднимаем локальный скин-сервер и authlib-injector
    let (skin_server_url, authlib_injector_path) = if account.kind == "offline" {
        let info = skins::info(&state, &account.id);
        if info.has_skin || info.has_cape {
            let url = skins::ensure_server(&state).await.ok();
            let injector = skins::ensure_authlib_injector(&state).await.ok();
            (url, injector)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let params = launch::LaunchParams {
        instance_id: inst.id.clone(),
        instance_dir: dir,
        version_id: inst.version_id.clone(),
        username: account.username,
        uuid: account.uuid,
        access_token,
        user_type,
        xuid,
        memory_max_mb: inst.memory_max_mb.unwrap_or(settings.memory_max_mb),
        memory_min_mb: inst.memory_min_mb.unwrap_or(settings.memory_min_mb),
        java_path,
        width: inst.resolution_width.unwrap_or(settings.resolution_width),
        height: inst.resolution_height.unwrap_or(settings.resolution_height),
        fullscreen: settings.fullscreen,
        extra_jvm_args: settings.extra_jvm_args.clone(),
        skin_server_url,
        authlib_injector_path,
    };
    let pid = launch::launch_game(&app, &state, params).await?;
    instances::touch_last_played(&state, &inst.id)?;
    Ok(pid)
}

#[tauri::command]
fn kill_game(state: State<'_, AppState>, instance_id: String) -> Result<()> {
    launch::kill_game(&state, &instance_id)
}

#[tauri::command]
fn running_instances(state: State<'_, AppState>) -> Vec<String> {
    state.running.lock().unwrap().keys().cloned().collect()
}

// ---------- аккаунты ----------

#[tauri::command]
fn get_accounts(state: State<'_, AppState>) -> Result<auth::AccountsFile> {
    auth::load_accounts(&state)
}

#[tauri::command]
fn add_offline_account(state: State<'_, AppState>, username: String) -> Result<auth::Account> {
    auth::add_offline_account(&state, &username)
}

#[tauri::command]
fn remove_account(state: State<'_, AppState>, id: String) -> Result<()> {
    auth::remove_account(&state, &id)
}

#[tauri::command]
fn set_active_account(state: State<'_, AppState>, id: String) -> Result<()> {
    auth::set_active(&state, &id)
}

#[tauri::command]
async fn start_msa_login(app: AppHandle, state: State<'_, AppState>) -> Result<String> {
    auth::start_device_flow(&app, &state).await
}

#[tauri::command]
fn cancel_msa_login(state: State<'_, AppState>, session_id: String) {
    auth::cancel_device_flow(&state, &session_id);
}

/// Открывает страницу входа Microsoft во встроенном окне лаунчера (как в Lunar Client).
/// Код устройства подставляется в URL, пользователю остаётся ввести логин и пароль.
#[tauri::command]
async fn open_msa_login_window(app: AppHandle, user_code: String) -> Result<()> {
    use tauri::Manager;
    // закрываем предыдущее окно входа, если осталось
    if let Some(w) = app.get_webview_window("msa-login") {
        let _ = w.close();
    }
    let url = format!("https://www.microsoft.com/link?otc={}", user_code.trim());
    let parsed: tauri::Url = url
        .parse()
        .map_err(|_| error::Error::msg("Некорректный URL входа"))?;
    tauri::WebviewWindowBuilder::new(&app, "msa-login", tauri::WebviewUrl::External(parsed))
        .title("Вход Microsoft — YammyLaunch")
        .inner_size(520.0, 680.0)
        .min_inner_size(420.0, 520.0)
        .center()
        .resizable(true)
        .build()
        .map_err(|_| error::Error::msg("Не удалось открыть окно входа"))?;
    Ok(())
}

#[tauri::command]
fn close_msa_login_window(app: AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("msa-login") {
        let _ = w.close();
    }
}

// ---------- скины и плащи (оффлайн) ----------

#[tauri::command]
fn set_skin(state: State<'_, AppState>, account_id: String, bytes: Vec<u8>, model: String) -> Result<skins::SkinInfo> {
    skins::set_skin(&state, &account_id, &bytes, &model)
}

#[tauri::command]
fn set_cape(state: State<'_, AppState>, account_id: String, bytes: Vec<u8>) -> Result<skins::SkinInfo> {
    skins::set_cape(&state, &account_id, &bytes)
}

#[tauri::command]
fn remove_skin(state: State<'_, AppState>, account_id: String) -> Result<skins::SkinInfo> {
    skins::remove_skin(&state, &account_id)
}

#[tauri::command]
fn remove_cape(state: State<'_, AppState>, account_id: String) -> Result<skins::SkinInfo> {
    skins::remove_cape(&state, &account_id)
}

#[tauri::command]
fn skin_preview(state: State<'_, AppState>, account_id: String) -> Result<Option<String>> {
    skins::skin_preview(&state, &account_id)
}

#[tauri::command]
fn cape_preview(state: State<'_, AppState>, account_id: String) -> Result<Option<String>> {
    skins::cape_preview(&state, &account_id)
}

// ---------- моды ----------

#[tauri::command]
async fn search_mods(
    state: State<'_, AppState>,
    query: String,
    game_version: String,
    loader: String,
    offset: u64,
) -> Result<modrinth::SearchResponse> {
    modrinth::search(&state, &query, &game_version, &loader, offset).await
}

#[tauri::command]
async fn install_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> Result<Vec<instances::InstalledMod>> {
    let inst = instances::get(&state, &instance_id)?;
    let loader = if inst.loader == "vanilla" { "fabric" } else { inst.loader.as_str() };
    modrinth::install_mod(&app, &state, &instance_id, &project_id, &inst.game_version, loader).await
}

#[tauri::command]
fn list_mods(state: State<'_, AppState>, instance_id: String) -> Result<Vec<instances::InstalledMod>> {
    modrinth::list_mods(&state, &instance_id)
}

#[tauri::command]
fn remove_mod(state: State<'_, AppState>, instance_id: String, file_name: String) -> Result<()> {
    modrinth::remove_mod(&state, &instance_id, &file_name)
}

#[tauri::command]
fn toggle_mod(state: State<'_, AppState>, instance_id: String, file_name: String) -> Result<String> {
    modrinth::toggle_mod(&state, &instance_id, &file_name)
}

#[tauri::command]
async fn fabric_loaders(state: State<'_, AppState>, game_version: String) -> Result<Vec<fabric::FabricLoaderEntry>> {
    fabric::loader_versions(&state, &game_version).await
}

/// Какие лоадеры доступны для версии игры.
/// NeoForge существует только с 1.20.2 (и год-версий 26.x), Fabric — с 1.14.
#[tauri::command]
async fn available_loaders(state: State<'_, AppState>, game_version: String) -> Result<Vec<String>> {
    let mut loaders = vec!["vanilla".to_string()];
    // Fabric: проверяем по официальному списку
    if let Ok(games) = fabric::supported_game_versions(&state).await {
        if games.iter().any(|g| g.version == game_version) {
            loaders.push("fabric".to_string());
        }
    }
    // Forge: есть промо для этой версии?
    if forge::recommended_version(&state, &game_version).await.is_ok() {
        loaders.push("forge".to_string());
    }
    // NeoForge: только 1.20.2+ и год-версии
    if neoforge_supported(&game_version) {
        loaders.push("neoforge".to_string());
    }
    Ok(loaders)
}

fn neoforge_supported(game_version: &str) -> bool {
    let parts: Vec<&str> = game_version.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let major: u32 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if major >= 26 {
        return true; // год-версии 26.x
    }
    if major != 1 {
        return false;
    }
    let minor: u32 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if minor > 20 {
        return true;
    }
    if minor == 20 {
        let patch: u32 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        return patch >= 2;
    }
    false
}

// ---------- OptiFine ----------

#[tauri::command]
async fn optifine_versions(state: State<'_, AppState>, game_version: String) -> Result<Vec<optifine::OptifineEntry>> {
    optifine::versions_for(&state, &game_version).await
}

// ---------- готовые сборки ----------

#[tauri::command]
async fn search_modpacks(state: State<'_, AppState>, query: String, offset: u64) -> Result<modpacks::PackSearchResponse> {
    modpacks::search(&state, &query, offset).await
}

#[tauri::command]
async fn install_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    name: Option<String>,
) -> Result<instances::Instance> {
    modpacks::install_pack(&app, &state, &project_id, name).await
}

// ---------- ИИ-помощник ----------

#[tauri::command]
async fn ai_chat(state: State<'_, AppState>, messages: Vec<ai::ChatMessage>) -> Result<String> {
    ai::chat(&state, messages).await
}

// ---------- система и оптимизация ----------

#[tauri::command]
fn get_system_info() -> Result<system::SystemInfo> {
    system::system_info()
}

#[tauri::command]
fn get_recommended_settings() -> Result<system::RecommendedSettings> {
    let info = system::system_info()?;
    Ok(system::recommend(&info))
}

#[tauri::command]
async fn apply_recommended_settings(state: State<'_, AppState>) -> Result<system::RecommendedSettings> {
    let info = system::system_info()?;
    let rec = system::recommend(&info);
    let mut s = state.settings();
    s.memory_max_mb = rec.memory_max_mb;
    s.memory_min_mb = rec.memory_min_mb;
    s.extra_jvm_args = rec.extra_jvm_args.clone();
    settings::save(&state.settings_path, &s)?;
    *state.settings.lock().unwrap() = s;
    Ok(rec)
}

#[tauri::command]
fn get_optimize_pack(loader: String) -> optimize::OptimizePack {
    optimize::pack_info(&loader)
}

#[tauri::command]
async fn install_optimize_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    with_shaders: bool,
) -> Result<optimize::OptimizeResult> {
    optimize::install_pack(&app, &state, &instance_id, with_shaders).await
}

// ---------- настройки ----------

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> settings::Settings {
    state.settings()
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, new_settings: settings::Settings) -> Result<()> {
    settings::save(&state.settings_path, &new_settings)?;
    *state.settings.lock().unwrap() = new_settings;
    Ok(())
}

#[tauri::command]
fn get_data_dir(state: State<'_, AppState>) -> Result<String> {
    Ok(state.root()?.to_string_lossy().to_string())
}

#[tauri::command]
async fn open_data_folder(state: State<'_, AppState>) -> Result<()> {
    let root = state.root()?;
    open_path(&root)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_versions,
            prepare_instance,
            list_instances,
            create_instance,
            update_instance,
            delete_instance,
            open_instance_folder,
            launch_instance,
            kill_game,
            running_instances,
            get_accounts,
            add_offline_account,
            remove_account,
            set_active_account,
            start_msa_login,
            cancel_msa_login,
            open_msa_login_window,
            close_msa_login_window,
            set_skin,
            set_cape,
            remove_skin,
            remove_cape,
            skin_preview,
            cape_preview,
            search_mods,
            install_mod,
            list_mods,
            remove_mod,
            toggle_mod,
            fabric_loaders,
            available_loaders,
            optifine_versions,
            search_modpacks,
            install_modpack,
            ai_chat,
            get_system_info,
            get_recommended_settings,
            apply_recommended_settings,
            get_optimize_pack,
            install_optimize_pack,
            get_settings,
            save_settings,
            get_data_dir,
            open_data_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}