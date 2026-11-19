import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion } from "framer-motion";
import { Play, Square, Loader2, ChevronDown, FolderOpen } from "lucide-react";
import { api, LogLine, skinHeadUrl } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import ProgressBar from "../components/ProgressBar";
import LogConsole from "../components/LogConsole";
import Emoji from "../components/Emoji";
import clsx from "clsx";

const loaderLabel: Record<string, string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
};

export default function PlayPage() {
  const instances = useStore((s) => s.instances);
  const selectedId = useStore((s) => s.selectedInstanceId);
  const setSelectedId = useStore((s) => s.setSelectedInstanceId);
  const running = useStore((s) => s.running);
  const refreshRunning = useStore((s) => s.refreshRunning);
  const refreshInstances = useStore((s) => s.refreshInstances);
  const busy = useStore((s) => s.busy);
  const setBusy = useStore((s) => s.setBusy);
  const setProgress = useStore((s) => s.setProgress);
  const toast = useStore((s) => s.toast);
  const setPage = useStore((s) => s.setPage);
  const accounts = useStore((s) => s.accounts);
  const refreshAccounts = useStore((s) => s.refreshAccounts);

  const [showLog, setShowLog] = useState(false);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [dropdown, setDropdown] = useState(false);
  const [accDropdown, setAccDropdown] = useState(false);

  const selected = instances.find((i) => i.id === selectedId) ?? instances[0] ?? null;
  const isRunning = selected ? running.includes(selected.id) : false;
  const account = accounts.accounts.find((a) => a.id === accounts.activeId) ?? null;

  const pickAccount = async (id: string) => {
    sfx.click();
    await api.setActiveAccount(id);
    await refreshAccounts();
    setAccDropdown(false);
  };

  useEffect(() => {
    if (!selectedId && instances.length > 0) setSelectedId(instances[0].id);
  }, [instances, selectedId, setSelectedId]);

  useEffect(() => {
    const un: (() => void)[] = [];
    (async () => {
      un.push(
        await listen<LogLine>("launch://log", (e) => {
          if (!selected || e.payload.instanceId === selected.id) {
            setLogs((prev) => [...prev.slice(-2000), e.payload]);
          }
        })
      );
      un.push(
        await listen<{ instanceId: string; code: number }>("launch://exit", (e) => {
          refreshRunning();
          if (e.payload.code === 0) {
            sfx.exit();
          } else {
            toast(`Игра завершилась с кодом ${e.payload.code}. Загляни в лог или спроси Ямми ИИ.`, "error");
            setShowLog(true);
          }
        })
      );
    })();
    return () => un.forEach((u) => u());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected?.id]);

  const play = async () => {
    if (!selected) return;
    if (!account) {
      toast("Сначала добавь аккаунт 🍊", "error");
      setPage("accounts");
      return;
    }
    sfx.click();
    setBusy(selected.id);
    setLogs([]);
    try {
      await api.prepareInstance(selected.id);
      setProgress(null);
      await api.launchInstance(selected.id);
      await refreshRunning();
      await refreshInstances();
      setShowLog(true);
      sfx.launch();
      toast("Игра запущена! Удачной игры 🚀", "success");
    } catch (e) {
      toast(String(e), "error");
      setProgress(null);
    } finally {
      setBusy(null);
    }
  };

  const stop = async () => {
    if (!selected) return;
    sfx.click();
    await api.killGame(selected.id);
    await refreshRunning();
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 bg-grid">
      <div className="flex-1 flex flex-col items-center justify-center gap-6 p-8 min-h-0">
        <motion.div
          initial={{ opacity: 0, y: 14 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ type: "spring", stiffness: 200, damping: 20 }}
          className="text-center space-y-3"
        >
          <motion.div
            animate={{ y: [0, -8, 0], rotate: [0, 6, -4, 0] }}
            transition={{ duration: 4, repeat: Infinity, ease: "easeInOut" }}
          >
            <Emoji name="orange" size="xl" />
          </motion.div>
          <h1 className="font-display text-4xl tracking-wide">
            YAMMY<span className="text-gradient">LAUNCH</span>
          </h1>
          <p className="text-muted text-sm flex items-center justify-center gap-2">
            Твой мир. Твои правила. Твои моды. <Emoji name="sparkles" />
          </p>
        </motion.div>

        {instances.length === 0 ? (
          <motion.button
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            onClick={() => { sfx.click(); setPage("instances"); }}
            className="glass rounded-2xl px-8 py-6 flex flex-col items-center gap-3 hover:border-accent/40 transition-colors cursor-pointer"
          >
            <Emoji name="rocket" size="lg" />
            <div className="font-semibold">Создай свою первую сборку</div>
            <div className="text-sm text-muted">Vanilla, Fabric, Forge или OptiFine — на выбор</div>
          </motion.button>
        ) : (
          <>
            <div className="relative w-80">
              <button
                onClick={() => { sfx.click(); setDropdown(!dropdown); }}
                className="w-full glass rounded-xl px-4 py-3 flex items-center justify-between hover:border-accent/40 transition-colors cursor-pointer"
              >
                <div className="text-left">
                  <div className="font-semibold text-sm">{selected?.name}</div>
                  <div className="text-[11px] text-muted">
                    {selected && `${loaderLabel[selected.loader]} ${selected.gameVersion}`}
                    {selected?.loaderVersion && ` · ${selected.loaderVersion}`}
                    {selected?.optifineVersion && ` · OptiFine ${selected.optifineVersion}`}
                  </div>
                </div>
                <ChevronDown size={16} className={clsx("text-muted transition-transform", dropdown && "rotate-180")} />
              </button>
              {dropdown && (
                <div className="absolute top-full mt-2 w-full glass rounded-xl overflow-hidden z-10 max-h-64 overflow-y-auto">
                  {instances.map((i) => (
                    <button
                      key={i.id}
                      onClick={() => {
                        sfx.click();
                        setSelectedId(i.id);
                        setDropdown(false);
                      }}
                      className={clsx(
                        "w-full text-left px-4 py-2.5 hover:bg-panel-2 transition-colors text-sm cursor-pointer",
                        i.id === selected?.id && "bg-accent/10 text-accent"
                      )}
                    >
                      <div className="font-medium">{i.name}</div>
                      <div className="text-[11px] text-muted">
                        {loaderLabel[i.loader]} {i.gameVersion}
                        {i.optifineVersion ? " + OptiFine" : ""}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* выбор аккаунта */}
            <div className="relative w-80">
              <button
                onClick={() => { sfx.click(); setAccDropdown(!accDropdown); setDropdown(false); }}
                className="w-full glass rounded-xl px-4 py-2.5 flex items-center gap-3 hover:border-accent/40 transition-colors cursor-pointer"
              >
                {account ? (
                  <>
                    <img
                      src={skinHeadUrl(account)}
                      alt=""
                      className="w-8 h-8 rounded-md border border-border"
                      style={{ imageRendering: "pixelated" }}
                    />
                    <div className="text-left flex-1 min-w-0">
                      <div className="font-semibold text-sm truncate">{account.username}</div>
                      <div className="text-[11px] text-muted">
                        {account.kind === "msa" ? "Microsoft" : "Оффлайн"}
                      </div>
                    </div>
                  </>
                ) : (
                  <div className="text-left flex-1 text-sm text-muted">Выбери аккаунт…</div>
                )}
                <ChevronDown size={16} className={clsx("text-muted transition-transform", accDropdown && "rotate-180")} />
              </button>
              {accDropdown && (
                <div className="absolute top-full mt-2 w-full glass rounded-xl overflow-hidden z-20 max-h-64 overflow-y-auto">
                  {accounts.accounts.map((a) => (
                    <button
                      key={a.id}
                      onClick={() => pickAccount(a.id)}
                      className={clsx(
                        "w-full text-left px-4 py-2.5 hover:bg-panel-2 transition-colors text-sm cursor-pointer flex items-center gap-3",
                        a.id === accounts.activeId && "bg-accent/10 text-accent"
                      )}
                    >
                      <img
                        src={skinHeadUrl(a)}
                        alt=""
                        className="w-7 h-7 rounded-md border border-border"
                        style={{ imageRendering: "pixelated" }}
                      />
                      <div className="min-w-0">
                        <div className="font-medium truncate">{a.username}</div>
                        <div className="text-[11px] text-muted">{a.kind === "msa" ? "Microsoft" : "Оффлайн"}</div>
                      </div>
                    </button>
                  ))}
                  <button
                    onClick={() => { sfx.click(); setAccDropdown(false); setPage("accounts"); }}
                    className="w-full text-left px-4 py-2.5 hover:bg-panel-2 transition-colors text-sm text-accent cursor-pointer border-t border-border"
                  >
                    + Добавить аккаунт
                  </button>
                </div>
              )}
            </div>

            <motion.button
              whileHover={{ scale: busy || isRunning ? 1 : 1.03 }}
              whileTap={{ scale: busy || isRunning ? 1 : 0.97 }}
              onClick={isRunning ? stop : play}
              disabled={!!busy}
              className={clsx(
                "w-80 py-4 rounded-2xl font-display text-lg tracking-widest flex items-center justify-center gap-3 transition-colors cursor-pointer",
                isRunning
                  ? "bg-danger/90 hover:bg-danger text-white"
                  : "bg-gradient-to-r from-accent-dark via-accent-2 to-accent text-bg glow-accent",
                busy && "opacity-70 cursor-wait"
              )}
            >
              {busy ? (
                <>
                  <Loader2 size={22} className="animate-spin" />
                  ПОДГОТОВКА…
                </>
              ) : isRunning ? (
                <>
                  <Square size={20} />
                  ОСТАНОВИТЬ
                </>
              ) : (
                <>
                  <Play size={22} fill="currentColor" />
                  ИГРАТЬ
                </>
              )}
            </motion.button>

            {!account && (
              <div className="text-sm text-warn flex items-center gap-2">
                <Emoji name="warning" /> Нет аккаунта — добавь его в разделе «Аккаунты»
              </div>
            )}

            <div className="w-[560px] space-y-3">
              <ProgressBar />
              <div className="flex gap-2 justify-center">
                <button
                  onClick={() => { sfx.click(); setShowLog(!showLog); }}
                  className="text-xs text-muted hover:text-text px-3 py-1.5 rounded-lg hover:bg-panel-2 transition-colors cursor-pointer"
                >
                  {showLog ? "Скрыть лог" : "Показать лог"}
                </button>
                {selected && (
                  <button
                    onClick={() => { sfx.click(); api.openInstanceFolder(selected.id); }}
                    className="text-xs text-muted hover:text-text px-3 py-1.5 rounded-lg hover:bg-panel-2 transition-colors flex items-center gap-1.5 cursor-pointer"
                  >
                    <FolderOpen size={13} />
                    Папка сборки
                  </button>
                )}
                {selected && (
                  <button
                    onClick={() => { sfx.click(); setPage("ai"); }}
                    className="text-xs text-accent-soft hover:text-accent px-3 py-1.5 rounded-lg hover:bg-panel-2 transition-colors flex items-center gap-1.5 cursor-pointer"
                  >
                    <Emoji name="robot" />
                    Спросить Ямми
                  </button>
                )}
              </div>
            </div>
          </>
        )}
      </div>

      {showLog && selected && (
        <div className="h-56 shrink-0 p-4 pt-0 flex flex-col">
          <LogConsole lines={logs} />
        </div>
      )}
    </div>
  );
}