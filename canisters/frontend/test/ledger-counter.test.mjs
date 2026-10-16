import test from 'node:test';
import assert from 'node:assert/strict';
import { createLedgerCounter, interpolateTotal, formatTotal } from '../public/ledger-counter.js';
import { INSTANCES } from '../public/instances.js';
import { deferred } from './helpers.mjs';
import { clock, flush } from './counter-clock.mjs';
const live = INSTANCES[0];
function fixture(query = async () => 100n) {
  const time = clock(), states = [], calls = [];
  const controller = createLedgerCounter({ ...time, query: (entry, options) => {
    calls.push({ entry, ...options, at: time.now() }); return query(entry, options, calls.length);
  }, render: state => states.push(state) });
  return { ...controller, time, states, calls, last: () => states.at(-1) };
}

test('precision, grouping, exact fixed-point interpolation and pathological clocks', () => {
  for (const base of [0n, (1n << 53n) - 1n, 1n << 53n, (1n << 64n) - 1n, 1n << 100n]) {
    assert.equal(interpolateTotal(base, base + 3n, 7500, 15000), base + 1n);
    assert.equal(interpolateTotal(base, base + 3n, 15000, 15000), base + 3n);
    for (const bad of [undefined, null, '100', {}, NaN, -Infinity, -1]) assert.equal(interpolateTotal(base, base + 3n, bad, 15000), base);
    assert.equal(interpolateTotal(base, base + 3n, Infinity, 15000), base + 3n);
  }
  assert.deepEqual([0n, 999n, 1000n, 1000000n].map(formatTotal), ['0', '999', '1,000', '1,000,000']);
  assert.equal(formatTotal((1n << 64n) - 1n), '18,446,744,073,709,551,615');
});

test('deterministic randomized monotonic bounded interpolation, independent of frame rate', () => {
  let seed = 1234567;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed; };
  for (let run = 0; run < 1000; run++) {
    const a = (BigInt(random()) << 80n) + BigInt(random()), b = a + BigInt(random());
    let previous = a;
    for (let t = 0; t < 15000; t += random() % 900 + 1) {
      const value = interpolateTotal(a, b, t, 15000);
      assert.ok(previous <= value && value <= b); previous = value;
    }
    assert.equal(interpolateTotal(a, b, 15000, 15000), b);
    assert.equal(interpolateTotal(a, b, 5000, 15000), a + (b - a) * 333333n / 1000000n);
  }
});

test('first sample directly renders history, warm-up happens once, steady polls after completion', async () => {
  const f = fixture(); f.select(live); assert.equal(f.last().displayed, null); await flush();
  assert.equal(f.last().displayed, 100n); assert.equal(f.time.frames.size, 0);
  await f.time.tick(2999); assert.equal(f.calls.length, 1);
  await f.time.tick(1); assert.equal(f.calls.length, 2);
  await f.time.tick(15000); assert.equal(f.calls.length, 3);
  await f.time.tick(15000); assert.equal(f.calls.length, 4); f.dispose();
});

test('genuine zero differs from initial unavailable; invalid samples fail', async () => {
  const f = fixture(async () => 0n); f.select(live); await flush(); assert.equal(f.last().displayed, 0n); f.dispose();
  for (const bad of [undefined, null, 5, -1n, 1n << 64n]) {
    const g = fixture(async () => bad); g.select(live); await flush();
    assert.equal(g.last().status, 'unavailable'); assert.equal(g.last().checked, null); g.dispose();
  }
});

test('linear animation retargets from current display and equal samples never extend it', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush();
  total = 400n; await f.time.tick(3000); f.time.frame(10500); assert.equal(f.last().displayed, 250n);
  // Advance timer to equal sample before the next animation frame.
  await f.time.tick(7500); assert.equal(f.last().accepted, 400n);
  f.time.frame(18000); assert.equal(f.last().displayed, 400n); assert.equal(f.time.frames.size, 0);
  total = 1000n; await f.time.tick(15000); f.time.frame(38000); assert.equal(f.last().displayed, 599n);
  total = 1600n; await f.time.tick(10000); // No frame since t=37000: retarget from the rendered 599.
  assert.equal(f.last().displayed, 599n); f.time.frame(55500); assert.equal(f.last().displayed, 1099n);
  f.time.frame(90000); assert.equal(f.last().displayed, 1600n); assert.equal(f.time.frames.size, 0); f.dispose();
});

