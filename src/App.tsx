import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence, motion } from "framer-motion";
import TitleBar from "./components/TitleBar";
import Sidebar from "./components/Sidebar";
import Toasts from "./components/Toasts";
import PlayPage from "./pages/Play";
import InstancesPage from "./pages/Instances";
import PacksPage from "./pages/Packs";
import ModBrowser from "./pages/ModBrowser";
import AiPage from "./pages/Ai";
import AccountsPage from "./pages/Accounts";
import SettingsPage from "./pages/Settings";
import { useStore } from "./stores/app";
import { ProgressEvent } from "./lib/tauri";

export default function App() {
  const page = useStore((s) => s.page);
  const refreshInstances = useStore((s) => s.refreshInstances);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const refreshVersions = useStore((s) => s.refreshVersions);
  const refreshSettings = useStore((s) => s.refreshSettings);
  const refreshRunning = useStore((s) => s.refreshRunning);
  const setProgress = useStore((s) => s.setProgress);

  useEffect(() => {
    refreshInstances();
    refreshAccounts();
    refreshVersions();
    refreshSettings();
    refreshRunning();
    const un: (() => void)[] = [];
    (async () => {
      un.push(
        await listen<ProgressEvent>("download://progress", (e) => {
          setProgress(e.payload);
        })
      );
    })();
    const interval = setInterval(refreshRunning, 5000);
    return () => {
      un.forEach((u) => u());
      clearInterval(interval);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="h-full flex flex-col">
      <TitleBar />
      <div className="flex-1 flex min-h-0">
        <Sidebar />
        <AnimatePresence mode="wait">
          <motion.div
            key={page}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.15 }}
            className="flex-1 flex flex-col min-h-0 min-w-0"
          >
            {page === "play" && <PlayPage />}
            {page === "instances" && <InstancesPage />}
            {page === "packs" && <PacksPage />}
            {page === "mods" && <ModBrowser />}
            {page === "ai" && <AiPage />}
            {page === "accounts" && <AccountsPage />}
            {page === "settings" && <SettingsPage />}
          </motion.div>
        </AnimatePresence>
      </div>
      <Toasts />
    </div>
  );
}