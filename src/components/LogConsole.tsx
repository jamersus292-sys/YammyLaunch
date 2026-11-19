import { useEffect, useRef } from "react";
import { LogLine } from "../lib/tauri";
import clsx from "clsx";

export default function LogConsole({ lines }: { lines: LogLine[] }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);
  return (
    <div
      ref={ref}
      className="log-console flex-1 overflow-y-auto bg-bg rounded-lg border border-border p-3 min-h-0"
    >
      {lines.length === 0 ? (
        <div className="text-muted">Лог игры появится здесь после запуска…</div>
      ) : (
        lines.map((l, i) => (
          <div
            key={i}
            className={clsx(
              "whitespace-pre-wrap break-all",
              l.stream === "stderr" && "text-danger/90",
              l.stream === "launcher" && "text-accent",
              l.stream === "stdout" && "text-text/80"
            )}
          >
            {l.line}
          </div>
        ))
      )}
    </div>
  );
}