import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Send, Loader2, Trash2 } from "lucide-react";
import { api, ChatMessage } from "../lib/tauri";
import { useStore } from "../stores/app";
import { sfx } from "../lib/sounds";
import Emoji from "../components/Emoji";
import clsx from "clsx";

const SUGGESTIONS = [
  "Как поднять FPS в Minecraft?",
  "Какие моды поставить для выживания?",
  "Игра крашится с кодом 1 — что делать?",
  "Чем Fabric отличается от Forge?",
  "Как поставить шейдеры?",
];

export default function AiPage() {
  const toast = useStore((s) => s.toast);
  const [messages, setMessages] = useState<ChatMessage[]>(() => {
    try {
      return JSON.parse(localStorage.getItem("yammy-chat") ?? "[]");
    } catch {
      return [];
    }
  });
  const [input, setInput] = useState("");
  const [thinking, setThinking] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    localStorage.setItem("yammy-chat", JSON.stringify(messages.slice(-60)));
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, thinking]);

  const send = async (text?: string) => {
    const content = (text ?? input).trim();
    if (!content || thinking) return;
    sfx.click();
    const next: ChatMessage[] = [...messages, { role: "user", content }];
    setMessages(next);
    setInput("");
    setThinking(true);
    try {
      const reply = await api.aiChat(next);
      sfx.message();
      setMessages([...next, { role: "assistant", content: reply }]);
    } catch (e) {
      toast(String(e), "error");
      setMessages([...next, { role: "assistant", content: "Ой, не получилось ответить 😿 Попробуй ещё раз чуть позже." }]);
    } finally {
      setThinking(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 p-6 gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl tracking-wide flex items-center gap-2">
            <Emoji name="robot" size="lg" /> ЯММИ ИИ
          </h1>
          <p className="text-sm text-muted mt-1">
            Помощник по Minecraft: моды, ошибки, FPS, шейдеры. Спрашивай что угодно 🍊
          </p>
        </div>
        {messages.length > 0 && (
          <button
            onClick={() => { sfx.click(); setMessages([]); }}
            className="flex items-center gap-2 px-3 py-2 rounded-lg bg-panel-2 text-muted hover:text-danger text-sm transition-colors cursor-pointer"
            title="Очистить диалог"
          >
            <Trash2 size={14} />
            Очистить
          </button>
        )}
      </div>

      <div ref={scrollRef} className="flex-1 overflow-y-auto min-h-0 space-y-3 pr-1">
        {messages.length === 0 && (
          <div className="h-full flex flex-col items-center justify-center gap-5">
            <motion.div
              animate={{ y: [0, -6, 0] }}
              transition={{ duration: 3, repeat: Infinity, ease: "easeInOut" }}
            >
              <Emoji name="robot" size="xl" />
            </motion.div>
            <div className="text-center">
              <div className="font-semibold">Привет! Я Ямми <Emoji name="wave" /></div>
              <div className="text-sm text-muted mt-1">Задай вопрос или выбери подсказку:</div>
            </div>
            <div className="flex flex-wrap gap-2 justify-center max-w-lg">
              {SUGGESTIONS.map((s) => (
                <button
                  key={s}
                  onClick={() => send(s)}
                  className="text-xs px-3 py-2 rounded-full glass hover:border-accent/40 text-muted hover:text-text transition-colors cursor-pointer"
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((m, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            className={clsx("flex gap-2.5", m.role === "user" ? "justify-end" : "justify-start")}
          >
            {m.role === "assistant" && <Emoji name="robot" className="mt-1 shrink-0" />}
            <div
              className={clsx(
                "max-w-[75%] rounded-2xl px-4 py-2.5 text-sm whitespace-pre-wrap break-words",
                m.role === "user"
                  ? "bg-gradient-to-br from-accent-dark to-accent-2 text-bg rounded-br-md font-medium"
                  : "glass rounded-bl-md"
              )}
            >
              {m.content}
            </div>
          </motion.div>
        ))}

        {thinking && (
          <div className="flex gap-2.5 items-center">
            <Emoji name="robot" />
            <div className="glass rounded-2xl rounded-bl-md px-4 py-3 flex items-center gap-2 text-sm text-muted">
              <Loader2 size={14} className="animate-spin" />
              Ямми думает… <Emoji name="thinking" />
            </div>
          </div>
        )}
      </div>

      <div className="flex gap-2 shrink-0">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && send()}
          placeholder="Спроси Ямми о Minecraft…"
          disabled={thinking}
          className="flex-1 bg-bg border border-border rounded-xl px-4 py-3 text-sm outline-none focus:border-accent/50 transition-colors disabled:opacity-60"
        />
        <button
          onClick={() => send()}
          disabled={!input.trim() || thinking}
          className="px-5 rounded-xl bg-gradient-to-r from-accent-dark to-accent text-bg font-semibold hover:opacity-90 disabled:opacity-40 transition-opacity cursor-pointer flex items-center gap-2"
        >
          <Send size={16} />
        </button>
      </div>
    </div>
  );
}