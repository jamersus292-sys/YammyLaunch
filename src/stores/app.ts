import { create } from "zustand";
import {
  api,
  Account,
  AccountsFile,
  Instance,
  ProgressEvent,
  Settings,
  VersionList,
} from "../lib/tauri";
import { sfx, setSoundEnabled } from "../lib/sounds";

export type Page = "play" | "instances" | "packs" | "mods" | "ai" | "accounts" | "settings";

interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "success";
}

interface AppStore {
  page: Page;
  setPage: (p: Page) => void;

  selectedInstanceId: string | null;
  setSelectedInstanceId: (id: string | null) => void;

  instances: Instance[];
  refreshInstances: () => Promise<void>;

  accounts: AccountsFile;
  refreshAccounts: () => Promise<void>;

  versions: VersionList | null;
  refreshVersions: () => Promise<void>;

  settings: Settings | null;
  refreshSettings: () => Promise<void>;

  running: string[];
  refreshRunning: () => Promise<void>;

  progress: ProgressEvent | null;
  setProgress: (p: ProgressEvent | null) => void;

  busy: string | null;
  setBusy: (b: string | null) => void;

  soundOn: boolean;
  setSoundOn: (v: boolean) => void;

  toasts: Toast[];
  toast: (text: string, kind?: Toast["kind"]) => void;
  dismissToast: (id: number) => void;
}

let toastId = 0;

export const useStore = create<AppStore>((set, get) => ({
  page: "play",
  setPage: (page) => {
    if (get().page !== page) sfx.nav();
    set({ page });
  },

  selectedInstanceId: null,
  setSelectedInstanceId: (selectedInstanceId) => set({ selectedInstanceId }),

  instances: [],
  refreshInstances: async () => {
    try {
      set({ instances: await api.listInstances() });
    } catch (e) {
      get().toast(String(e), "error");
    }
  },

  accounts: { accounts: [], activeId: null },
  refreshAccounts: async () => {
    try {
      set({ accounts: await api.getAccounts() });
    } catch (e) {
      get().toast(String(e), "error");
    }
  },

  versions: null,
  refreshVersions: async () => {
    try {
      set({ versions: await api.getVersions() });
    } catch (e) {
      get().toast("Не удалось загрузить список версий: " + String(e), "error");
    }
  },

  settings: null,
  refreshSettings: async () => {
    try {
      set({ settings: await api.getSettings() });
    } catch (e) {
      get().toast(String(e), "error");
    }
  },

  running: [],
  refreshRunning: async () => {
    try {
      set({ running: await api.runningInstances() });
    } catch {
      /* ignore */
    }
  },

  progress: null,
  setProgress: (progress) => set({ progress }),

  busy: null,
  setBusy: (busy) => set({ busy }),

  soundOn: localStorage.getItem("yammy-sound") !== "off",
  setSoundOn: (v) => {
    localStorage.setItem("yammy-sound", v ? "on" : "off");
    setSoundEnabled(v);
    set({ soundOn: v });
    if (v) sfx.toggle();
  },

  toasts: [],
  toast: (text, kind = "info") => {
    const id = ++toastId;
    if (kind === "error") sfx.error();
    else if (kind === "success") sfx.success();
    set((s) => ({ toasts: [...s.toasts, { id, text, kind }] }));
    setTimeout(() => get().dismissToast(id), kind === "error" ? 8000 : 4000);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

setSoundEnabled(useStore.getState().soundOn);

export function activeAccount(): Account | null {
  const { accounts } = useStore.getState();
  return accounts.accounts.find((a) => a.id === accounts.activeId) ?? null;
}