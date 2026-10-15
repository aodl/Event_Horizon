import test from 'node:test';
import assert from 'node:assert/strict';
import { recentRate, retainRateSamples, RATE_MAX_SAMPLES, createLedgerCounter } from '../public/ledger-counter.js';
import { INSTANCES } from '../public/instances.js';
import { clock, flush } from './counter-clock.mjs';
const pair = (delta, at = 15000, base = 1n << 100n) => [{ total: base, at: 0 }, { total: base + delta, at }];
test('rate needs 3 seconds; exact BigInt deltas, zero, tiny positive, rounded and grouped rates', () => {
  assert.equal(recentRate([]), null); assert.equal(recentRate(pair(10n, 2999)), null);
  assert.equal(recentRate(pair(6n, 3000)).text, '2.0');
  assert.equal(recentRate(pair(0n, 3000)).text, '0');
  for (const base of [0n, 1n << 53n, 1n << 64n, 1n << 200n]) {
    assert.equal(recentRate(pair(24n, 15000, base)).text, '1.6');
    assert.equal(recentRate(pair(0n, 15000, base)).text, '0');
    assert.equal(recentRate(pair(1n, 60000, base)).text, '<0.1');
  }
  assert.equal(recentRate(pair(154n, 20000)).text, '7.7');
  assert.equal(recentRate(pair(1n << 100n)).text, '84,510,040,015,215,293,433,113,547,025.1');
  assert.equal(recentRate(pair(1n, 20000)).text, '<0.1');
  for (const at of [0, -1, NaN, Infinity, 60001]) assert.equal(recentRate(pair(1n, at)), null);
  assert.equal(recentRate(pair(-1n)), null);
});
test('uneven receipt gaps and rolling window use actual elapsed time and bounded sample memory', () => {
  let samples = [];
  for (const at of [0, 2000, 17000, 34000, 56000]) samples = retainRateSamples(samples, { total: BigInt(at / 1000) * 2n, at });
  assert.equal(recentRate(samples).text, '2.0');
  samples = retainRateSamples(samples, { total: 160n, at: 71000 });
  assert.equal(samples[0].at, 17000); assert.equal(recentRate(samples).text, '2.3');
  for (let at = 72000; at < 80000; at++) samples = retainRateSamples(samples, { total: 160n, at });
  assert.equal(samples.length, RATE_MAX_SAMPLES); assert.equal(recentRate(samples), null);
  assert.deepEqual(retainRateSamples(samples, { total: 160n, at: 1 }), []);
});
function fixture() {
  const time = clock(), states = []; let total = 100n, fail = false, calls = 0;
  const c = createLedgerCounter({ ...time, query: async () => { calls++; if (fail) throw Error('offline'); return total; }, render: s => states.push(s) });
  return { ...c, time, states, last: () => states.at(-1), total: n => { total = n; }, fail: b => { fail = b; }, calls: () => calls };
}
test('rate only changes on accepted samples, never animation frames; no new polls', async () => {
  const f = fixture(); f.select(INSTANCES[0]); await flush(); assert.equal(f.last().rate, null);
  f.total(106n); await f.time.tick(2999); assert.equal(f.last().rate, null); assert.equal(f.calls(), 1);
  await f.time.tick(1); assert.equal(f.last().rate.text, '2.0'); assert.equal(f.calls(), 2);
  f.total(136n); await f.time.tick(15000); assert.equal(f.last().rate.text, '2.0');
  const rate = f.last().rate; f.time.frame(20000); f.time.frame(21000); assert.equal(f.last().rate, rate); assert.equal(f.calls(), 3);
  f.dispose(); assert.equal(f.time.timers.size, 0);
});
for (const lifecycle of ['hidden']) test(`rate survives ${lifecycle}, direct resume uses fresh history`, async () => {
  const f = fixture(); f.select(INSTANCES[0]); await flush(); await f.time.tick(18000); assert.equal(f.last().rate.text, '0');
  const held = f.last().rate;
  f.setLifecycle({ [lifecycle]: true }); assert.equal(f.last().rate, held); assert.equal(f.last().rateIsLastKnown, true); const calls = f.calls();
  await f.time.tick(100000); assert.equal(f.calls(), calls);
  f.setLifecycle({ [lifecycle]: false }); await flush(); assert.equal(f.last().rate, held); assert.equal(f.last().rateIsLastKnown, true);
  f.total(136n); await f.time.tick(18000); assert.equal(f.last().rate.text, '2.0'); assert.equal(f.last().rateIsLastKnown, false); f.dispose();
});
test('failure, timeout/backoff, lower replies, delayed data and recovery never retain old rate', async () => {
  const f = fixture(); f.select(INSTANCES[0]); await flush(); await f.time.tick(18000); assert.ok(f.last().rate);
  f.total(99n); await f.time.tick(15000); assert.equal(f.last().rate, null); assert.equal(f.last().accepted, 100n);
  await f.time.tick(30000); await f.time.tick(60000); assert.equal(f.last().status, 'inconsistent'); assert.equal(f.last().rate, null);
  f.total(100n); await f.time.tick(120000); assert.equal(f.last().rate, null);
  await f.time.tick(15000); assert.equal(f.last().rate.text, '0');
  f.fail(true); await f.time.tick(15000); assert.equal(f.last().rate, null); await f.time.tick(30000); assert.equal(f.last().status, 'delayed'); f.dispose();
});
test('reduced motion keeps sampled rate; every selection and mismatch clears rate independently of high-water', async () => {
  const f = fixture(); f.select(INSTANCES[0]); await flush(); await f.time.tick(18000); f.setLifecycle({ reduced: true }); assert.equal(f.last().rate.text, '0');
  f.select({ ...INSTANCES[0], id: 'same-ledger-new-selection' }); assert.equal(f.last().rate, null); await flush();
  await f.time.tick(18000); assert.ok(f.last().rate); f.configurationMismatch(); assert.equal(f.last().rate, null);
  f.select(INSTANCES[1]); assert.equal(f.last().rate, null); f.dispose();
});

test('hung request deadline and late success cannot restore a retired rate', async () => {
  const time = clock(), states = []; let hung = false, resolve;
  const c = createLedgerCounter({ ...time, render: s => states.push(s), query: async () => {
    if (hung) return new Promise(r => { resolve = r; }); return 100n;
  } });
  c.select(INSTANCES[0]); await flush(); await time.tick(18000); assert.ok(states.at(-1).rate);
  hung = true; await time.tick(25000); assert.equal(states.at(-1).rate, null); assert.equal(states.at(-1).status, 'retrying');
  c.setLifecycle({ hidden: true }); resolve(200n); await flush(); assert.equal(states.at(-1).rate, null);
  assert.equal(states.at(-1).accepted, 100n); c.dispose();
});
