import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Plus, Trash2, Check, X, Loader2 } from "lucide-react";
import { api, skinHeadUrl } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import MsaLoginModal from "../components/MsaLoginModal";
import Emoji from "../components/Emoji";
import clsx from "clsx";

export default function AccountsPage() {
  const accounts = useStore((s) => s.accounts);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const toast = useStore((s) => s.toast);

  const [msaModal, setMsaModal] = useState(false);
  const [offlineModal, setOfflineModal] = useState(false);
  const [nickname, setNickname] = useState("");
  const [adding, setAdding] = useState(false);

  const addOffline = async () => {
    setAdding(true);
    try {
      await api.addOfflineAccount(nickname);
      await refreshAccounts();
      setOfflineModal(false);
      setNickname("");
      sfx.pop();
      toast("Оффлайн-аккаунт добавлен 🍊", "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setAdding(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4 overflow-y-auto">
      <div>
        <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
          <Emoji name="users" size="lg" /> АККАУНТЫ
        </h1>
        <p className="text-sm text-muted mt-1">
          Microsoft — для лицензии и официальных серверов. Оффлайн — для одиночной игры и пиратских серверов.
        </p>
      </div>

      <div className="flex gap-3">
        <button
          onClick={() => { sfx.click(); setMsaModal(true); }}
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold text-sm hover:opacity-90 transition-opacity cursor-pointer"
        >
          <Emoji name="shield" />
          Войти через Microsoft
        </button>
        <button
          onClick={() => { sfx.click(); setOfflineModal(true); }}
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-panel-2 border border-border text-sm font-medium hover:bg-border transition-colors cursor-pointer"
        >
          <Emoji name="key" />
          Оффлайн-аккаунт
        </button>
      </div>

      <div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
        <AnimatePresence>
          {accounts.accounts.map((a) => {
            const active = a.id === accounts.activeId;
            return (
              <motion.div
                key={a.id}
                layout
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95 }}
                className={clsx(
                  "glass rounded-2xl p-4 flex items-center gap-4 transition-colors",
                  active && "border-accent/50 glow-accent"
                )}
              >
                <img
                  src={skinHeadUrl(a)}
                  alt=""
                  className="w-14 h-14 rounded-xl border border-border"
                  style={{ imageRendering: "pixelated" }}
                />
                <div className="flex-1 min-w-0">
                  <div className="font-semibold truncate">{a.username}</div>
                  <div className="text-xs text-muted flex items-center gap-1.5 mt-0.5">
                    {a.kind === "msa" ? (
                      <><Emoji name="shield" /> Microsoft</>
                    ) : (
                      <><Emoji name="key" /> Оффлайн</>
                    )}
                  </div>
                  <div className="flex gap-1.5 mt-2">
                    {!active && (
                      <button
                        onClick={async () => {
                          sfx.click();
                          await api.setActiveAccount(a.id);
                          refreshAccounts();
                        }}
                        className="text-xs px-2.5 py-1 rounded-md bg-accent/15 text-accent hover:bg-accent/25 transition-colors cursor-pointer"
                      >
                        Выбрать
                      </button>
                    )}
                    {active && (
                      <span className="text-xs px-2.5 py-1 rounded-md bg-accent/15 text-accent flex items-center gap-1">
                        <Check size={12} /> Активен
                      </span>
                    )}
                    <button
                      onClick={async () => {
                        await api.removeAccount(a.id);
                        refreshAccounts();
                        toast("Аккаунт удалён", "info");
                      }}
                      className="text-xs px-2 py-1 rounded-md bg-panel-2 text-muted hover:text-danger transition-colors cursor-pointer"
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>

      {accounts.accounts.length === 0 && (
        <div className="text-center text-muted text-sm py-10 flex flex-col items-center gap-2">
          <Emoji name="sleep" size="xl" />
          Аккаунтов пока нет. Добавь Microsoft-аккаунт или создай оффлайн-профиль.
        </div>
      )}

      {msaModal && <MsaLoginModal onClose={() => setMsaModal(false)} />}

      {offlineModal && (
        <div className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm flex items-center justify-center">
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="glass rounded-2xl p-6 w-96 space-y-4"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold flex items-center gap-2"><Emoji name="key" /> Оффлайн-аккаунт</h2>
              <button onClick={() => setOfflineModal(false)} className="text-muted hover:text-text cursor-pointer">
                <X size={18} />
              </button>
            </div>
            <p className="text-xs text-muted">
              Ник от 3 до 16 символов: латинские буквы, цифры и _. Лицензионные сервера с таким аккаунтом не работают.
            </p>
            <input
              value={nickname}
              onChange={(e) => setNickname(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && nickname && addOffline()}
              placeholder="Steve"
              autoFocus
              className="w-full bg-bg border border-border rounded-lg px-3 py-2.5 text-sm outline-none focus:border-accent/50 transition-colors"
            />
            <button
              onClick={addOffline}
              disabled={!nickname || adding}
              className="w-full py-2.5 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold text-sm flex items-center justify-center gap-2 hover:opacity-90 disabled:opacity-50 transition-opacity cursor-pointer"
            >
              {adding ? <Loader2 size={16} className="animate-spin" /> : <Plus size={16} />}
              Добавить
            </button>
          </motion.div>
        </div>
      )}
    </div>
  );
}