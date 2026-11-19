import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { motion } from "framer-motion";
import { Copy, ExternalLink, Loader2, X } from "lucide-react";
import { api, DeviceCodeInfo } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import Emoji from "./Emoji";

export default function MsaLoginModal({ onClose }: { onClose: () => void }) {
  const [info, setInfo] = useState<DeviceCodeInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const [copied, setCopied] = useState(false);
  const toast = useStore((s) => s.toast);
  const refreshAccounts = useStore((s) => s.refreshAccounts);

  useEffect(() => {
    let sessionId: string | null = null;
    const unlisteners: (() => void)[] = [];
    (async () => {
      unlisteners.push(
        await listen<DeviceCodeInfo>("auth://device-code", (e) => {
          sessionId = e.payload.sessionId;
          setInfo(e.payload);
          openUrl(e.payload.verificationUri).catch(() => {});
        })
      );
      unlisteners.push(
        await listen("auth://success", async () => {
          setDone(true);
          sfx.success();
          await refreshAccounts();
          toast("Вход через Microsoft выполнен!", "success");
          setTimeout(onClose, 1500);
        })
      );
      unlisteners.push(
        await listen<{ sessionId: string; error: string }>("auth://error", (e) => {
          setError(e.payload.error);
        })
      );
      try {
        sessionId = await api.startMsaLogin();
      } catch (e) {
        setError(String(e));
      }
    })();
    return () => {
      unlisteners.forEach((u) => u());
      if (sessionId) api.cancelMsaLogin(sessionId).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const copy = async () => {
    if (!info) return;
    await navigator.clipboard.writeText(info.userCode);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm flex items-center justify-center">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="glass rounded-2xl p-6 w-[440px] space-y-4"
      >
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold flex items-center gap-2"><Emoji name="shield" /> Вход через Microsoft</h2>
          <button onClick={onClose} className="text-muted hover:text-text cursor-pointer">
            <X size={18} />
          </button>
        </div>

        {done ? (
          <div className="flex flex-col items-center py-6 gap-3">
            <Emoji name="party" size="xl" />
            <div className="text-sm text-muted">Готово! Аккаунт добавлен.</div>
          </div>
        ) : error ? (
          <div className="space-y-4">
            <div className="text-sm text-danger bg-danger/10 border border-danger/30 rounded-lg p-3">
              {error}
            </div>
            <button
              onClick={onClose}
              className="w-full py-2 rounded-lg bg-panel-2 hover:bg-border text-sm transition-colors"
            >
              Закрыть
            </button>
          </div>
        ) : info ? (
          <div className="space-y-4">
            <p className="text-sm text-muted">
              1. Откроется страница Microsoft (или перейдите по ссылке ниже).
              <br />
              2. Введите этот код:
            </p>
            <div className="flex items-center gap-2">
              <div className="flex-1 text-center text-2xl font-mono font-bold tracking-[0.3em] bg-bg rounded-lg py-3 border border-accent/30 text-accent">
                {info.userCode}
              </div>
              <button
                onClick={copy}
                className="p-3 rounded-lg bg-panel-2 hover:bg-border transition-colors cursor-pointer"
                title="Скопировать код"
              >
                {copied ? <Emoji name="check" /> : <Copy size={18} />}
              </button>
            </div>
            <button
              onClick={() => openUrl(info.verificationUri)}
              className="w-full flex items-center justify-center gap-2 py-2 rounded-lg bg-panel-2 hover:bg-border text-sm transition-colors"
            >
              <ExternalLink size={15} />
              {info.verificationUri}
            </button>
            <div className="flex items-center gap-2 text-sm text-muted justify-center">
              <Loader2 size={15} className="animate-spin" />
              Ожидаем подтверждение входа…
            </div>
          </div>
        ) : (
          <div className="flex items-center gap-2 text-sm text-muted justify-center py-6">
            <Loader2 size={16} className="animate-spin" />
            Запрашиваем код у Microsoft…
          </div>
        )}
      </motion.div>
    </div>
  );
}