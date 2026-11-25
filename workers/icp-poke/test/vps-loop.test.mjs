import test from 'node:test';
import assert from 'node:assert/strict';
import { pollLoop } from '../src/poll-loop.mjs';

test('normal scans use one-second start-to-start cadence without overlap', async () => {
  const shutdown = new AbortController();
  let now = 0;
  let active = false;
  const starts = [];
  const waits = [];
  await pollLoop({
    signal: shutdown.signal, clock: () => now,
    wait: async ms => { waits.push(ms); now += ms; },
    run: async () => {
      assert.equal(active, false);
      active = true;
      starts.push(now);
      await Promise.resolve();
      now += 225;
      active = false;
      if (starts.length === 4) shutdown.abort();
    },
  });
  assert.deepEqual(starts, [0, 1000, 2000, 3000]);
  assert.deepEqual(waits, [775, 775, 775]);
});

test('slow successful scans continue immediately instead of adding a one-second sleep', async () => {
  const shutdown = new AbortController();
  let now = 0;
  const starts = [];
  const waits = [];
  await pollLoop({ signal: shutdown.signal, clock: () => now,
    wait: async ms => { waits.push(ms); now += ms; },
    run: async () => { starts.push(now); now += 1700; if (starts.length === 3) shutdown.abort(); },
  });
  assert.deepEqual(starts, [0, 1700, 3400]);
  assert.deepEqual(waits, []);
});

test('consecutive failures back off to a cap and a successful scan resets cadence', async () => {
  const shutdown = new AbortController();
  let now = 0;
  let count = 0;
  const waits = [];
  const errors = [];
  await pollLoop({ signal: shutdown.signal, clock: () => now, backoffMaxMs: 4000,
    wait: async ms => { waits.push(ms); now += ms; },
    onError: error => errors.push(error.message),
    run: async () => {
      count += 1;
      if (count <= 4) throw new Error('gateway failed');
      now += 100;
      if (count === 6) shutdown.abort();
    },
  });
  assert.deepEqual(errors, Array(4).fill('gateway failed'));
  assert.deepEqual(waits, [1000, 2000, 4000, 4000, 900]);
});

test('shutdown interrupts an idle wait without starting another scan', async () => {
  const shutdown = new AbortController();
  let calls = 0;
  await pollLoop({ signal: shutdown.signal, run: async () => { calls += 1; },
    wait: async (_ms, signal) => { shutdown.abort(); signal.throwIfAborted(); },
  });
  assert.equal(calls, 1);
});
