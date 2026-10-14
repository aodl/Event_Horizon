export const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
export function clock() {
  let time = 0, id = 0;
  const timers = new Map(), frames = new Map();
  return {
    timers, frames, now: () => time, wallNow: () => 1760000000000 + time,
    setTimer(fn, delay) { const key = ++id; timers.set(key, { at: time + delay, fn }); return key; },
    clearTimer: key => timers.delete(key),
    requestFrame(fn) { const key = ++id; frames.set(key, fn); return key; },
    cancelFrame: key => frames.delete(key),
    set(value) { time = value; },
    frame(value = time) { time = value; const callbacks = [...frames.values()]; frames.clear(); callbacks.forEach(fn => fn(time)); },
    async tick(ms) {
      const end = time + ms;
      for (;;) {
        const due = [...timers.entries()].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
        if (!due) break;
        time = due[1].at; timers.delete(due[0]); due[1].fn(); await flush();
      }
      time = end; await flush();
    },
  };
}
