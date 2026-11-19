import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Search, Download, Loader2, ChevronLeft, ChevronRight, CheckCircle2 } from "lucide-react";
import { api, formatDownloads, PackHit } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import ProgressBar from "../components/ProgressBar";
import Emoji from "../components/Emoji";
import clsx from "clsx";

export default function PacksPage() {
  const toast = useStore((s) => s.toast);
  const setProgress = useStore((s) => s.setProgress);
  const refreshInstances = useStore((s) => s.refreshInstances);
  const setSelectedId = useStore((s) => s.setSelectedInstanceId);

  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<PackHit[]>([]);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = async (q: string, off: number) => {
    setLoading(true);
    try {
      const res = await api.searchModpacks(q, off);
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
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => doSearch(query, 0), 400);
    return () => { if (timer.current) clearTimeout(timer.current); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query]);

  const install = async (hit: PackHit) => {
    sfx.click();
    setInstalling(hit.project_id);
    try {
      const inst = await api.installModpack(hit.project_id);
      setInstalled((prev) => new Set(prev).add(hit.project_id));
      await refreshInstances();
      setSelectedId(inst.id);
      sfx.pop();
      toast(`Сборка «${inst.name}» установлена! Можно играть 🎉`, "success");
      setProgress(null);
    } catch (e) {
      toast(String(e), "error");
      setProgress(null);
    } finally {
      setInstalling(null);
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4">
      <div>
        <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
          <Emoji name="package" size="lg" /> ГОТОВЫЕ СБОРКИ
        </h1>
        <p className="text-sm text-muted mt-1">
          Модпаки с Modrinth — скачиваются и устанавливаются целиком, со всеми модами и настройками
        </p>
      </div>

      <div className="relative">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Поиск сборок… например: fabulously optimized, create, skyfactory"
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
              <img src={hit.icon_url} alt="" className="w-14 h-14 rounded-xl border border-border shrink-0 object-cover" />
            ) : (
              <div className="w-14 h-14 rounded-xl bg-panel-2 flex items-center justify-center shrink-0">
                <Emoji name="package" size="lg" />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="font-semibold text-sm truncate">{hit.title}</span>
                <span className="text-[11px] text-muted shrink-0">by {hit.author}</span>
              </div>
              <div className="text-xs text-muted line-clamp-2 mt-0.5">{hit.description}</div>
              <div className="flex gap-2 mt-1.5 flex-wrap items-center">
                <span className="text-[10px] text-muted">⬇ {formatDownloads(hit.downloads)}</span>
                {(hit.display_categories.length ? hit.display_categories : hit.categories)
                  .slice(0, 4)
                  .map((c) => (
                    <span key={c} className="text-[10px] px-1.5 py-0.5 rounded bg-panel-2 text-muted">{c}</span>
                  ))}
                {hit.versions.length > 0 && (
                  <span className="text-[10px] text-accent-soft">MC {hit.versions[hit.versions.length - 1]}</span>
                )}
              </div>
            </div>
            <button
              onClick={() => install(hit)}
              disabled={installing !== null}
              className={clsx(
                "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors shrink-0 cursor-pointer",
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
              {installing === hit.project_id ? "Ставим…" : installed.has(hit.project_id) ? "Установлена" : "Установить"}
            </button>
          </motion.div>
        ))}
        {!loading && hits.length === 0 && (
          <div className="text-center text-muted text-sm py-10 flex flex-col items-center gap-2">
            <Emoji name="eyes" size="lg" /> Ничего не найдено
          </div>
        )}
      </div>

      {total > 20 && (
        <div className="flex items-center justify-center gap-3 shrink-0">
          <button
            disabled={offset === 0 || loading}
            onClick={() => doSearch(query, Math.max(0, offset - 20))}
            className="p-2 rounded-lg bg-panel-2 text-muted hover:text-text disabled:opacity-40 transition-colors cursor-pointer"
          >
            <ChevronLeft size={16} />
          </button>
          <span className="text-xs text-muted">
            {offset + 1}–{Math.min(offset + 20, total)} из {total}
          </span>
          <button
            disabled={offset + 20 >= total || loading}
            onClick={() => doSearch(query, offset + 20)}
            className="p-2 rounded-lg bg-panel-2 text-muted hover:text-text disabled:opacity-40 transition-colors cursor-pointer"
          >
            <ChevronRight size={16} />
          </button>
        </div>
      )}
    </div>
  );
}