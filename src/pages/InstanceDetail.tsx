import { useEffect, useState } from "react";
import { ArrowLeft, Settings2, Trash2, Power, FolderOpen, Search, Zap, Loader2 } from "lucide-react";
import { api, formatBytes, InstalledMod, Instance } from "../lib/tauri";
import { useStore } from "../stores/app";
import Emoji from "../components/Emoji";
import clsx from "clsx";

export default function InstanceDetail({ instance, onBack }: { instance: Instance; onBack: () => void }) {
  const [tab, setTab] = useState<"mods" | "settings">("mods");
  const [inst, setInst] = useState(instance);
  const toast = useStore((s) => s.toast);
  const setPage = useStore((s) => s.setPage);
  const setSelectedId = useStore((s) => s.setSelectedInstanceId);

  const save = async (patch: Partial<Instance>) => {
    const updated = { ...inst, ...patch };
    setInst(updated);
    try {
      await api.updateInstance(updated);
    } catch (e) {
      toast(String(e), "error");
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4">
      <div className="flex items-center gap-3">
        <button onClick={onBack} className="p-2 rounded-lg hover:bg-panel-2 text-muted hover:text-text transition-colors">
          <ArrowLeft size={18} />
        </button>
        <div className="flex-1">
          <h1 className="text-xl font-bold">{inst.name}</h1>
          <div className="text-xs text-muted">
            {inst.loader === "vanilla" ? "Vanilla" : `${inst.loader} ${inst.loaderVersion ?? ""}`} · Minecraft {inst.gameVersion}
          </div>
        </div>
        <button
          onClick={() => api.openInstanceFolder(inst.id)}
          className="flex items-center gap-2 px-3 py-2 rounded-lg bg-panel-2 text-sm text-muted hover:text-text transition-colors"
        >
          <FolderOpen size={15} />
          Папка
        </button>
      </div>

      <div className="flex gap-2">
        <button
          onClick={() => setTab("mods")}
          className={clsx(
            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors",
            tab === "mods" ? "bg-accent/15 text-accent" : "text-muted hover:text-text hover:bg-panel-2"
          )}
        >
          <Emoji name="gem" />
          Моды
        </button>
        <button
          onClick={() => setTab("settings")}
          className={clsx(
            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors",
            tab === "settings" ? "bg-accent/15 text-accent" : "text-muted hover:text-text hover:bg-panel-2"
          )}
        >
          <Settings2 size={15} />
          Настройки
        </button>
      </div>

      {tab === "mods" ? (
        <ModsTab instance={inst} onBrowse={() => { setSelectedId(inst.id); setPage("mods"); }} />
      ) : (
        <div className="glass rounded-xl p-5 space-y-4 max-w-xl overflow-y-auto">
          <NumField
            label="Максимум памяти (МБ)"
            value={inst.memoryMaxMb}
            placeholder="Глобальная настройка"
            onChange={(v) => save({ memoryMaxMb: v })}
          />
          <NumField
            label="Минимум памяти (МБ)"
            value={inst.memoryMinMb}
            placeholder="Глобальная настройка"
            onChange={(v) => save({ memoryMinMb: v })}
          />
          <NumField
            label="Ширина окна"
            value={inst.resolutionWidth}
            placeholder="Глобальная настройка"
            onChange={(v) => save({ resolutionWidth: v })}
          />
          <NumField
            label="Высота окна"
            value={inst.resolutionHeight}
            placeholder="Глобальная настройка"
            onChange={(v) => save({ resolutionHeight: v })}
          />
          <label className="block space-y-1.5">
            <span className="text-xs text-muted font-medium">Свой путь к javaw.exe (необязательно)</span>
            <input
              defaultValue={inst.javaPath ?? ""}
              onBlur={(e) => save({ javaPath: e.target.value || null })}
              placeholder="Автоопределение / автоскачивание"
              className="w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors"
            />
          </label>
          <p className="text-[11px] text-muted">
            Пустые поля используют глобальные настройки из раздела «Настройки».
          </p>
        </div>
      )}
    </div>
  );
}

function NumField({
  label,
  value,
  placeholder,
  onChange,
}: {
  label: string;
  value?: number | null;
  placeholder: string;
  onChange: (v: number | null) => void;
}) {
  return (
    <label className="block space-y-1.5">
      <span className="text-xs text-muted font-medium">{label}</span>
      <input
        type="number"
        defaultValue={value ?? ""}
        onBlur={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
        placeholder={placeholder}
        className="w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors"
      />
    </label>
  );
}

function OptimizeButton({ instance, onDone }: { instance: Instance; onDone: () => void }) {
  const toast = useStore((s) => s.toast);
  const [busy, setBusy] = useState(false);
  const run = async () => {
    setBusy(true);
    try {
      const res = await api.installOptimizePack(instance.id, true);
      if (res.installed.length > 0) {
        toast(`Оптимизация установлена: ${res.installed.length} мод(ов) 🚀`, "success");
      }
      if (res.skipped.length > 0) {
        toast(`Пропущены (нет версии для ${instance.gameVersion}): ${res.skipped.join(", ")}`, "info");
      }
      if (res.installed.length === 0 && res.skipped.length === 0) {
        toast("Все моды оптимизации уже стоят", "info");
      }
      onDone();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <button
      onClick={run}
      disabled={busy}
      className="flex items-center gap-2 px-3 py-2 rounded-lg bg-panel-2 border border-border text-sm font-medium text-accent-soft hover:border-accent/40 transition-colors disabled:opacity-60 cursor-pointer"
      title="Sodium/Lithium/Iris и др. — буст FPS и меньше памяти"
    >
      {busy ? <Loader2 size={15} className="animate-spin" /> : <Zap size={15} />}
      Оптимизировать
    </button>
  );
}

function ModsTab({ instance, onBrowse }: { instance: Instance; onBrowse: () => void }) {
  const [mods, setMods] = useState<InstalledMod[]>([]);
  const toast = useStore((s) => s.toast);

  const refresh = async () => {
    try {
      setMods(await api.listMods(instance.id));
    } catch (e) {
      toast(String(e), "error");
    }
  };
  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instance.id]);

  if (instance.loader === "vanilla") {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-muted gap-2">
        <Emoji name="sleep" size="xl" />
        <div>Это Vanilla-сборка — моды не поддерживаются.</div>
        <div className="text-sm">Создай сборку с Fabric или Forge, чтобы ставить моды.</div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 gap-3">
      <div className="flex justify-between items-center">
        <div className="text-sm text-muted">{mods.length} мод(ов) установлено</div>
        <div className="flex gap-2">
          <OptimizeButton instance={instance} onDone={refresh} />
          <button
            onClick={onBrowse}
            className="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent/15 text-accent text-sm font-medium hover:bg-accent/25 transition-colors cursor-pointer"
          >
            <Search size={15} />
            Найти моды
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto space-y-2 min-h-0">
        {mods.length === 0 ? (
          <div className="text-muted text-sm text-center py-10">
            Модов пока нет. Нажмите «Найти моды», чтобы открыть каталог Modrinth.
          </div>
        ) : (
          mods.map((m) => {
            const disabled = m.fileName.endsWith(".disabled");
            return (
              <div key={m.fileName} className={clsx("glass rounded-lg px-4 py-3 flex items-center gap-3", disabled && "opacity-50")}>
                <Emoji name="gem" className="shrink-0" />
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium truncate">{m.title ?? m.fileName}</div>
                  <div className="text-[11px] text-muted truncate">
                    {m.fileName} · {formatBytes(m.size)}
                  </div>
                </div>
                <button
                  onClick={async () => {
                    await api.toggleMod(instance.id, m.fileName);
                    refresh();
                  }}
                  className={clsx(
                    "p-2 rounded-lg transition-colors",
                    disabled ? "text-muted hover:text-accent bg-panel-2" : "text-accent hover:text-warn bg-panel-2"
                  )}
                  title={disabled ? "Включить" : "Отключить"}
                >
                  <Power size={15} />
                </button>
                <button
                  onClick={async () => {
                    await api.removeMod(instance.id, m.fileName);
                    refresh();
                    toast("Мод удалён", "info");
                  }}
                  className="p-2 rounded-lg bg-panel-2 text-muted hover:text-danger transition-colors"
                  title="Удалить"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}