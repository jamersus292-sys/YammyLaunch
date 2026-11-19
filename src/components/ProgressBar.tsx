import { formatBytes } from "../lib/tauri";
import { useStore } from "../stores/app";

export default function ProgressBar() {
  const progress = useStore((s) => s.progress);
  if (!progress || progress.total === 0) return null;
  const pct = Math.min(100, Math.round((progress.done / progress.total) * 100));
  return (
    <div className="glass rounded-xl p-4 space-y-2">
      <div className="flex justify-between text-sm">
        <span className="font-medium">{progress.phase}</span>
        <span className="text-muted">
          {progress.done} / {progress.total}
          {progress.bytesTotal > 0 && (
            <> · {formatBytes(progress.bytesDone)} / {formatBytes(progress.bytesTotal)}</>
          )}
        </span>
      </div>
      <div className="h-2 rounded-full bg-panel-2 overflow-hidden">
        <div
          className="h-full rounded-full bg-gradient-to-r from-accent-dark to-accent transition-all duration-300"
          style={{ width: `${pct}%` }}
        />
      </div>
      {progress.currentFile && (
        <div className="text-[11px] text-muted truncate font-mono">{progress.currentFile}</div>
      )}
    </div>
  );
}