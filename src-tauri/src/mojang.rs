use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::state::AppState;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Download>,
    #[serde(default)]
    pub classifiers: HashMap<String, Download>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub extract: Option<Extract>,
    /// Maven-репозиторий для Fabric/Forge библиотек без downloads
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Ruled { rules: Vec<Rule>, value: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexInfo {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(rename = "totalSize", default)]
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    #[serde(default)]
    pub component: Option<String>,
    #[serde(rename = "majorVersion", default)]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingClient {
    pub file: LoggingFile,
    pub argument: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default)]
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(rename = "assetIndex", default)]
    pub asset_index: Option<AssetIndexInfo>,
    #[serde(default)]
    pub downloads: HashMap<String, Download>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub logging: Option<Logging>,
    #[serde(rename = "inheritsFrom", default)]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub jar: Option<String>,
}

impl VersionJson {
    pub fn java_major(&self) -> u32 {
        self.java_version
            .as_ref()
            .map(|j| j.major_version)
            .filter(|&v| v > 0)
            .unwrap_or(8)
    }
}

/// Оценка правил Mojang: последнее совпавшее правило выигрывает.
pub fn rules_allow(rules: &[Rule], features: &HashMap<String, bool>) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        let mut matches = true;
        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                let cur = std::env::consts::OS;
                let cur = match cur {
                    "macos" => "osx",
                    other => other,
                };
                if name != cur {
                    matches = false;
                }
            }
            if let Some(arch) = &os.arch {
                let cur = if cfg!(target_pointer_width = "32") { "x86" } else { "x64" };
                if arch != cur && !(arch == "x86_64" && cur == "x64") {
                    matches = false;
                }
            }
            if let Some(ver) = &os.version {
                if let Ok(re) = regex_lite_match(ver) {
                    if !re {
                        matches = false;
                    }
                }
            }
        }
        if let Some(feats) = &rule.features {
            for (k, want) in feats {
                let have = features.get(k).copied().unwrap_or(false);
                if have != *want {
                    matches = false;
                    break;
                }
            }
        }
        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn regex_lite_match(_pattern: &str) -> std::result::Result<bool, ()> {
    // os.version используется крайне редко (старые osx-правила); на Windows игнорируем.
    Ok(true)
}

pub fn library_allowed(lib: &Library) -> bool {
    rules_allow(&lib.rules, &HashMap::new())
}

/// Maven-координаты group:artifact:version[:classifier] -> относительный путь
pub fn maven_path(name: &str) -> Result<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return Err(Error::msg(format!("Некорректные maven-координаты: {name}")));
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let file = if parts.len() > 3 {
        format!("{artifact}-{version}-{}.jar", parts[3])
    } else {
        format!("{artifact}-{version}.jar")
    };
    Ok(format!("{group}/{artifact}/{version}/{file}"))
}

/// Ключ дедупликации библиотеки: group:artifact[:classifier].
/// Классификатор важен: у Forge universal и client — разные jar с одинаковыми group:artifact.
pub fn maven_ga(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() >= 4 {
        format!("{}:{}:{}", parts[0], parts[1], parts[3])
    } else if parts.len() >= 2 {
        format!("{}:{}", parts[0], parts[1])
    } else {
        name.to_string()
    }
}

pub async fn fetch_manifest(state: &AppState) -> Result<VersionManifest> {
    let root = state.root()?;
    let cache = root.join("version_manifest.json");
    // Кэш на 10 минут
    if let Ok(meta) = std::fs::metadata(&cache) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().map(|d| d.as_secs() < 600).unwrap_or(false) {
                if let Ok(s) = std::fs::read_to_string(&cache) {
                    if let Ok(m) = serde_json::from_str(&s) {
                        return Ok(m);
                    }
                }
            }
        }
    }
    let manifest: VersionManifest = state.http.get(MANIFEST_URL).send().await?.error_for_status()?.json().await?;
    std::fs::write(&cache, serde_json::to_string(&manifest)?)?;
    Ok(manifest)
}

pub fn version_json_path(root: &Path, id: &str) -> PathBuf {
    root.join("versions").join(id).join(format!("{id}.json"))
}

pub async fn fetch_version_json(state: &AppState, mv: &ManifestVersion) -> Result<VersionJson> {
    let root = state.root()?;
    let path = version_json_path(&root, &mv.id);
    if path.exists() {
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<VersionJson>(&s) {
                return Ok(v);
            }
        }
    }
    let v: VersionJson = state.http.get(&mv.url).send().await?.error_for_status()?.json().await?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&v)?)?;
    Ok(v)
}

pub fn load_version_json(root: &Path, id: &str) -> Result<VersionJson> {
    let path = version_json_path(root, id);
    let s = std::fs::read_to_string(&path)
        .map_err(|_| Error::msg(format!("Версия {id} не скачана")))?;
    Ok(serde_json::from_str(&s)?)
}

/// Разворачивает цепочку inheritsFrom (Fabric/Forge) в один плоский VersionJson.
pub fn resolve_inheritance(root: &Path, mut v: VersionJson) -> Result<VersionJson> {
    let mut depth = 0;
    while let Some(parent_id) = v.inherits_from.clone() {
        depth += 1;
        if depth > 5 {
            return Err(Error::msg("Слишком глубокая цепочка inheritsFrom"));
        }
        let parent = load_version_json(root, &parent_id)?;
        // Библиотеки родителя + наши (наши перекрывают по group:artifact)
        let mut libs: Vec<Library> = parent.libraries.clone();
        for lib in &v.libraries {
            let ga = maven_ga(&lib.name);
            libs.retain(|l| maven_ga(&l.name) != ga);
            libs.push(lib.clone());
        }
        let mut arguments = parent.arguments.clone().unwrap_or_default();
        if let Some(child_args) = &v.arguments {
            arguments.game.extend(child_args.game.clone());
            arguments.jvm.extend(child_args.jvm.clone());
        }
        let downloads = if v.downloads.is_empty() { parent.downloads.clone() } else { v.downloads.clone() };
        let asset_index = v.asset_index.clone().or(parent.asset_index.clone());
        let assets = v.assets.clone().or(parent.assets.clone());
        let java_version = v.java_version.clone().or(parent.java_version.clone());
        let logging = v.logging.clone().or(parent.logging.clone());
        let minecraft_arguments = v.minecraft_arguments.clone().or(parent.minecraft_arguments.clone());
        let jar = v.jar.clone().or(parent.jar.clone()).or(Some(parent.id.clone()));
        v = VersionJson {
            id: v.id,
            kind: v.kind.or(parent.kind),
            main_class: v.main_class,
            assets,
            asset_index,
            downloads,
            libraries: libs,
            arguments: Some(arguments),
            minecraft_arguments,
            java_version,
            logging,
            inherits_from: parent.inherits_from,
            jar,
        };
    }
    Ok(v)
}