test('early retarget, clock reversal and stalled frames preserve the invariant', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush();
  total = 1000n; await f.time.tick(3000); f.time.frame(5000); const seen = f.last().displayed;
  f.time.frame(4000); assert.equal(f.last().displayed, seen);
  // Use a short steady interval to get a genuine higher sample mid-animation.
  f.dispose(); const t = clock(), values = []; let n = 0;
  const c = createLedgerCounter({ ...t, timing: { warmup: 2, interval: 3, duration: 15, deadline: 10, backoff: 30, maxBackoff: 120, delayed: 45 },
    query: async () => [100n, 400n, 700n][Math.min(n++, 2)], render: s => values.push(s) });
  c.select(live); await flush(); await t.tick(2); t.frame(4); const a = values.at(-1).displayed;
  await t.tick(1); assert.equal(values.at(-1).displayed, a);
  t.frame(10); assert.ok(values.at(-1).displayed >= a); assert.ok(values.at(-1).displayed <= 700n);
  t.frame(1000); assert.equal(values.at(-1).displayed, 700n); c.dispose();
});

test('single flight includes hung query and timeout retires it with bounded backoff', async () => {
  const pending = deferred(); const f = fixture(() => pending.promise); f.select(live); await flush();
  await f.time.tick(29999); assert.equal(f.calls.length, 1);
  await f.time.tick(1); assert.equal(f.calls[0].signal.aborted, true); assert.equal(f.last().status, 'unavailable');
  await f.time.tick(29999); assert.equal(f.calls.length, 1);
  await f.time.tick(1); assert.equal(f.calls.length, 2);
  pending.resolve(500n); await flush(); assert.equal(f.last().displayed, 500n); f.dispose();
});

test('a slow successful poll gets thirty seconds without concurrent retries or a failure state', async () => {
  const pending = deferred(), f = fixture((_entry, _options, n) => n === 1 ? 100n : pending.promise);
  f.select(live); await flush(); await f.time.tick(3000);
  await f.time.tick(29999);
  assert.equal(f.calls.length, 2); assert.equal(f.calls[1].signal.aborted, false);
  assert.equal(f.last().status, 'current'); assert.equal(f.last().accepted, 100n);
  pending.resolve(200n); await flush();
  assert.equal(f.last().status, 'current'); assert.equal(f.last().accepted, 200n);
  await f.time.tick(14999); assert.equal(f.calls.length, 2);
  await f.time.tick(1); assert.equal(f.calls.length, 3); f.dispose();
});

test('failure backoff is 30/60/120 capped and equal success resets it', async () => {
  let fail = true; const f = fixture(async () => { if (fail) throw new Error('offline'); return 100n; });
  f.select(live); await flush(); await f.time.tick(30000); await f.time.tick(60000); await f.time.tick(120000); await f.time.tick(120000);
  assert.deepEqual(f.calls.map(c => c.at), [0, 30000, 90000, 210000, 330000]);
  fail = false; await f.time.tick(120000); await f.time.tick(3000); assert.equal(f.last().status, 'current');
  fail = true; await f.time.tick(15000); const count = f.calls.length; await f.time.tick(30000); assert.equal(f.calls.length, count + 1); f.dispose();
});

test('freshness allows three minutes independently of an idle numeral and recovery jumps directly', async () => {
  let fail = false, total = 100n; const f = fixture(async () => { if (fail) throw Error('offline'); return total; });
  f.select(live); await flush(); fail = true; await f.time.tick(179999); assert.equal(f.last().status, 'retrying');
  await f.time.tick(1); assert.equal(f.last().status, 'delayed'); assert.equal(f.time.frames.size, 0);
  fail = false; total = 10000n; await f.time.tick(33000);
  assert.equal(f.last().displayed, 10000n); assert.equal(f.time.frames.size, 0); f.dispose();
});

test('lower replies retain high-water and timestamp; third is inconsistent; equal recovers', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush(); const checked = f.last().checked;
  total = 99n; await f.time.tick(3000); await f.time.tick(30000); await f.time.tick(60000);
  assert.equal(f.last().status, 'inconsistent'); assert.equal(f.last().checked, checked); assert.equal(f.last().accepted, 100n);
  total = 100n; await f.time.tick(120000); assert.equal(f.last().status, 'current'); assert.ok(f.last().checked > checked); f.dispose();
});

for (const reason of ['hide', 'selection', 'dispose', 'configuration']) {
  for (const terminal of ['resolve', 'reject']) test(`${reason} retires late ${terminal} without changing state or newer schedules`, async () => {
    const a = deferred(), b = deferred(); const f = fixture((_e, _o, n) => n === 1 ? a.promise : b.promise);
    f.select(live); await flush();
    if (reason === 'hide') f.setLifecycle({ hidden: true });
    if (reason === 'selection') f.select({ ...live, id: 'shared-ledger' });
    if (reason === 'dispose') f.dispose();
    if (reason === 'configuration') f.configurationMismatch();
    await flush(); const state = f.last(), timers = f.time.timers.size;
    assert.equal(f.calls[0].signal.aborted, true);
    a[terminal](terminal === 'resolve' ? 999999n : new Error('late')); await flush();
    assert.equal(f.last(), state); assert.equal(f.time.timers.size, timers);
    if (reason === 'selection') { b.resolve(200n); await flush(); assert.equal(f.last().displayed, 200n); }
    f.dispose(); assert.equal(f.time.timers.size, 0); assert.equal(f.time.frames.size, 0);
  });
}

