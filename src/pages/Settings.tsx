import { useEffect, useState } from "react";
import { Save, FolderOpen, Loader2, Wand2 } from "lucide-react";
import { api, Settings, SystemInfo } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import Emoji from "../components/Emoji";

export default function SettingsPage() {
  const settings = useStore((s) => s.settings);
  const refreshSettings = useStore((s) => s.refreshSettings);
  const toast = useStore((s) => s.toast);
  const soundOn = useStore((s) => s.soundOn);
  const setSoundOn = useStore((s) => s.setSoundOn);
  const [form, setForm] = useState<Settings | null>(settings);
  const [dataDir, setDataDir] = useState("");
  const [saving, setSaving] = useState(false);
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);
  const [autoApplying, setAutoApplying] = useState(false);

  useEffect(() => {
    api.getSystemInfo().then(setSysInfo).catch(() => {});
  }, []);

  const autoTune = async () => {
    setAutoApplying(true);
    try {
      const rec = await api.applyRecommendedSettings();
      await refreshSettings();
      sfx.success();
      toast(`Настройки подобраны под твой ПК: ${rec.memoryMaxMb} МБ памяти ✨`, "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setAutoApplying(false);
    }
  };

  useEffect(() => {
    if (!settings) refreshSettings();
    else setForm(settings);
    api.getDataDir().then(setDataDir).catch(() => {});
  }, [settings, refreshSettings]);

  if (!form) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted">
        <Loader2 size={20} className="animate-spin" />
      </div>
    );
  }

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setForm({ ...form, [key]: value });

  const save = async () => {
    setSaving(true);
    try {
      await api.saveSettings(form);
      await refreshSettings();
      toast("Настройки сохранены ✨", "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-2xl space-y-6">
        <div>
          <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
            <Emoji name="gear" size="lg" /> НАСТРОЙКИ
          </h1>
          <p className="text-sm text-muted mt-1">Глобальные параметры — применяются ко всем сборкам, если в сборке не задано своё</p>
        </div>

        <section className="glass rounded-2xl p-5 space-y-4">
          <h2 className="font-semibold text-sm text-accent flex items-center gap-2"><Emoji name="zap" /> Java и память</h2>

          {sysInfo && (
            <div className="rounded-xl border border-accent/25 bg-accent/5 p-3 space-y-2">
              <div className="text-xs text-muted space-y-0.5">
                <div className="flex items-center gap-1.5"><Emoji name="bulb" /> Твой ПК:</div>
                <div>🧠 {sysInfo.cpuName} · {sysInfo.cpuThreads} потоков</div>
                <div>💾 {Math.round(sysInfo.ramTotalMb / 1024)} ГБ ОЗУ · 🎮 {sysInfo.gpuName}{sysInfo.gpuVramMb > 0 ? ` (${Math.round(sysInfo.gpuVramMb / 1024)} ГБ)` : ""}</div>
              </div>
              <button
                onClick={autoTune}
                disabled={autoApplying}
                className="w-full flex items-center justify-center gap-2 py-2 rounded-lg bg-gradient-to-r from-accent-dark to-accent text-bg text-sm font-semibold hover:opacity-90 disabled:opacity-60 transition-opacity cursor-pointer"
              >
                {autoApplying ? <Loader2 size={15} className="animate-spin" /> : <Wand2 size={15} />}
                Подобрать настройки под мой ПК
              </button>
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <Field label="Максимум памяти (МБ)">
              <input
                type="number"
                value={form.memoryMaxMb}
                onChange={(e) => set("memoryMaxMb", Number(e.target.value))}
                className={inputCls}
              />
            </Field>
            <Field label="Минимум памяти (МБ)">
              <input
                type="number"
                value={form.memoryMinMb}
                onChange={(e) => set("memoryMinMb", Number(e.target.value))}
                className={inputCls}
              />
            </Field>
          </div>
          <Field label="Свой путь к javaw.exe (пусто = автоскачивание нужной Java)">
            <input
              value={form.javaPathOverride ?? ""}
              onChange={(e) => set("javaPathOverride", e.target.value || null)}
              placeholder="C:\Program Files\Java\...\bin\javaw.exe"
              className={inputCls}
            />
          </Field>
          <Field label="Дополнительные JVM-аргументы">
            <input
              value={form.extraJvmArgs}
              onChange={(e) => set("extraJvmArgs", e.target.value)}
              placeholder="-XX:+UseG1GC -XX:+ParallelRefProcEnabled"
              className={inputCls}
            />
          </Field>
        </section>

        <section className="glass rounded-2xl p-5 space-y-4">
          <h2 className="font-semibold text-sm text-accent flex items-center gap-2"><Emoji name="game" /> Окно игры</h2>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Ширина">
              <input
                type="number"
                value={form.resolutionWidth}
                onChange={(e) => set("resolutionWidth", Number(e.target.value))}
                className={inputCls}
              />
            </Field>
            <Field label="Высота">
              <input
                type="number"
                value={form.resolutionHeight}
                onChange={(e) => set("resolutionHeight", Number(e.target.value))}
                className={inputCls}
              />
            </Field>
          </div>
          <Toggle
            label="Запускать в полный экран"
            checked={form.fullscreen}
            onChange={(v) => set("fullscreen", v)}
          />
        </section>

        <section className="glass rounded-2xl p-5 space-y-4">
          <h2 className="font-semibold text-sm text-accent flex items-center gap-2"><Emoji name="orange" /> Лаунчер</h2>
          <Toggle
            label="Показывать snapshot-версии в списке"
            checked={form.showSnapshots}
            onChange={(v) => set("showSnapshots", v)}
          />
          <Toggle
            label="Милые звуковые эффекты 🔊"
            checked={soundOn}
            onChange={setSoundOn}
          />
          <Field label="Папка данных (игры, Java, ассеты)">
            <div className="flex gap-2">
              <input
                value={form.dataDir ?? ""}
                onChange={(e) => set("dataDir", e.target.value || null)}
                placeholder={dataDir}
                className={inputCls + " flex-1"}
              />
              <button
                onClick={() => api.openDataFolder()}
                className="px-3 rounded-lg bg-panel-2 border border-border text-muted hover:text-text transition-colors cursor-pointer"
                title="Открыть папку"
              >
                <FolderOpen size={15} />
              </button>
            </div>
            <p className="text-[11px] text-muted mt-1">
              Сейчас: {dataDir}. Смена папки не переносит уже скачанные файлы.
            </p>
          </Field>
          <Field label="Microsoft Client ID (для продвинутых)">
            <input
              value={form.msaClientId}
              onChange={(e) => set("msaClientId", e.target.value)}
              className={inputCls}
            />
          </Field>
          <Field label="Ключ ИИ-помощника (пусто = встроенный)">
            <input
              type="password"
              value={form.aiApiKey ?? ""}
              onChange={(e) => set("aiApiKey", e.target.value || null)}
              placeholder="sk-… свой ключ api.reformboss.com"
              className={inputCls}
            />
            <p className="text-[11px] text-muted mt-1">
              Ямми работает из коробки. Свой ключ нужен, если хочешь использовать личный аккаунт ИИ-сервиса.
            </p>
          </Field>
        </section>

        <button
          onClick={save}
          disabled={saving}
          className="flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold hover:opacity-90 disabled:opacity-60 transition-opacity cursor-pointer"
        >
          {saving ? <Loader2 size={16} className="animate-spin" /> : <Save size={16} />}
          Сохранить
        </button>
      </div>
    </div>
  );
}

const inputCls =
  "w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block space-y-1.5">
      <span className="text-xs text-muted font-medium">{label}</span>
      {children}
    </label>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button onClick={() => { sfx.toggle(); onChange(!checked); }} className="flex items-center gap-3 w-full text-left cursor-pointer">
      <div
        className={
          "w-10 h-5 rounded-full transition-colors relative shrink-0 " +
          (checked ? "bg-accent-2" : "bg-panel-2 border border-border")
        }
      >
        <div
          className={
            "absolute top-0.5 w-4 h-4 rounded-full bg-white transition-all " +
            (checked ? "left-5" : "left-0.5")
          }
        />
      </div>
      <span className="text-sm">{label}</span>
    </button>
  );
}