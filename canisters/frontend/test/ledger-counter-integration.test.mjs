import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createApp, initLedgerCounter, initHeroArtwork } from '../public/app.js';
import { INSTANCES } from '../public/instances.js';
import { verifyInstanceConfiguration, ConfigurationMismatchError } from '../public/instance-view.js';
import { fakeDocument, eventTarget, deferred, backendResult, principal } from './helpers.mjs';
import { clock, flush } from './counter-clock.mjs';
const a = INSTANCES[0], b = { ...a, id: 'b', name: 'Reviewed B', backendCanisterId: 'backend-b', ledgerCountSource: 'icrc3-log-length' };
function fixture(query, queryBackend = async () => backendResult(a, 10), instances = [a, b, INSTANCES[1]]) {
  const doc = fakeDocument({ counter: true }), time = clock(), media = { ...eventTarget(), matches: false };
  const win = { ...eventTarget(), AbortController, requestAnimationFrame() {}, matchMedia: () => media };
  const counter = initLedgerCounter(doc, win, { query, dependencies: time });
  const app = createApp({ documentRef: doc, instances, queryBackend, counter, autoLoad: false });
  return { doc, time, media, win, counter, app };
}

test('page binds selection before pricing and rejects reversed ledger successes/failures', async () => {
  for (const terminal of ['resolve', 'reject']) {
    const old = deferred(), current = deferred(), backend = deferred(), calls = [];
    const f = fixture(entry => { calls.push(entry); return entry.id === 'icp' ? old.promise : current.promise; }, () => backend.promise);
    const loadA = f.app.load(); await flush(); assert.equal(calls.length, 1);
    f.doc.nodes.instance.value = 'b'; const loadB = f.app.load();
    assert.equal(f.doc.nodes['ledger-count'].textContent, '—'); assert.equal(f.doc.nodes['ledger-label'].textContent, 'Reviewed B ledger records');
    await flush(); current.resolve(1n << 100n); await flush(); const text = f.doc.nodes['ledger-count'].textContent;
    old[terminal](terminal === 'resolve' ? 999n : Error('late')); await flush(); assert.equal(f.doc.nodes['ledger-count'].textContent, text);
    backend.reject(Error('pricing transport')); await Promise.all([loadA, loadB]);
    assert.equal(f.doc.nodes['ledger-count'].textContent, text); assert.equal(f.doc.nodes.principal.disabled, true);
    f.app.dispose(); assert.equal(f.time.timers.size, 0); assert.equal(f.time.frames.size, 0);
  }
});

test('positive mismatch has a typed path and retires the statistic; transport failure does not', async () => {
  const wrong = backendResult(a, 10); wrong.instance.observed_ledger = principal('unexpected');
  assert.throws(() => verifyInstanceConfiguration(a, wrong.instance), ConfigurationMismatchError);
  const pending = deferred(), f = fixture(() => pending.promise, async () => wrong);
  await f.app.load(); assert.match(f.doc.nodes['ledger-status'].textContent, /configuration mismatch/);
  pending.resolve(999n); await flush(); assert.equal(f.doc.nodes['ledger-count'].textContent, '—');
  assert.equal(f.doc.nodes.principal.disabled, true); f.app.dispose();
  const g = fixture(async () => 100n, async () => { throw Error('Configuration mismatch: misleading transport text'); });
  await g.app.load(); await flush(); assert.match(g.doc.nodes['ledger-status'].textContent, /Periodically refreshed/); g.app.dispose();
});

test('ledger failure leaves verified pricing and memo construction available', async () => {
  const f = fixture(async () => { throw Error('<img onerror=bad>'); }); await f.app.load(); await flush();
  assert.match(f.doc.nodes.pricing.innerHTML, /Account subscription: 10 ICP/);
  f.doc.nodes.principal.value = 'uuc56-gyb'; f.doc.nodes.subaccount.value = '7'; f.doc.dispatch('input');
  assert.equal(f.doc.nodes['memo-output'].textContent, 'X.uuc56gyb.7');
  assert.match(f.doc.nodes['ledger-status'].textContent, /unavailable/); assert.doesNotMatch(f.doc.nodes['ledger-status'].textContent, /img/); f.app.dispose();
});

