import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Volume2, VolumeX } from "lucide-react";
import { useStore } from "../stores/app";
import Emoji from "./Emoji";

const win = getCurrentWindow();

export default function TitleBar() {
  const soundOn = useStore((s) => s.soundOn);
  const setSoundOn = useStore((s) => s.setSoundOn);
  return (
    <div
      data-tauri-drag-region
      className="h-11 flex items-center justify-between shrink-0 border-b border-border bg-bg-soft/90 select-none"
    >
      <div data-tauri-drag-region className="flex items-center gap-2.5 px-4 pointer-events-none">
        <Emoji name="orange" size="lg" />
        <span className="font-display text-sm tracking-wider">
          YAMMY<span className="text-accent">LAUNCH</span>
        </span>
        <span className="text-[10px] text-muted ml-1 font-sans">v0.2 🍊 edition</span>
      </div>
      <div className="flex h-full items-center">
        <button
          onClick={() => setSoundOn(!soundOn)}
          className="w-11 h-full flex items-center justify-center text-muted hover:bg-panel-2 hover:text-accent transition-colors"
          title={soundOn ? "Выключить звуки" : "Включить звуки"}
        >
          {soundOn ? <Volume2 size={15} /> : <VolumeX size={15} />}
        </button>
        <button
          onClick={() => win.minimize()}
          className="w-12 h-full flex items-center justify-center text-muted hover:bg-panel-2 hover:text-text transition-colors"
          title="Свернуть"
        >
          <Minus size={15} />
        </button>
        <button
          onClick={() => win.toggleMaximize()}
          className="w-12 h-full flex items-center justify-center text-muted hover:bg-panel-2 hover:text-text transition-colors"
          title="Развернуть"
        >
          <Square size={12} />
        </button>
        <button
          onClick={() => win.close()}
          className="w-12 h-full flex items-center justify-center text-muted hover:bg-danger hover:text-white transition-colors"
          title="Закрыть"
        >
          <X size={16} />
        </button>
      </div>
    </div>
  );
}