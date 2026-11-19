use serde::Serialize;

use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub ram_total_mb: u64,
    pub ram_free_mb: u64,
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub gpu_name: String,
    pub gpu_vram_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedSettings {
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub extra_jvm_args: String,
    pub explanation: String,
}

#[cfg(windows)]
pub fn system_info() -> Result<SystemInfo> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let script = r#"
$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$gpu = Get-CimInstance Win32_VideoController | Where-Object { $_.AdapterRAM -gt 0 } | Sort-Object AdapterRAM -Descending | Select-Object -First 1
# WMI AdapterRAM врёт (лимит 4 ГБ) — настоящий VRAM берём из реестра драйвера
$vram = 0
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' -ErrorAction SilentlyContinue | ForEach-Object {
  $m = $_.'HardwareInformation.qwMemorySize'
  if ($m -and $m -gt $vram) { $vram = $m }
}
if ($vram -eq 0 -and $gpu) { $vram = $gpu.AdapterRAM }
[pscustomobject]@{
  ramTotalMb = [math]::Round($cs.TotalPhysicalMemory / 1MB)
  ramFreeMb = [math]::Round($os.FreePhysicalMemory / 1KB)
  cpuName = $cpu.Name.Trim()
  cpuCores = $cpu.NumberOfCores
  cpuThreads = $cpu.NumberOfLogicalProcessors
  gpuName = if ($gpu) { $gpu.Name } else { 'unknown' }
  gpuVramMb = [math]::Round($vram / 1MB)
} | ConvertTo-Json -Compress
"#;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| crate::error::Error::msg("Не удалось прочитать характеристики ПК"))?;
    Ok(SystemInfo {
        ram_total_mb: v["ramTotalMb"].as_u64().unwrap_or(8192),
        ram_free_mb: v["ramFreeMb"].as_u64().unwrap_or(4096),
        cpu_name: v["cpuName"].as_str().unwrap_or("CPU").to_string(),
        cpu_cores: v["cpuCores"].as_u64().unwrap_or(4) as u32,
        cpu_threads: v["cpuThreads"].as_u64().unwrap_or(8) as u32,
        gpu_name: v["gpuName"].as_str().unwrap_or("GPU").to_string(),
        gpu_vram_mb: v["gpuVramMb"].as_u64().unwrap_or(0),
    })
}

#[cfg(not(windows))]
pub fn system_info() -> Result<SystemInfo> {
    Ok(SystemInfo {
        ram_total_mb: 8192,
        ram_free_mb: 4096,
        cpu_name: "CPU".into(),
        cpu_cores: 4,
        cpu_threads: 8,
        gpu_name: "GPU".into(),
        gpu_vram_mb: 0,
    })
}

/// Рекомендации по памяти и JVM-флагам на основе железа.
pub fn recommend(info: &SystemInfo) -> RecommendedSettings {
    let total = info.ram_total_mb;
    // Minecraft редко нужно больше 6-8 ГБ; оставляем системе минимум 3 ГБ
    let max = if total <= 4096 {
        2048
    } else if total <= 8192 {
        3072
    } else if total <= 16384 {
        4096
    } else {
        6144
    };
    let min = (max / 4).max(512);

    // G1GC — стабильный выбор для MC; тюнинг под многоядерность
    let threads = info.cpu_threads.max(2);
    let conc_threads = (threads / 4).max(1);
    let mut args = String::from(
        "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1",
    );
    args.push_str(&format!(" -XX:ConcGCThreads={conc_threads} -XX:ParallelGCThreads={threads}"));

    let ram_gb = total / 1024;
    let explanation = format!(
        "ПК: {} ГБ ОЗУ, {} ({} потоков), {}. Выделено {} МБ игре, остальное остаётся системе. G1GC настроен под {} потоков.",
        ram_gb, info.cpu_name, info.cpu_threads, info.gpu_name, max, threads
    );

    RecommendedSettings {
        memory_max_mb: max,
        memory_min_mb: min,
        extra_jvm_args: args,
        explanation,
    }
}