test('planned selection stops work and clears both memo and statistic synchronously', async () => {
  let calls = 0; const f = fixture(async () => { calls++; return 100n; }); await f.app.load(); await flush();
  f.doc.nodes.instance.value = 'io'; await f.app.load();
  assert.equal(f.doc.nodes['ledger-count'].textContent, '—'); assert.match(f.doc.nodes['ledger-status'].textContent, /until launch/);
  assert.equal(f.doc.nodes['memo-output'].textContent, '—'); await f.time.tick(100000); assert.equal(calls, 1);
  f.doc.nodes.instance.value = 'icp'; await f.app.load(); await flush(); assert.equal(calls, 2); f.app.dispose();
});

test('visibility coalesces, reduced motion remains dynamic, disposal removes listeners', async () => {
  let total = 100n, calls = 0; const f = fixture(async () => { calls++; return total; }); await f.app.load(); await flush();
  f.doc.hidden = true; f.doc.dispatch('visibilitychange'); f.doc.dispatch('visibilitychange');
  total = 1000n; f.media.matches = true; f.media.dispatch('change'); await f.time.tick(100000);
  assert.equal(calls, 1); assert.equal(f.doc.nodes['ledger-count'].textContent, '100');
  f.doc.hidden = false; f.doc.dispatch('visibilitychange'); f.doc.dispatch('visibilitychange'); await flush();
  assert.equal(calls, 2); assert.equal(f.doc.nodes['ledger-count'].textContent, '1,000');
  assert.equal(f.doc.nodes['ledger-status'].textContent, 'Periodically refreshed');
  f.app.dispose(); f.media.dispatch('change'); f.doc.dispatch('visibilitychange'); await f.time.tick(100000);
  assert.equal(calls, 2); assert.equal(f.media.listeners.get('change').size, 0);
});

