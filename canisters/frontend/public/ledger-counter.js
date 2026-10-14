import { eligibleLedger, validateTotal } from './ledger-counter-client.js';

export const COUNTER_TIMING = Object.freeze({ warmup: 2000, interval: 15000, duration: 15000,
  deadline: 10000, backoff: 30000, maxBackoff: 120000, delayed: 45000 });
export const formatTotal = total => total.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');

export function interpolateTotal(a, b, elapsed, duration) {
  const progress = typeof elapsed !== 'number' || Number.isNaN(elapsed) || elapsed <= 0 ? 0
    : elapsed >= duration ? 1000000 : Math.floor(elapsed / duration * 1000000);
  return a + (b - a) * BigInt(progress) / 1000000n;
}

export function createLedgerCounter({ query, render, now = () => performance.now(),
  wallNow = () => Date.now(), setTimer = setTimeout, clearTimer = clearTimeout,
  requestFrame = callback => requestAnimationFrame(callback), cancelFrame = cancelAnimationFrame,
  timing = COUNTER_TIMING,
} = {}) {
  let entry, generation = 0, disposed = false, paused = false, hidden = false, reduced = false;
  let accepted = null, displayed = null, target = null, received = null, checked = null;
  let status = 'unavailable', failures = 0, lower = 0, warm = false, baseline = true;
  let poll = null, fresh = null, frame = null, active = null, animation = null;
  const suspended = () => paused || hidden;
  const delayed = () => received !== null && now() - received >= timing.delayed;
  function emit() {
    if (disposed) return;
    try { render({ entry, accepted, displayed, checked, reduced, paused,
      status: status === 'configuration' ? status : !eligibleLedger(entry) ? status
        : suspended() ? (paused ? 'paused' : 'hidden')
          : lower >= 3 ? 'inconsistent' : delayed() ? 'delayed' : status });
    } catch {
      // A failed optional renderer must not leave requests or asynchronous errors.
      disposed = true; clearWork();
    }
  }
  function clearWork() {
    ++generation;
    clearTimer(poll); poll = null;
    clearTimer(fresh); fresh = null;
    cancelFrame(frame); frame = null;
    if (active) { clearTimer(active.deadline); active.abort.abort(); active = null; }
    animation = null;
  }
  function scheduleFresh() {
    clearTimer(fresh); fresh = null;
    if (received === null || suspended() || disposed || delayed()) return;
    const gen = generation;
    fresh = setTimer(() => {
      if (gen !== generation || disposed || suspended()) return;
      fresh = null;
      baseline = true;
      emit();
    }, Math.max(0, timing.delayed - (now() - received)));
  }
  function advance() {
    if (!animation) return;
    const next = interpolateTotal(animation.from, target, now() - animation.start, timing.duration);
    displayed = next < displayed ? displayed : next;
    if (displayed === target) animation = null;
  }
  function animate() {
    if (frame !== null || !animation || suspended() || reduced) return;
    const gen = generation;
    frame = requestFrame(() => {
      if (gen !== generation || disposed || suspended()) return;
      frame = null;
      advance(); emit(); animate();
    });
  }
  function schedule(delay) {
    if (disposed || suspended() || !eligibleLedger(entry) || status === 'configuration') return;
    clearTimer(poll);
    const gen = generation;
    poll = setTimer(() => { if (gen !== generation) return; poll = null; attempt(); }, delay);
  }
  function attempt() {
    if (active || disposed || suspended() || !eligibleLedger(entry) || status === 'configuration') return;
    const gen = generation, captured = entry;
    const owned = { abort: new AbortController(), deadline: null };
    active = owned;
    const current = () => gen === generation && active === owned && !disposed;
    let rejectTimeout;
    const timeout = new Promise((_, reject) => { rejectTimeout = reject; });
    owned.deadline = setTimer(() => {
      if (!current()) return;
      owned.abort.abort();
      rejectTimeout(new Error('Ledger request timed out.'));
    }, timing.deadline);
    // Both branches have rejection handlers even if a retired dependency settles late.
    Promise.race([Promise.resolve().then(() => {
      if (!current()) throw new Error('Retired counter attempt.');
      return query(captured, { signal: owned.abort.signal });
    }), timeout]).then(total => {
      if (!current()) return;
      validateTotal(total, captured.ledgerCountSource);
      if (accepted !== null && total < accepted) {
        lower++;
        throw new Error('Ledger total decreased.');
      }
      const direct = baseline || delayed() || accepted === null || reduced;
      if (direct) {
        displayed = total; target = total; animation = null;
        cancelFrame(frame); frame = null;
      } else if (total > accepted) {
        target = total; animation = { from: displayed, start: now() };
      }
      accepted = total; received = now(); checked = wallNow();
      baseline = false; failures = 0; lower = 0; status = 'current';
      scheduleFresh(); emit(); animate();
    }).catch(() => {
      if (!current()) return;
      failures++; status = accepted === null ? 'unavailable' : 'retrying';
      emit();
    }).finally(() => {
      clearTimer(owned.deadline);
      if (!current()) return;
      active = null;
      // Retire owned transport even for injected queries that ignore failures.
      owned.abort.abort();
      let delay = timing.interval;
      if (failures) delay = Math.min(timing.maxBackoff, timing.backoff * 2 ** Math.min(failures - 1, 2));
      else if (!warm) { warm = true; delay = timing.warmup; }
      schedule(delay);
    });
  }
  return {
    select(next) {
      if (disposed) return;
      clearWork(); entry = next;
      accepted = displayed = target = received = checked = null;
      failures = lower = 0; warm = false; baseline = true;
      status = next?.status === 'planned' ? 'planned' : eligibleLedger(next) ? 'loading' : 'unsupported';
      emit(); attempt();
    },
    configurationMismatch() {
      clearWork(); status = 'configuration'; displayed = accepted = target = null; checked = null; emit();
    },
    setLifecycle(next = {}) {
      if (disposed) return;
      const was = suspended();
      if ('paused' in next) paused = Boolean(next.paused);
      if ('hidden' in next) hidden = Boolean(next.hidden);
      if ('reduced' in next && reduced !== Boolean(next.reduced)) {
        reduced = Boolean(next.reduced);
        if (reduced && !suspended() && accepted !== null) {
          displayed = target = accepted; animation = null; cancelFrame(frame); frame = null;
        }
      }
      if (was !== suspended()) {
        clearWork(); baseline = true;
        if (!suspended()) { warm = false; scheduleFresh(); attempt(); }
      }
      emit();
    },
    dispose() { disposed = true; clearWork(); },
  };
}
