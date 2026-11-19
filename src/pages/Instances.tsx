import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Plus, Trash2, FolderOpen, X, Loader2, Settings2, Check } from "lucide-react";
import { api, Instance, OptifineEntry, OptimizeModInfo } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import ProgressBar from "../components/ProgressBar";
import InstanceDetail from "./InstanceDetail";
import Emoji from "../components/Emoji";
import clsx from "clsx";

const loaderColors: Record<string, string> = {
  vanilla: "text-accent border-accent/30 bg-accent/10",
  fabric: "text-sky-400 border-sky-400/30 bg-sky-400/10",
  forge: "text-amber-300 border-amber-300/30 bg-amber-300/10",
  neoforge: "text-orange-400 border-orange-400/30 bg-orange-400/10",
};

const loaderNames: Record<string, string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
};

export default function InstancesPage() {
  const instances = useStore((s) => s.instances);
  const refreshInstances = useStore((s) => s.refreshInstances);
  const versions = useStore((s) => s.versions);
  const refreshVersions = useStore((s) => s.refreshVersions);
  const toast = useStore((s) => s.toast);
  const busy = useStore((s) => s.busy);
  const setPage = useStore((s) => s.setPage);
  const setSelectedId = useStore((s) => s.setSelectedInstanceId);

  const [creating, setCreating] = useState(false);
  const [detail, setDetail] = useState<Instance | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<Instance | null>(null);

  useEffect(() => {
    if (!versions) refreshVersions();
  }, [versions, refreshVersions]);

  if (detail) {
    return <InstanceDetail instance={detail} onBack={() => { setDetail(null); refreshInstances(); }} />;
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4 overflow-y-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
            <Emoji name="wrench" size="lg" /> СБОРКИ
          </h1>
          <p className="text-sm text-muted mt-1">Каждая сборка — изолированный мир со своими модами и настройками</p>
        </div>
        <button
          onClick={() => { sfx.click(); setCreating(true); }}
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold text-sm hover:opacity-90 transition-opacity cursor-pointer"
        >
          <Plus size={16} />
          Новая сборка
        </button>
      </div>

      {busy && <ProgressBar />}

      {instances.length === 0 && !busy ? (
        <div className="flex-1 flex flex-col items-center justify-center text-muted gap-3">
          <Emoji name="sleep" size="xl" />
          <div>Пока нет сборок. Создай первую!</div>
        </div>
      ) : (
        <div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
          <AnimatePresence>
            {instances.map((inst) => (
              <motion.div
                key={inst.id}
                layout
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95 }}
                className="glass rounded-2xl p-4 flex flex-col gap-3 hover:border-accent/30 transition-colors"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="font-semibold truncate">{inst.name}</div>
                    <div className="text-xs text-muted mt-0.5">Minecraft {inst.gameVersion}</div>
                  </div>
                  <div className="flex flex-col gap-1 items-end shrink-0">
                    <span className={clsx("text-[11px] px-2 py-1 rounded-md border font-medium", loaderColors[inst.loader])}>
                      {loaderNames[inst.loader]} {inst.loader !== "vanilla" ? (inst.loaderVersion ?? "") : ""}
                    </span>
                    {inst.optifineVersion && (
                      <span className="text-[10px] px-2 py-0.5 rounded-md border border-accent-soft/30 bg-accent-soft/10 text-accent-soft font-medium">
                        OptiFine {inst.optifineVersion}
                      </span>
                    )}
                  </div>
                </div>
                <div className="text-[11px] text-muted">
                  {inst.lastPlayed
                    ? `Играли: ${new Date(inst.lastPlayed).toLocaleString("ru-RU")}`
                    : "Ещё не запускалась"}
                </div>
                <div className="flex gap-1.5 mt-auto">
                  <button
                    onClick={() => { sfx.click(); setSelectedId(inst.id); setPage("play"); }}
                    className="flex-1 py-1.5 rounded-lg bg-accent/15 text-accent text-xs font-medium hover:bg-accent/25 transition-colors cursor-pointer"
                  >
                    Играть
                  </button>
                  <button
                    onClick={() => { sfx.click(); setDetail(inst); }}
                    className="p-1.5 rounded-lg bg-panel-2 text-muted hover:text-text transition-colors cursor-pointer"
                    title="Моды и настройки"
                  >
                    <Settings2 size={14} />
                  </button>
                  <button
                    onClick={() => api.openInstanceFolder(inst.id)}
                    className="p-1.5 rounded-lg bg-panel-2 text-muted hover:text-text transition-colors cursor-pointer"
                    title="Открыть папку"
                  >
                    <FolderOpen size={14} />
                  </button>
                  <button
                    onClick={() => setConfirmDelete(inst)}
                    className="p-1.5 rounded-lg bg-panel-2 text-muted hover:text-danger transition-colors cursor-pointer"
                    title="Удалить"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}

      {creating && (
        <CreateModal
          onClose={() => setCreating(false)}
          onCreated={async (inst) => {
            setCreating(false);
            await refreshInstances();
            setSelectedId(inst.id);
            sfx.pop();
            toast(`Сборка «${inst.name}» создана 🎉`, "success");
          }}
        />
      )}

      {confirmDelete && (
        <div className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm flex items-center justify-center">
          <div className="glass rounded-2xl p-6 w-96 space-y-4">
            <h2 className="font-semibold flex items-center gap-2"><Emoji name="trash" /> Удалить сборку?</h2>
            <p className="text-sm text-muted">
              «{confirmDelete.name}» будет удалена вместе с мирами, модами и настройками. Это необратимо.
            </p>
            <div className="flex gap-2">
              <button
                onClick={() => setConfirmDelete(null)}
                className="flex-1 py-2 rounded-lg bg-panel-2 hover:bg-border text-sm transition-colors cursor-pointer"
              >
                Отмена
              </button>
              <button
                onClick={async () => {
                  await api.deleteInstance(confirmDelete.id);
                  setConfirmDelete(null);
                  refreshInstances();
                  toast("Сборка удалена", "info");
                }}
                className="flex-1 py-2 rounded-lg bg-danger/80 hover:bg-danger text-white text-sm transition-colors cursor-pointer"
              >
                Удалить
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function CreateModal({ onClose, onCreated }: { onClose: () => void; onCreated: (i: Instance) => void }) {
  const versions = useStore((s) => s.versions);
  const settings = useStore((s) => s.settings);
  const toast = useStore((s) => s.toast);
  const setBusy = useStore((s) => s.setBusy);
  const [name, setName] = useState("");
  const [gameVersion, setGameVersion] = useState("");
  const [loader, setLoader] = useState<"vanilla" | "fabric" | "forge" | "neoforge">("vanilla");
  const [availLoaders, setAvailLoaders] = useState<string[]>(["vanilla"]);
  const [optifine, setOptifine] = useState(false);
  const [ofEntries, setOfEntries] = useState<OptifineEntry[]>([]);
  const [ofVersion, setOfVersion] = useState("");
  const [ofLoading, setOfLoading] = useState(false);
  const [optimize, setOptimize] = useState(false);
  const [optShaders, setOptShaders] = useState(false);
  const [optPack, setOptPack] = useState<OptimizeModInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [forgeLog, setForgeLog] = useState<string[]>([]);

  const showSnapshots = settings?.showSnapshots ?? false;
  const versionList = (versions?.versions ?? []).filter(
    (v) => v.type === "release" || (showSnapshots && v.type === "snapshot")
  );
  const effectiveVersion = gameVersion || versions?.latestRelease || "";

  useEffect(() => {
    const un: (() => void)[] = [];
    (async () => {
      un.push(await listen<string>("forge://log", (e) => {
        setForgeLog((prev) => [...prev.slice(-300), e.payload]);
      }));
    })();
    return () => un.forEach((u) => u());
  }, []);

  // Подгружаем доступные OptiFine при включении галочки
  useEffect(() => {
    if (!optifine || !effectiveVersion) {
      setOfEntries([]);
      return;
    }
    let cancelled = false;
    setOfLoading(true);
    api.optifineVersions(effectiveVersion)
      .then((list) => {
        if (!cancelled) {
          setOfEntries(list);
          setOfVersion(list[0] ? `${list[0].type}_${list[0].patch}` : "");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setOfEntries([]);
          toast(`OptiFine для ${effectiveVersion} не найден — включи «Оптимизацию» (Sodium и др.)`, "info");
          setOptifine(false);
        }
      })
      .finally(() => !cancelled && setOfLoading(false));
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [optifine, effectiveVersion]);

  // Список модов оптимизации для выбранного лоадера
  useEffect(() => {
    if (loader === "vanilla") {
      setOptPack([]);
      return;
    }
    api.getOptimizePack(loader).then((p) => setOptPack(p.mods)).catch(() => setOptPack([]));
  }, [loader]);

  // Какие лоадеры доступны для выбранной версии (NeoForge только с 1.20.2 и т.д.)
  useEffect(() => {
    if (!effectiveVersion) return;
    let cancelled = false;
    api.availableLoaders(effectiveVersion)
      .then((list) => {
        if (cancelled) return;
        setAvailLoaders(list);
        // если текущий лоадер недоступен для версии — сбрасываем на vanilla
        setLoader((cur) => (list.includes(cur) ? cur : "vanilla"));
      })
      .catch(() => !cancelled && setAvailLoaders(["vanilla", "fabric", "forge", "neoforge"]));
    return () => { cancelled = true; };
  }, [effectiveVersion]);

  const create = async () => {
    if (!effectiveVersion) {
      toast("Список версий ещё грузится, подожди", "error");
      return;
    }
    sfx.click();
    setCreating(true);
    setBusy("create");
    setForgeLog([]);
    try {
      const inst = await api.createInstance({
        name,
        gameVersion: effectiveVersion,
        loader,
        optifine: optifine && ofEntries.length > 0,
        optifineVersion: ofVersion || undefined,
      });
      // Пак оптимизации после создания сборки
      if (optimize && loader !== "vanilla") {
        try {
          const res = await api.installOptimizePack(inst.id, optShaders);
          if (res.skipped.length > 0) {
            toast(`Оптимизация: часть модов не имеет версии для ${effectiveVersion} (${res.skipped.join(", ")})`, "info");
          }
        } catch (e) {
          toast("Оптимизация не установлена: " + String(e), "error");
        }
      }
      onCreated(inst);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setCreating(false);
      setBusy(null);
    }
  };

  return (
    <div className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm flex items-center justify-center">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="glass rounded-2xl p-6 w-[500px] space-y-4 max-h-[88vh] overflow-y-auto"
      >
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold flex items-center gap-2"><Emoji name="rocket" /> Новая сборка</h2>
          <button onClick={onClose} disabled={creating} className="text-muted hover:text-text disabled:opacity-40 cursor-pointer">
            <X size={18} />
          </button>
        </div>

        <label className="block space-y-1.5">
          <span className="text-xs text-muted font-medium">Название</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Например: Выживание с друзьями"
            disabled={creating}
            className="w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors"
          />
        </label>

        <label className="block space-y-1.5">
          <span className="text-xs text-muted font-medium">Версия Minecraft</span>
          <select
            value={effectiveVersion}
            onChange={(e) => setGameVersion(e.target.value)}
            disabled={creating}
            className="w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors"
          >
            {versionList.map((v) => (
              <option key={v.id} value={v.id}>
                {v.id} {v.id === versions?.latestRelease ? "— последняя" : v.type === "snapshot" ? "(snapshot)" : ""}
              </option>
            ))}
          </select>
        </label>

        <div className="space-y-1.5">
          <span className="text-xs text-muted font-medium">Загрузчик модов</span>
          <div className="grid grid-cols-4 gap-2">
            {(["vanilla", "fabric", "forge", "neoforge"] as const).map((l) => {
              const available = availLoaders.includes(l);
              return (
                <button
                  key={l}
                  onClick={() => { if (available) { sfx.toggle(); setLoader(l); } }}
                  disabled={creating || !available}
                  title={available ? undefined : `${loaderNames[l]} не поддерживает ${effectiveVersion}`}
                  className={clsx(
                    "py-2.5 rounded-lg border text-sm font-medium transition-colors",
                    !available
                      ? "border-border/40 text-muted/30 cursor-not-allowed line-through"
                      : loader === l ? loaderColors[l] : "border-border text-muted hover:text-text hover:bg-panel-2 cursor-pointer"
                  )}
                >
                  {loaderNames[l]}
                </button>
              );
            })}
          </div>
          <p className="text-[11px] text-muted">
            {loader === "vanilla" && "Чистая игра. Можно добавить OptiFine для шейдеров и FPS."}
            {loader === "fabric" && "Лёгкий современный лоадер. Большинство модов на Modrinth — под Fabric."}
            {loader === "forge" && "Классический лоадер для больших модпаков. Установка займёт пару минут."}
            {loader === "neoforge" && "Наследник Forge для новых версий (1.20.2+). Сюда переехали Sodium и другие оптимизаторы."}
          </p>
        </div>

        <div className="space-y-2 rounded-xl border border-border p-3 bg-bg/40">
          <button
            onClick={() => { sfx.toggle(); setOptifine(!optifine); }}
            disabled={creating}
            className="flex items-center gap-3 w-full text-left cursor-pointer"
          >
            <div className={clsx(
              "w-10 h-5 rounded-full transition-colors relative shrink-0",
              optifine ? "bg-accent-2" : "bg-panel-2 border border-border"
            )}>
              <div className={clsx(
                "absolute top-0.5 w-4 h-4 rounded-full bg-white transition-all",
                optifine ? "left-5" : "left-0.5"
              )} />
            </div>
            <span className="text-sm font-medium flex items-center gap-2">
              <Emoji name="zap" /> OptiFine
              <span className="text-[11px] text-muted font-normal">шейдеры, зум, +FPS</span>
            </span>
          </button>
          {optifine && (
            ofLoading ? (
              <div className="text-xs text-muted flex items-center gap-2"><Loader2 size={13} className="animate-spin" /> Ищем версии OptiFine…</div>
            ) : ofEntries.length > 0 ? (
              <select
                value={ofVersion}
                onChange={(e) => setOfVersion(e.target.value)}
                disabled={creating}
                className="w-full bg-bg border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-accent/50"
              >
                {ofEntries.map((e) => (
                  <option key={e.filename} value={`${e.type}_${e.patch}`}>
                    {e.type}_{e.patch} {e.forge && e.forge !== "Forge N/A" ? `(${e.forge})` : ""}
                  </option>
                ))}
              </select>
            ) : null
          )}
          {optifine && loader === "fabric" && (
            <p className="text-[11px] text-muted">На Fabric OptiFine работает через OptiFabric — поставим автоматически.</p>
          )}
          {optifine && loader === "forge" && (
            <p className="text-[11px] text-muted">На Forge OptiFine ставится как мод. Выбирай версию, совместимую с Forge (указана в скобках).</p>
          )}
        </div>

        {loader !== "vanilla" && (
          <div className="space-y-2 rounded-xl border border-border p-3 bg-bg/40">
            <button
              onClick={() => { sfx.toggle(); setOptimize(!optimize); }}
              disabled={creating}
              className="flex items-center gap-3 w-full text-left cursor-pointer"
            >
              <div className={clsx(
                "w-10 h-5 rounded-full transition-colors relative shrink-0",
                optimize ? "bg-accent-2" : "bg-panel-2 border border-border"
              )}>
                <div className={clsx(
                  "absolute top-0.5 w-4 h-4 rounded-full bg-white transition-all",
                  optimize ? "left-5" : "left-0.5"
                )} />
              </div>
              <span className="text-sm font-medium flex items-center gap-2">
                <Emoji name="rocket" /> Оптимизация
                <span className="text-[11px] text-muted font-normal">+FPS, −память · работает на новых версиях</span>
              </span>
            </button>
            {optimize && (
              <>
                <div className="space-y-1">
                  {optPack.map((m) => (
                    <div key={m.slug} className="flex items-start gap-2 text-[11px]">
                      <Emoji name="zap" className="mt-0.5 shrink-0" />
                      <span><b className="text-text">{m.title}</b> <span className="text-muted">— {m.description}</span></span>
                    </div>
                  ))}
                </div>
                <button
                  onClick={() => { sfx.toggle(); setOptShaders(!optShaders); }}
                  disabled={creating}
                  className="flex items-center gap-2 text-xs text-muted hover:text-text cursor-pointer"
                >
                  <div className={clsx(
                    "w-4 h-4 rounded border flex items-center justify-center transition-colors",
                    optShaders ? "bg-accent-2 border-accent-2" : "border-border"
                  )}>
                    {optShaders && <Check size={11} className="text-bg" />}
                  </div>
                  Добавить поддержку шейдеров ({loader === "fabric" || loader === "neoforge" ? "Iris" : "Oculus"})
                </button>
                <p className="text-[11px] text-muted">
                  Моды подбираются под версию {effectiveVersion || "…"} автоматически. Если мода нет для этой версии — он пропускается.
                </p>
              </>
            )}
          </div>
        )}

        {creating && (loader === "forge" || loader === "neoforge") && forgeLog.length > 0 && (
          <div className="log-console bg-bg rounded-lg border border-border p-2 max-h-32 overflow-y-auto text-[11px] text-muted">
            {forgeLog.slice(-30).map((l, i) => (
              <div key={i} className="whitespace-pre-wrap break-all">{l}</div>
            ))}
          </div>
        )}

        <button
          onClick={create}
          disabled={creating}
          className="w-full py-3 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold flex items-center justify-center gap-2 hover:opacity-90 transition-opacity disabled:opacity-60 cursor-pointer"
        >
          {creating ? (
            <>
              <Loader2 size={18} className="animate-spin" />
              {loader === "forge" || loader === "neoforge" ? `Установка ${loaderNames[loader]}…` : "Создание…"}
            </>
          ) : (
            <>
              <Plus size={18} />
              Создать сборку
            </>
          )}
        </button>
      </motion.div>
    </div>
  );
}