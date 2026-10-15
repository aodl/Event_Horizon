import {formatCycles, tierProgress} from './operating-tier.js';

export function createOperatingStatus({query, render, setTimer = setTimeout, clearTimer = clearTimeout,
  interval = 60000, deadline = 10000} = {}) {
  let entry, snapshot = null, status = 'unavailable', generation = 0, hidden = false, disposed = false;
  let timer = null, active = null;
  const emit = () => {
    if (disposed) return;
    try { render({entry, snapshot, status}); }
    catch { disposed = true; retire(); } // An optional panel must never break pricing or keep polling after a render failure.
  };
  const retire = () => {
    generation++; clearTimer(timer); timer = null;
    if (active) { clearTimer(active.timer); active.abort.abort(); active = null; }
  };
  const eligible = () => entry?.status === 'live' && Boolean(entry.backendCanisterId);
  function attempt() {
    if (disposed || hidden || active || !eligible() || status === 'configuration') return;
    const gen = generation, captured = entry, owned = {abort: new AbortController(), timer: null};
    active = owned;
    const current = () => !disposed && generation === gen && active === owned;
    const timeout = new Promise((_, reject) => {
      owned.timer = setTimer(() => { if (current()) { owned.abort.abort(); reject(Error('Operating status timed out.')); } }, deadline);
    });
    Promise.race([Promise.resolve().then(() => {
      if (!current()) throw Error('Retired operating status request.');
      return query(captured, {signal: owned.abort.signal});
    }), timeout]).then(value => {
      if (!current()) return;
      tierProgress(value.mode, value.liquidCycles); // Validate before accepting or rendering.
      snapshot = value; status = 'current'; emit();
    }).catch(() => {
      if (!current()) return;
      status = 'unavailable'; emit();
    }).finally(() => {
      clearTimer(owned.timer);
      if (!current()) return;
      owned.abort.abort(); active = null;
      timer = setTimer(() => { timer = null; if (gen === generation) attempt(); }, interval);
    });
  }
  return {
    select(next) { if (disposed) return; retire(); entry = next; snapshot = null; status = next?.status === 'planned' ? 'planned' : eligible() ? 'loading' : 'unavailable'; emit(); attempt(); },
    setHidden(value) { if (disposed || hidden === Boolean(value)) return; hidden = Boolean(value); retire(); if (!hidden) attempt(); },
    configurationMismatch() { if (disposed) return; retire(); snapshot = null; status = 'configuration'; emit(); },
    dispose() { disposed = true; retire(); },
  };
}

export function renderOperatingStatus(documentRef, {snapshot, status}) {
  const node = id => documentRef.getElementById(id);
  if (!node('operating-tier')) return;
  const write = (id, value) => { if (node(id)) node(id).textContent = value; };
  if (!snapshot || status !== 'current') {
    write('operating-tier', status === 'planned' ? 'Planned' : status === 'loading' ? '—' : 'Unavailable');
    write('operating-balance', '—'); write('operating-delay', '');
    write('operating-next', status === 'planned' ? 'Operating status available after launch.' : status === 'configuration' ? 'Instance configuration mismatch.' : status === 'loading' ? 'Checking operating status…' : 'Operating status unavailable · Retrying.');
    if (node('operating-progress')) node('operating-progress').hidden = true;
    write('operating-checked', ''); return;
  }
  const {tier, next, remaining, percent} = tierProgress(snapshot.mode, snapshot.liquidCycles);
  write('operating-tier', tier.label); write('operating-balance', formatCycles(snapshot.balanceCycles ?? snapshot.liquidCycles));
  write('operating-balance-label', snapshot.balanceCycles === undefined ? 'Liquid cycles' : 'Cycles balance');
  write('operating-mode-label', snapshot.reported ? 'Last reported tier' : 'Polling tier');
  write('operating-delay', tier.delay);
  write('operating-next', !next ? 'Highest polling tier' : remaining > 0n
    ? `${formatCycles(remaining)} to ${next.label} · ${formatCycles(next.enter)} liquid-cycle entry threshold`
    : `${next.label} entry threshold met · Cadence changes at a scheduler check`);
  const progress = node('operating-progress');
  if (progress) { progress.hidden = !next; if (next) { progress.value = percent; progress.setAttribute('aria-valuetext', `${percent}% of the ${next.label} entry threshold`); } }
  write('operating-checked', snapshot.reported ? `Balance checked ${snapshot.checkedAt}. Threshold progress uses ${formatCycles(snapshot.liquidCycles)} liquid cycles reported ${snapshot.observedAt}.` : `Checked ${snapshot.observedAt}`);
}

export function initOperatingStatus(documentRef, windowRef, {query, client, dependencies = {}} = {}) {
  if (!documentRef.getElementById('operating-tier') || !windowRef?.AbortController) return null;
  const controller = createOperatingStatus({query:query ?? client.query, render:state => renderOperatingStatus(documentRef,state), ...dependencies});
  const lifecycle = () => controller.setHidden(documentRef.hidden);
  documentRef.addEventListener('visibilitychange',lifecycle); lifecycle();
  return {select:controller.select, configurationMismatch:controller.configurationMismatch, dispose() {
    controller.dispose(); client?.dispose(); documentRef.removeEventListener('visibilitychange',lifecycle);
  }};
}
