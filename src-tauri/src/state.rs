use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::settings::{self, Settings};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub http: reqwest::Client,
    /// Запущенные игры: instance_id -> pid
    pub running: Mutex<HashMap<String, u32>>,
    /// Игры, запущенные в этой сессии лаунчера (о чужих выходах не сообщаем)
    pub owned_runs: Mutex<std::collections::HashSet<String>>,
    /// Активные задачи входа Microsoft (для отмены)
    pub auth_cancel: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
}

impl AppState {
    pub fn new() -> Self {
        let base = settings::root_dir(&Settings::default()).expect("no data dir");
        std::fs::create_dir_all(&base).ok();
        let settings_path = base.join("settings.json");
        let settings = settings::load(&settings_path);
        let http = reqwest::Client::builder()
            .user_agent("YammyLaunch/0.1.0 (github.com/solov/YammyLaunch)")
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("http client");
        Self {
            settings: Mutex::new(settings),
            settings_path,
            http,
            running: Mutex::new(HashMap::new()),
            owned_runs: Mutex::new(std::collections::HashSet::new()),
            auth_cancel: Mutex::new(HashMap::new()),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn root(&self) -> crate::error::Result<PathBuf> {
        let s = self.settings();
        let root = settings::root_dir(&s)?;
        std::fs::create_dir_all(&root)?;
        Ok(root)
    }
}