test('A to planned to A clears number and bootstraps anew; unsupported modes fail closed', async () => {
  const f = fixture(); f.select(live); await flush(); f.select(INSTANCES[1]);
  assert.equal(f.last().displayed, null); assert.equal(f.last().status, 'planned'); await f.time.tick(100000); assert.equal(f.calls.length, 1);
  f.select(live); await flush(); assert.equal(f.last().displayed, 100n);
  for (const source of [null, undefined, 'unknown']) { f.select({ ...live, ledgerCountSource: source }); await flush(); assert.equal(f.last().status, 'unsupported'); }
  assert.equal(f.calls.length, 2); f.dispose();
});

test('hide/show coalesces, resume baseline retains monotonicity', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush();
  f.setLifecycle({ hidden: true }); f.setLifecycle({ hidden: true }); f.setLifecycle({ hidden: true });
  total = 1000n; await flush(); assert.equal(f.calls.length, 1); assert.equal(f.last().displayed, 100n);
  f.setLifecycle({ hidden: false }); f.setLifecycle({ hidden: false }); f.setLifecycle({ hidden: false }); await flush();
  assert.equal(f.calls.length, 2); assert.equal(f.last().displayed, 1000n); assert.equal(f.time.frames.size, 0);
  f.setLifecycle({ hidden: true }); total = 900n; f.setLifecycle({ hidden: false }); await flush(); assert.equal(f.last().accepted, 1000n); f.dispose();
});

test('dynamic reduced motion stops interpolation, keeps polling, and cannot override hidden suspension', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush(); total = 400n; await f.time.tick(3000); f.time.frame(5000);
  f.setLifecycle({ reduced: true }); assert.equal(f.last().displayed, 400n); assert.equal(f.time.frames.size, 0);
  total = 500n; await f.time.tick(15000); assert.equal(f.last().displayed, 500n);
  f.setLifecycle({ hidden: true }); f.setLifecycle({ reduced: false }); await f.time.tick(100000); assert.equal(f.calls.length, 3);
  total = 1000n; f.setLifecycle({ hidden: false }); await flush(); assert.equal(f.last().displayed, 1000n); f.dispose();
});

test('retired timer and animation callbacks cannot mutate a replacement context', async () => {
  let total = 100n; const f = fixture(async () => total); f.select(live); await flush();
  total = 500n; await f.time.tick(3000);
  const staleFrames = [...f.time.frames.values()], staleTimers = [...f.time.timers.values()].map(t => t.fn);
  f.select({ ...live, id: 'other' }); await flush(); const state = f.last(), timers = f.time.timers.size;
  staleFrames.forEach(fn => fn()); staleTimers.forEach(fn => fn()); await flush();
  assert.equal(f.last(), state); assert.equal(f.time.timers.size, timers); f.dispose();
});

test('equal sample in mid-animation updates freshness without changing its linear timeline', async () => {
  const t = clock(), states = []; let count = 0;
  const c = createLedgerCounter({ ...t, timing: { warmup: 2, interval: 3, duration: 15, deadline: 10, backoff: 30, maxBackoff: 120, delayed: 45 },
    query: async () => ++count === 1 ? 100n : 400n, render: s => states.push(s) });
  c.select(live); await flush(); await t.tick(2); t.frame(4); assert.equal(states.at(-1).displayed, 139n);
  await t.tick(1); const checked = states.at(-1).checked;
  t.frame(8); assert.equal(states.at(-1).displayed, 220n); assert.ok(checked > 1760000000000);
  await t.tick(3); t.frame(17); assert.equal(states.at(-1).displayed, 400n); assert.equal(t.frames.size, 0); c.dispose();
});

test('ICRC-3 tiny increment at an enormous baseline arrives exactly without Number conversion', async () => {
  const huge = (1n << 200n) + 999n; let total = huge;
  const f = fixture(async () => total); f.select({ ...live, ledgerCountSource: 'icrc3-log-length' }); await flush();
  assert.equal(f.last().displayed, huge); total++; await f.time.tick(3000); f.time.frame(10500); assert.equal(f.last().displayed, huge);
  f.time.frame(18000); assert.equal(f.last().displayed, huge + 1n); f.dispose();
});


test('an optional renderer exception retires work without an unhandled rejection or retry', async () => {
  const t = clock(); let renders = 0, queries = 0;
  const c = createLedgerCounter({ ...t, query: async () => { queries++; return 100n; },
    render() { if (++renders === 2) throw Error('removed DOM'); } });
  c.select(live); await flush(); await t.tick(120000);
  assert.equal(queries, 1); assert.equal(renders, 2);
  assert.equal(t.timers.size, 0); assert.equal(t.frames.size, 0); c.dispose();
});
