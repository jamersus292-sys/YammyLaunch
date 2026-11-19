import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Upload, Trash2, Loader2 } from "lucide-react";
import { Account, api } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import Emoji from "./Emoji";
import clsx from "clsx";

export default function SkinManager({ account }: { account: Account }) {
  const toast = useStore((s) => s.toast);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const [skinPreview, setSkinPreview] = useState<string | null>(null);
  const [capePreview, setCapePreview] = useState<string | null>(null);
  const [model, setModel] = useState(account.skinModel ?? "classic");
  const [busy, setBusy] = useState<"skin" | "cape" | null>(null);

  const loadPreviews = async () => {
    try {
      setSkinPreview(await api.skinPreview(account.id));
      setCapePreview(await api.capePreview(account.id));
    } catch {
      /* ignore */
    }
  };

  useEffect(() => {
    loadPreviews();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [account.id]);

  const pickFile = async (kind: "skin" | "cape") => {
    sfx.click();
    const path = await open({
      multiple: false,
      filters: [{ name: "PNG скин", extensions: ["png"] }],
    });
    if (!path || typeof path !== "string") return;
    setBusy(kind);
    try {
      const resp = await fetch(convertFileSrc(path));
      const buf = await resp.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      if (kind === "skin") {
        await api.setSkin(account.id, bytes, model);
        toast("Скин установлен! Он появится в игре при следующем запуске 🎨", "success");
      } else {
        await api.setCape(account.id, bytes);
        toast("Плащ установлен! 🧥", "success");
      }
      await loadPreviews();
      await refreshAccounts();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const remove = async (kind: "skin" | "cape") => {
    sfx.click();
    try {
      if (kind === "skin") {
        await api.removeSkin(account.id);
        setSkinPreview(null);
      } else {
        await api.removeCape(account.id);
        setCapePreview(null);
      }
      await refreshAccounts();
      toast(kind === "skin" ? "Скин удалён" : "Плащ удалён", "info");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  return (
    <div className="space-y-3 rounded-xl border border-border bg-bg/40 p-3">
      <div className="text-xs font-medium text-muted flex items-center gap-1.5">
        <Emoji name="sparkles" /> Скин и плащ (видны в игре на оффлайн-аккаунте)
      </div>

      <div className="flex gap-4">
        {/* скин */}
        <div className="flex-1 space-y-2">
          <div className="text-[11px] text-muted">Скин</div>
          <div className="flex items-center gap-2">
            <div className="w-16 h-16 rounded-lg border border-border bg-panel-2 flex items-center justify-center overflow-hidden">
              {skinPreview ? (
                <img src={skinPreview} alt="скин" className="w-full h-full object-contain" style={{ imageRendering: "pixelated" }} />
              ) : (
                <Emoji name="sleep" />
              )}
            </div>
            <div className="flex flex-col gap-1.5">
              <button
                onClick={() => pickFile("skin")}
                disabled={busy !== null}
                className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-lg bg-accent/15 text-accent hover:bg-accent/25 transition-colors cursor-pointer disabled:opacity-50"
              >
                {busy === "skin" ? <Loader2 size={12} className="animate-spin" /> : <Upload size={12} />}
                Загрузить PNG
              </button>
              {skinPreview && (
                <button
                  onClick={() => remove("skin")}
                  className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-lg bg-panel-2 text-muted hover:text-danger transition-colors cursor-pointer"
                >
                  <Trash2 size={12} /> Убрать
                </button>
              )}
            </div>
          </div>
          <div className="flex gap-1.5">
            {(["classic", "slim"] as const).map((m) => (
              <button
                key={m}
                onClick={async () => {
                  sfx.toggle();
                  setModel(m);
                  if (skinPreview) {
                    // переустанавливаем с новой моделью не нужно — модель применится при следующем запуске
                  }
                }}
                className={clsx(
                  "text-[11px] px-2 py-1 rounded-md border transition-colors cursor-pointer",
                  model === m ? "border-accent/40 bg-accent/10 text-accent" : "border-border text-muted hover:text-text"
                )}
              >
                {m === "classic" ? "Classic (4px)" : "Slim (3px)"}
              </button>
            ))}
          </div>
        </div>

        {/* плащ */}
        <div className="flex-1 space-y-2">
          <div className="text-[11px] text-muted">Плащ</div>
          <div className="flex items-center gap-2">
            <div className="w-16 h-16 rounded-lg border border-border bg-panel-2 flex items-center justify-center overflow-hidden">
              {capePreview ? (
                <img src={capePreview} alt="плащ" className="w-full h-full object-contain" style={{ imageRendering: "pixelated" }} />
              ) : (
                <Emoji name="sleep" />
              )}
            </div>
            <div className="flex flex-col gap-1.5">
              <button
                onClick={() => pickFile("cape")}
                disabled={busy !== null}
                className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-lg bg-accent/15 text-accent hover:bg-accent/25 transition-colors cursor-pointer disabled:opacity-50"
              >
                {busy === "cape" ? <Loader2 size={12} className="animate-spin" /> : <Upload size={12} />}
                Загрузить PNG
              </button>
              {capePreview && (
                <button
                  onClick={() => remove("cape")}
                  className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-lg bg-panel-2 text-muted hover:text-danger transition-colors cursor-pointer"
                >
                  <Trash2 size={12} /> Убрать
                </button>
              )}
            </div>
          </div>
          <p className="text-[10px] text-muted">PNG 64×32 или 64×64</p>
        </div>
      </div>
      <p className="text-[10px] text-muted">
        Скин и плащ применяются при запуске игры через встроенный скин-сервер. На других игроках оффлайн-серверов видно не будет — только у тебя.
      </p>
    </div>
  );
}