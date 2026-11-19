import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Search, Download, Loader2, ChevronLeft, ChevronRight, CheckCircle2, X } from "lucide-react";
import { api, formatDownloads, Instance, SearchHit } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import ProgressBar from "../components/ProgressBar";
import Emoji from "../components/Emoji";
import clsx from "clsx";

const loaderLabel: Record<string, string> = {
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
};

export default function ModBrowser() {
  const instances = useStore((s) => s.instances);
  const selectedId = useStore((s) => s.selectedInstanceId);
  const setSelectedId = useStore((s) => s.setSelectedInstanceId);
  const toast = useStore((s) => s.toast);
  const setProgress = useStore((s) => s.setProgress);

  const moddable = instances.filter((i) => i.loader !== "vanilla");
  const instance = moddable.find((i) => i.id === selectedId) ?? moddable[0] ?? null;

  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [pickFor, setPickFor] = useState<SearchHit | null>(null);
  const searchTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = async (q: string, off: number) => {
    if (!instance) return;
    setLoading(true);
    try {
      const res = await api.searchMods(q, instance.gameVersion, instance.loader, off);
      setHits(res.hits);
      setTotal(res.total_hits);
      setOffset(off);
    } catch (e) {
      toast("Ошибка поиска: " + String(e), "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!instance) return;
    if (searchTimer.current) clearTimeout(searchTimer.current);
    searchTimer.current = setTimeout(() => doSearch(query, 0), 400);
    return () => {
      if (searchTimer.current) clearTimeout(searchTimer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, instance?.id]);

  const install = async (hit: SearchHit, target?: Instance) => {
    const inst = target ?? instance;
    if (!inst) return;
    sfx.click();
    setPickFor(null);
    setInstalling(hit.project_id);
    try {
      const mods = await api.installMod(inst.id, hit.project_id);
      setInstalled((prev) => new Set(prev).add(hit.project_id));
      sfx.pop();
      toast(
        mods.length > 1
          ? `«${hit.title}» установлен в «${inst.name}» (+${mods.length - 1} зависимостей)`
          : `«${hit.title}» установлен в «${inst.name}»`,
        "success"
      );
      setProgress(null);
    } catch (e) {
      toast(String(e), "error");
      setProgress(null);
    } finally {
      setInstalling(null);
    }
  };

  const onInstallClick = (hit: SearchHit) => {
    // если сборок несколько — спрашиваем, куда ставить
    const compatible = moddable.filter(
      (i) => hit.versions.includes(i.gameVersion) || hit.versions.length === 0
    );
    const candidates = compatible.length > 0 ? compatible : moddable;
    if (candidates.length > 1) {
      sfx.click();
      setPickFor(hit);
    } else {
      install(hit, candidates[0] ?? instance ?? undefined);
    }
  };

  if (moddable.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-muted gap-3 p-6">
        <Emoji name="gem" size="xl" />
        <div className="text-center">
          <div className="font-medium text-text">Нужна сборка с Fabric или Forge</div>
          <div className="text-sm mt-1">Создай её в разделе «Сборки», и здесь появится каталог модов.</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
            <Emoji name="gem" size="lg" /> МОДЫ
          </h1>
          <p className="text-sm text-muted mt-1">Каталог Modrinth · зависимости ставятся автоматически</p>
        </div>
        <select
          value={instance?.id ?? ""}
          onChange={(e) => setSelectedId(e.target.value)}
          className="bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 min-w-56"
        >
          {moddable.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name} ({i.loader} {i.gameVersion})
            </option>
          ))}
        </select>
      </div>

      <div className="relative">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={`Поиск модов для ${instance?.gameVersion} (${instance?.loader})… например, sodium`}
          className="w-full bg-bg border border-border rounded-xl pl-10 pr-4 py-3 text-sm outline-none focus:border-accent/50 transition-colors"
        />
        {loading && <Loader2 size={16} className="absolute right-3 top-1/2 -translate-y-1/2 text-muted animate-spin" />}
      </div>

      <ProgressBar />

      <div className="flex-1 overflow-y-auto min-h-0 space-y-2">
        {hits.map((hit) => (
          <motion.div
            key={hit.project_id}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            className="glass rounded-xl p-4 flex items-center gap-4 hover:border-accent/30 transition-colors"
          >
            {hit.icon_url ? (
              <img src={hit.icon_url} alt="" className="w-12 h-12 rounded-lg border border-border shrink-0 object-cover" />
            ) : (
              <div className="w-12 h-12 rounded-lg bg-panel-2 flex items-center justify-center shrink-0">
                <Emoji name="gem" size="lg" />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="font-semibold text-sm truncate">{hit.title}</span>
                <span className="text-[11px] text-muted shrink-0">by {hit.author}</span>
              </div>
              <div className="text-xs text-muted truncate mt-0.5">{hit.description}</div>
              <div className="flex gap-2 mt-1.5 flex-wrap">
                <span className="text-[10px] text-muted">⬇ {formatDownloads(hit.downloads)}</span>
                {(hit.display_categories.length ? hit.display_categories : hit.categories)
                  .filter((c) => !["fabric", "forge", "neoforge"].includes(c))
                  .slice(0, 4)
                  .map((c) => (
                    <span key={c} className="text-[10px] px-1.5 py-0.5 rounded bg-panel-2 text-muted">
                      {c}
                    </span>
                  ))}
              </div>
            </div>
            <button
              onClick={() => onInstallClick(hit)}
              disabled={installing !== null}
              className={clsx(
                "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors shrink-0",
                installed.has(hit.project_id)
                  ? "bg-accent/15 text-accent"
                  : "bg-accent/15 text-accent hover:bg-accent/25",
                installing !== null && "opacity-60"
              )}
            >
              {installing === hit.project_id ? (
                <Loader2 size={15} className="animate-spin" />
              ) : installed.has(hit.project_id) ? (
                <CheckCircle2 size={15} />
              ) : (
                <Download size={15} />
              )}
              {installed.has(hit.project_id) ? "Установлен" : "Установить"}
            </button>
          </motion.div>
        ))}
        {!loading && hits.length === 0 && (
          <div className="text-center text-muted text-sm py-10">Ничего не найдено</div>
        )}
      </div>

      {total > 20 && (
        <div className="flex items-center justify-center gap-3 shrink-0">
          <button
            disabled={offset === 0 || loading}
            onClick={() => doSearch(query, Math.max(0, offset - 20))}
            className="p-2 rounded-lg bg-panel-2 text-muted hover:text-text disabled:opacity-40 transition-colors"
          >
            <ChevronLeft size={16} />
          </button>
          <span className="text-xs text-muted">
            {offset + 1}–{Math.min(offset + 20, total)} из {total}
          </span>
          <button
            disabled={offset + 20 >= total || loading}
            onClick={() => doSearch(query, offset + 20)}
            className="p-2 rounded-lg bg-panel-2 text-muted hover:text-text disabled:opacity-40 transition-colors"
          >
            <ChevronRight size={16} />
          </button>
        </div>
      )}

      <AnimatePresence>
        {pickFor && (
          <div className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm flex items-center justify-center">
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              className="glass rounded-2xl p-6 w-[440px] space-y-4 max-h-[80vh] overflow-y-auto"
            >
              <div className="flex items-center justify-between">
                <h2 className="font-semibold flex items-center gap-2">
                  <Emoji name="package" /> Куда установить «{pickFor.title}»?
                </h2>
                <button onClick={() => setPickFor(null)} className="text-muted hover:text-text cursor-pointer">
                  <X size={18} />
                </button>
              </div>
              <div className="space-y-2">
                {moddable.map((i) => {
                  const compatible = pickFor.versions.includes(i.gameVersion);
                  return (
                    <button
                      key={i.id}
                      onClick={() => install(pickFor, i)}
                      className={clsx(
                        "w-full text-left px-4 py-3 rounded-xl border transition-colors cursor-pointer",
                        compatible
                          ? "border-border hover:border-accent/50 hover:bg-panel-2"
                          : "border-border/40 opacity-50 hover:opacity-80"
                      )}
                    >
                      <div className="font-medium text-sm">{i.name}</div>
                      <div className="text-[11px] text-muted">
                        {loaderLabel[i.loader]} {i.gameVersion}
                        {!compatible && " · мод может не подойти под эту версию"}
                      </div>
                    </button>
                  );
                })}
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}