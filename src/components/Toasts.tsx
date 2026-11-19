import { AnimatePresence, motion } from "framer-motion";
import { X } from "lucide-react";
import { useStore } from "../stores/app";
import Emoji from "./Emoji";
import clsx from "clsx";

export default function Toasts() {
  const toasts = useStore((s) => s.toasts);
  const dismiss = useStore((s) => s.dismissToast);
  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-2 w-96 pointer-events-none">
      <AnimatePresence>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            initial={{ opacity: 0, x: 40, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 40, scale: 0.95 }}
            transition={{ type: "spring", stiffness: 400, damping: 28 }}
            className={clsx(
              "glass rounded-xl p-3 flex items-start gap-2.5 pointer-events-auto text-sm",
              t.kind === "error" && "border-danger/40",
              t.kind === "success" && "border-accent/40"
            )}
          >
            <Emoji name={t.kind === "error" ? "warning" : t.kind === "success" ? "party" : "bulb"} />
            <div className="flex-1 break-words">{t.text}</div>
            <button onClick={() => dismiss(t.id)} className="text-muted hover:text-text cursor-pointer">
              <X size={14} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}