test('accessible sample exposes exact accepted value once while visual numeral interpolates quietly', async () => {
  let total = 100n; const f = fixture(async () => total); await f.app.load(); await flush(); total = 1000n;
  await f.time.tick(3000); f.time.frame(10500);
  assert.equal(f.doc.nodes['ledger-count'].textContent, '550'); assert.match(f.doc.nodes['ledger-sample'].textContent, /1,000 \(latest retrieved total\)/);
  const html = await readFile(new URL('../public/index.html', import.meta.url), 'utf8');
  assert.match(html, /id="ledger-count"[^>]*aria-hidden="true"/); assert.doesNotMatch(html, /<(?:output|status)[^>]*id="ledger/);
  assert.doesNotMatch(html, /aria-live/); assert.match(html, /aria-describedby="ledger-explanation"/);
  assert.doesNotMatch(html, /live-effects|Pause live effects/); f.app.dispose();
});

test('missing or unsupported counter cannot prevent ordinary page operation', async () => {
  const doc = fakeDocument(); assert.equal(initLedgerCounter(doc, {}), null);
  const app = createApp({ documentRef: doc, instances: [a], queryBackend: async () => backendResult(a, 10), autoLoad: false,
    counter: { select() { throw Error('component'); }, dispose() {} } });
  await app.load(); assert.equal(doc.nodes.principal.disabled, false); app.dispose();
});

test('repeated SVG loads synchronize visibility, focus and dynamic asset changes; listeners are removed', () => {
  const media = { ...eventTarget(), matches: false }, detail = { ...eventTarget(), dataset: { animatedSrc: '/moving', staticSrc: '/static' },
    data: '/moving', getAttribute() { return this.data; }, classList: { add() {} } };
  const doc = { ...eventTarget(), hidden: false, hasFocus: () => true, querySelector: () => detail };
  const win = { ...eventTarget(), matchMedia: () => media };
  doc.hidden = true; const art = initHeroArtwork(doc, win);
  let paused = false, timeline = false;
  const svg = () => ({ classList: { toggle(_name, value) { paused = value; } },
    pauseAnimations() { timeline = true; }, unpauseAnimations() { timeline = false; } });
  detail.contentDocument = { documentElement: svg() }; detail.dispatch('load'); assert.ok(paused && timeline);
  media.matches = true; media.dispatch('change'); assert.equal(detail.data, '/static');
  detail.contentDocument = { documentElement: svg() }; detail.dispatch('load'); assert.ok(paused && timeline);
  media.matches = false; media.dispatch('change'); assert.equal(detail.data, '/moving');
  detail.contentDocument = { documentElement: svg() }; detail.dispatch('load'); assert.ok(paused && timeline);
  doc.hidden = false; doc.dispatch('visibilitychange'); assert.equal(timeline, false); win.dispatch('blur'); assert.equal(timeline, true);
  doc.hidden = true; win.dispatch('focus'); assert.equal(timeline, true);
  doc.hidden = false; doc.dispatch('visibilitychange'); assert.equal(timeline, false);
  art.dispose(); assert.equal(timeline, true);
  for (const target of [doc, win, detail, media]) for (const listeners of target.listeners.values()) assert.equal(listeners.size, 0);
});

test('page rate uses source-specific units and a quiet exact description, with an unavailable placeholder on startup/suspension', async () => {
  let total = 100n;
  const f = fixture(async () => total); await f.app.load(); await flush();
  assert.equal(f.doc.nodes['ledger-rate'].textContent, '— tx/s');
  assert.equal(f.doc.nodes['ledger-status'].hidden, true);
  total = 136n; await f.time.tick(18000);
  assert.equal(f.doc.nodes['ledger-rate'].textContent, '2.0 tx/s');
  assert.match(f.doc.nodes['ledger-rate-description'].textContent, /Approximate recent average.*18.0 seconds/);
  const previous = f.doc.nodes['ledger-rate'].textContent;
  f.time.frame(20000); assert.equal(f.doc.nodes['ledger-rate'].textContent, previous);
  f.doc.hidden = true; f.doc.dispatch('visibilitychange'); assert.equal(f.doc.nodes['ledger-rate'].textContent, previous);
  f.doc.hidden = false; f.doc.dispatch('visibilitychange'); await flush(); assert.equal(f.doc.nodes['ledger-rate'].textContent, previous);
  assert.match(f.doc.nodes['ledger-rate-description'].textContent, /Last known estimate/);
  f.doc.nodes.instance.value = 'b'; await f.app.load(); await flush();
  total = 164n; await f.time.tick(18000); assert.match(f.doc.nodes['ledger-rate'].textContent, /records\/s/);
  f.doc.nodes.instance.value = 'io'; await f.app.load(); assert.equal(f.doc.nodes['ledger-rate'].textContent, ''); f.app.dispose();
});

test('startup is quiet before and during the first request; failures remain visible', async () => {
  const pending = deferred(), f = fixture(() => pending.promise);
  assert.equal(f.doc.nodes['ledger-status'].hidden, true);
  await f.app.load(); await flush();
  assert.equal(f.doc.nodes['ledger-count'].textContent, '—');
  assert.equal(f.doc.nodes['ledger-status'].hidden, true);
  pending.reject(Error('offline')); await flush();
  assert.equal(f.doc.nodes['ledger-status'].hidden, false);
  assert.match(f.doc.nodes['ledger-status'].textContent, /unavailable/);
  f.app.dispose();
  const html = await readFile(new URL('../public/index.html', import.meta.url), 'utf8');
  assert.match(html, /id="ledger-status" hidden/);
  assert.match(html, /<noscript><p>Ledger count requires JavaScript/);
});
