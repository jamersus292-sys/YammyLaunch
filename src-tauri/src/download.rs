use futures_util::StreamExt;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub task_id: String,
    pub phase: String,
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: String,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha1_of_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hex(&hasher.finalize()))
}

fn file_ok(task: &DownloadTask) -> bool {
    if !task.dest.exists() {
        return false;
    }
    if let Some(expected) = &task.sha1 {
        if let Ok(actual) = sha1_of_file(&task.dest) {
            return actual.eq_ignore_ascii_case(expected);
        }
        return false;
    }
    if let Some(size) = task.size {
        if let Ok(meta) = std::fs::metadata(&task.dest) {
            return meta.len() == size;
        }
        return false;
    }
    true
}

async fn download_one(client: &reqwest::Client, task: &DownloadTask) -> Result<()> {
    if let Some(parent) = task.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut last_err = None;
    for attempt in 0..3 {
        match try_download(client, task).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt + 1))).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| Error::msg("Не удалось скачать файл")))
}

async fn try_download(client: &reqwest::Client, task: &DownloadTask) -> Result<()> {
    let resp = client.get(&task.url).send().await?.error_for_status()?;
    let tmp = task.dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);
    if let Some(expected) = &task.sha1 {
        let actual = hex(&hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            tokio::fs::remove_file(&tmp).await.ok();
            return Err(Error::msg(format!(
                "SHA1 не совпал для {}: ожидался {expected}, получен {actual}",
                task.url
            )));
        }
    }
    tokio::fs::rename(&tmp, &task.dest).await?;
    Ok(())
}

/// Скачивает набор файлов параллельно (12 потоков), шлёт прогресс в UI.
pub async fn download_all(
    app: &AppHandle,
    client: &reqwest::Client,
    task_id: &str,
    phase: &str,
    tasks: Vec<DownloadTask>,
) -> Result<()> {
    let total = tasks.len();
    let bytes_total: u64 = tasks.iter().map(|t| t.size.unwrap_or(0)).sum();
    let sem = Arc::new(Semaphore::new(12));
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let bytes_done = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let client = client.clone();
    let task_id = task_id.to_string();
    let phase = phase.to_string();
    let app = app.clone();

    let emit = |current: &str| {
        let _ = app.emit(
            "download://progress",
            ProgressEvent {
                task_id: task_id.clone(),
                phase: phase.clone(),
                done: done.load(std::sync::atomic::Ordering::Relaxed),
                total,
                bytes_done: bytes_done.load(std::sync::atomic::Ordering::Relaxed),
                bytes_total,
                current_file: current.to_string(),
            },
        );
    };
    emit("");

    let mut handles = Vec::new();
    for task in tasks {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let done = done.clone();
        let bytes_done = bytes_done.clone();
        let app = app.clone();
        let task_id = task_id.clone();
        let phase = phase.clone();
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let name = task
                .dest
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let size = task.size.unwrap_or(0);
            let result = if file_ok(&task) {
                Ok(())
            } else {
                download_one(&client, &task).await
            };
            let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let b = bytes_done.fetch_add(size, std::sync::atomic::Ordering::Relaxed) + size;
            let _ = app.emit(
                "download://progress",
                ProgressEvent {
                    task_id,
                    phase,
                    done: d,
                    total,
                    bytes_done: b,
                    bytes_total,
                    current_file: name,
                },
            );
            result
        });
        handles.push(handle);
    }
    let mut errors = Vec::new();
    for h in handles {
        match h.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => errors.push(e.to_string()),
            Err(e) => errors.push(e.to_string()),
        }
    }
    if !errors.is_empty() {
        return Err(Error::msg(format!(
            "Ошибки загрузки ({}): {}",
            errors.len(),
            errors.first().unwrap()
        )));
    }
    Ok(())
}