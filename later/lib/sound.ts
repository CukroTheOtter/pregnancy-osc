// Tiny synthesized sounds (no audio files): heartbeat, kick thump, scan chime.

let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

function thump(at: number, freq: number, gain: number, len = 0.12) {
  const a = audio();
  if (!a) return;
  const o = a.createOscillator();
  const g = a.createGain();
  o.type = "sine";
  o.frequency.setValueAtTime(freq, at);
  o.frequency.exponentialRampToValueAtTime(freq * 0.6, at + len);
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(gain, at + 0.015);
  g.gain.exponentialRampToValueAtTime(0.0001, at + len);
  o.connect(g).connect(a.destination);
  o.start(at);
  o.stop(at + len + 0.02);
}

/** One "lub-dub". */
export function playHeartbeat(volume = 0.5) {
  const a = audio();
  if (!a) return;
  const t = a.currentTime;
  thump(t, 70, 0.6 * volume);
  thump(t + 0.18, 60, 0.4 * volume);
}

export function playKick(volume = 0.5) {
  const a = audio();
  if (!a) return;
  thump(a.currentTime, 110, 0.5 * volume, 0.09);
}

export function playChime(volume = 0.5) {
  const a = audio();
  if (!a) return;
  const t = a.currentTime;
  [660, 880, 1320].forEach((f, i) => {
    const o = a.createOscillator();
    const g = a.createGain();
    o.type = "triangle";
    o.frequency.value = f;
    g.gain.setValueAtTime(0.0001, t + i * 0.09);
    g.gain.exponentialRampToValueAtTime(0.25 * volume, t + i * 0.09 + 0.02);
    g.gain.exponentialRampToValueAtTime(0.0001, t + i * 0.09 + 0.5);
    o.connect(g).connect(a.destination);
    o.start(t + i * 0.09);
    o.stop(t + i * 0.09 + 0.55);
  });
}

let beatTimer: ReturnType<typeof setInterval> | null = null;

/** Continuous heartbeat while scanning, about 140 bpm like a baby's. */
export function startHeartbeat(volume = 0.5) {
  stopHeartbeat();
  playHeartbeat(volume);
  beatTimer = setInterval(() => playHeartbeat(volume), 430);
}

export function stopHeartbeat() {
  if (beatTimer) clearInterval(beatTimer);
  beatTimer = null;
}
