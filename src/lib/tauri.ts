import { invoke } from "@tauri-apps/api/core";

// ---------- типы ----------

export interface ManifestVersion {
  id: string;
  type: string;
  url: string;
  releaseTime: string;
  sha1?: string;
}

export interface VersionList {
  latestRelease: string;
  latestSnapshot: string;
  versions: ManifestVersion[];
}

export interface Instance {
  id: string;
  name: string;
  gameVersion: string;
  loader: "vanilla" | "fabric" | "forge" | "neoforge";
  loaderVersion?: string | null;
  optifineVersion?: string | null;
  versionId: string;
  memoryMaxMb?: number | null;
  memoryMinMb?: number | null;
  javaPath?: string | null;
  resolutionWidth?: number | null;
  resolutionHeight?: number | null;
  icon?: string | null;
  createdAt: string;
  lastPlayed?: string | null;
}

export interface Account {
  id: string;
  kind: "offline" | "msa";
  username: string;
  uuid: string;
  skinUrl?: string | null;
  createdAt?: string | null;
}

export interface AccountsFile {
  accounts: Account[];
  activeId?: string | null;
}

export interface Settings {
  memoryMaxMb: number;
  memoryMinMb: number;
  javaPathOverride?: string | null;
  resolutionWidth: number;
  resolutionHeight: number;
  fullscreen: boolean;
  showSnapshots: boolean;
  closeLauncherOnGameStart: boolean;
  msaClientId: string;
  dataDir?: string | null;
  extraJvmArgs: string;
  aiApiKey?: string | null;
}

export interface SearchHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  follows: number;
  icon_url?: string | null;
  categories: string[];
  display_categories: string[];
  versions: string[];
}

export interface SearchResponse {
  hits: SearchHit[];
  total_hits: number;
  offset: number;
  limit: number;
}

export interface InstalledMod {
  fileName: string;
  size: number;
  projectId?: string | null;
  versionId?: string | null;
  title?: string | null;
  iconUrl?: string | null;
}

export interface DeviceCodeInfo {
  sessionId: string;
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  interval: number;
}

export interface ProgressEvent {
  taskId: string;
  phase: string;
  done: number;
  total: number;
  bytesDone: number;
  bytesTotal: number;
  currentFile: string;
}

export interface LogLine {
  instanceId: string;
  line: string;
  stream: string;
}

export interface FabricLoaderEntry {
  loader: { version: string; stable: boolean };
}

export interface OptifineEntry {
  mcversion: string;
  patch: string;
  type: string;
  filename: string;
  forge?: string | null;
}

export interface PackHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  icon_url?: string | null;
  categories: string[];
  display_categories: string[];
  versions: string[];
}

export interface PackSearchResponse {
  hits: PackHit[];
  total_hits: number;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface SystemInfo {
  ramTotalMb: number;
  ramFreeMb: number;
  cpuName: string;
  cpuCores: number;
  cpuThreads: number;
  gpuName: string;
  gpuVramMb: number;
}

export interface RecommendedSettings {
  memoryMaxMb: number;
  memoryMinMb: number;
  extraJvmArgs: string;
  explanation: string;
}

export interface OptimizeModInfo {
  slug: string;
  title: string;
  description: string;
}

export interface OptimizePack {
  loader: string;
  mods: OptimizeModInfo[];
}

export interface OptimizeResult {
  installed: InstalledMod[];
  skipped: string[];
}

// ---------- команды ----------

export const api = {
  getVersions: () => invoke<VersionList>("get_versions"),
  prepareInstance: (instanceId: string) => invoke<void>("prepare_instance", { instanceId }),

  listInstances: () => invoke<Instance[]>("list_instances"),
  createInstance: (args: {
    name: string;
    gameVersion: string;
    loader: string;
    loaderVersion?: string;
    optifine?: boolean;
    optifineVersion?: string;
  }) => invoke<Instance>("create_instance", { args }),
  updateInstance: (instance: Instance) => invoke<void>("update_instance", { instance }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  openInstanceFolder: (id: string) => invoke<void>("open_instance_folder", { id }),

  launchInstance: (instanceId: string) => invoke<number>("launch_instance", { instanceId }),
  killGame: (instanceId: string) => invoke<void>("kill_game", { instanceId }),
  runningInstances: () => invoke<string[]>("running_instances"),

  getAccounts: () => invoke<AccountsFile>("get_accounts"),
  addOfflineAccount: (username: string) => invoke<Account>("add_offline_account", { username }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  setActiveAccount: (id: string) => invoke<void>("set_active_account", { id }),
  startMsaLogin: () => invoke<string>("start_msa_login"),
  cancelMsaLogin: (sessionId: string) => invoke<void>("cancel_msa_login", { sessionId }),
  openMsaLoginWindow: (userCode: string) => invoke<void>("open_msa_login_window", { userCode }),
  closeMsaLoginWindow: () => invoke<void>("close_msa_login_window"),

  searchMods: (query: string, gameVersion: string, loader: string, offset: number) =>
    invoke<SearchResponse>("search_mods", { query, gameVersion, loader, offset }),
  installMod: (instanceId: string, projectId: string) =>
    invoke<InstalledMod[]>("install_mod", { instanceId, projectId }),
  listMods: (instanceId: string) => invoke<InstalledMod[]>("list_mods", { instanceId }),
  removeMod: (instanceId: string, fileName: string) => invoke<void>("remove_mod", { instanceId, fileName }),
  toggleMod: (instanceId: string, fileName: string) => invoke<string>("toggle_mod", { instanceId, fileName }),
  fabricLoaders: (gameVersion: string) => invoke<FabricLoaderEntry[]>("fabric_loaders", { gameVersion }),
  availableLoaders: (gameVersion: string) => invoke<string[]>("available_loaders", { gameVersion }),
  optifineVersions: (gameVersion: string) => invoke<OptifineEntry[]>("optifine_versions", { gameVersion }),

  searchModpacks: (query: string, offset: number) =>
    invoke<PackSearchResponse>("search_modpacks", { query, offset }),
  installModpack: (projectId: string, name?: string) =>
    invoke<Instance>("install_modpack", { projectId, name }),

  aiChat: (messages: ChatMessage[]) => invoke<string>("ai_chat", { messages }),

  getSystemInfo: () => invoke<SystemInfo>("get_system_info"),
  getRecommendedSettings: () => invoke<RecommendedSettings>("get_recommended_settings"),
  applyRecommendedSettings: () => invoke<RecommendedSettings>("apply_recommended_settings"),
  getOptimizePack: (loader: string) => invoke<OptimizePack>("get_optimize_pack", { loader }),
  installOptimizePack: (instanceId: string, withShaders: boolean) =>
    invoke<OptimizeResult>("install_optimize_pack", { instanceId, withShaders }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (newSettings: Settings) => invoke<void>("save_settings", { newSettings }),
  getDataDir: () => invoke<string>("get_data_dir"),
  openDataFolder: () => invoke<void>("open_data_folder"),
};

export function skinHeadUrl(account: Account): string {
  if (account.kind === "msa") {
    return `https://minotar.net/helm/${account.uuid}/64.png`;
  }
  return `https://minotar.net/helm/${encodeURIComponent(account.username)}/64.png`;
}

export function formatBytes(n: number): string {
  if (n >= 1 << 30) return (n / (1 << 30)).toFixed(2) + " ГБ";
  if (n >= 1 << 20) return (n / (1 << 20)).toFixed(1) + " МБ";
  if (n >= 1 << 10) return (n / (1 << 10)).toFixed(0) + " КБ";
  return n + " Б";
}

export function formatDownloads(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(0) + "K";
  return String(n);
}