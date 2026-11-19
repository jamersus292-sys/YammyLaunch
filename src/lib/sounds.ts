// Милые звуковые эффекты на WebAudio — без аудиофайлов, всё синтезируется.
let ctx: AudioContext | null = null;
let enabled = true;

export function setSoundEnabled(v: boolean) {
  enabled = v;
}
export function soundEnabled() {
  return enabled;
}

function ac(): AudioContext | null {
  if (!enabled) return null;
  try {
    if (!ctx) ctx = new AudioContext();
    if (ctx.state === "suspended") ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

function tone(
  freq: number,
  start: number,
  dur: number,
  type: OscillatorType = "sine",
  vol = 0.12,
  slideTo?: number
) {
  const c = ac();
  if (!c) return;
  const t0 = c.currentTime + start;
  const osc = c.createOscillator();
  const gain = c.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, t0);
  if (slideTo) osc.frequency.exponentialRampToValueAtTime(slideTo, t0 + dur);
  gain.gain.setValueAtTime(0, t0);
  gain.gain.linearRampToValueAtTime(vol, t0 + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
  osc.connect(gain).connect(c.destination);
  osc.start(t0);
  osc.stop(t0 + dur + 0.05);
}

export const sfx = {
  /** мягкий клик по кнопке */
  click() {
    tone(660, 0, 0.07, "triangle", 0.07);
    tone(990, 0.02, 0.05, "sine", 0.04);
  },
  /** переход между страницами — лёгкий «вжух» */
  nav() {
    tone(440, 0, 0.09, "sine", 0.05, 660);
  },
  /** успех — весёлое двухнотное «динь-дон» вверх */
  success() {
    tone(523.25, 0, 0.12, "triangle", 0.1);
    tone(783.99, 0.1, 0.16, "triangle", 0.1);
    tone(1046.5, 0.2, 0.2, "sine", 0.06);
  },
  /** ошибка — мягкое «бу-бум» вниз, не пугающее */
  error() {
    tone(330, 0, 0.14, "sine", 0.09, 220);
    tone(196, 0.12, 0.18, "sine", 0.07);
  },
  /** запуск игры — нарастающий «взлёт» 🚀 */
  launch() {
    tone(261.63, 0, 0.1, "triangle", 0.09);
    tone(392, 0.09, 0.1, "triangle", 0.09);
    tone(523.25, 0.18, 0.12, "triangle", 0.1);
    tone(783.99, 0.28, 0.24, "sine", 0.08, 1046.5);
  },
  /** игра закрыта — спокойное «пока-пока» */
  exit() {
    tone(659.25, 0, 0.12, "sine", 0.07);
    tone(523.25, 0.11, 0.18, "sine", 0.06);
  },
  /** установка мода/сборки — «поп» как пробка */
  pop() {
    tone(300, 0, 0.06, "sine", 0.1, 600);
    tone(900, 0.04, 0.06, "triangle", 0.05);
  },
  /** сообщение ИИ — нежное «блюп» */
  message() {
    tone(880, 0, 0.08, "sine", 0.06, 1174.66);
  },
  /** переключение тоггла */
  toggle() {
    tone(520, 0, 0.05, "square", 0.035);
  },
};