import { Page, useStore, activeAccount } from "../stores/app";
import { skinHeadUrl } from "../lib/tauri";
import Emoji, { EmojiName } from "./Emoji";
import { Plus } from "lucide-react";
import clsx from "clsx";

const items: { page: Page; label: string; emoji: EmojiName }[] = [
  { page: "play", label: "Играть", emoji: "game" },
  { page: "instances", label: "Сборки", emoji: "wrench" },
  { page: "packs", label: "Готовые сборки", emoji: "package" },
  { page: "mods", label: "Моды", emoji: "gem" },
  { page: "ai", label: "Ямми ИИ", emoji: "robot" },
  { page: "accounts", label: "Аккаунты", emoji: "users" },
  { page: "settings", label: "Настройки", emoji: "gear" },
];

export default function Sidebar() {
  const page = useStore((s) => s.page);
  const setPage = useStore((s) => s.setPage);
  const accounts = useStore((s) => s.accounts);
  const account = activeAccount();

  return (
    <div className="w-60 shrink-0 flex flex-col border-r border-border bg-bg-soft/70">
      <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
        {items.map(({ page: p, label, emoji }) => (
          <button
            key={p}
            onClick={() => setPage(p)}
            className={clsx(
              "w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all cursor-pointer",
              page === p
                ? "bg-accent/15 text-accent border border-accent/30 shadow-[0_0_16px_rgba(255,138,30,0.12)]"
                : "text-muted hover:text-text hover:bg-panel-2 border border-transparent"
            )}
          >
            <Emoji name={emoji} />
            {label}
            {p === "ai" && (
              <span className="ml-auto text-[9px] px-1.5 py-0.5 rounded-full bg-accent/20 text-accent-soft font-semibold">
                NEW
              </span>
            )}
          </button>
        ))}
      </nav>

      <div className="p-3 border-t border-border">
        {account ? (
          <button
            onClick={() => setPage("accounts")}
            className="w-full flex items-center gap-3 p-2 rounded-xl hover:bg-panel-2 transition-colors cursor-pointer"
          >
            <img
              src={skinHeadUrl(account)}
              alt=""
              className="w-9 h-9 rounded-lg border border-border"
              style={{ imageRendering: "pixelated" }}
            />
            <div className="text-left min-w-0">
              <div className="text-sm font-semibold truncate">{account.username}</div>
              <div className="text-[11px] text-muted">
                {account.kind === "msa" ? "Microsoft ✓" : "Оффлайн"}
              </div>
            </div>
          </button>
        ) : (
          <button
            onClick={() => setPage("accounts")}
            className="w-full flex items-center gap-2 p-2 rounded-xl text-muted hover:text-accent hover:bg-panel-2 transition-colors text-sm cursor-pointer"
          >
            <Plus size={16} />
            Добавить аккаунт
          </button>
        )}
        {accounts.accounts.length > 1 && (
          <div className="text-[10px] text-muted text-center mt-1">
            {accounts.accounts.length} аккаунта
          </div>
        )}
      </div>
    </div>
  );
}