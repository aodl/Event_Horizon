import { performance } from 'node:perf_hooks';
import { setTimeout as sleep } from 'node:timers/promises';

/** One-second start-to-start attempts, without overlapping scans. A slow
 * successful pass is followed immediately. Repeated failures back off to 30s. */
export async function pollLoop({
  run, signal, onError, intervalMs = 1000, backoffMaxMs = 30_000,
  clock = () => performance.now(), wait = (ms, s) => sleep(ms, undefined, { signal: s }),
}) {
  if (typeof run !== 'function') throw new Error('run must be a function');
  if (!Number.isSafeInteger(intervalMs) || intervalMs < 1) throw new Error('Invalid poll interval');
  if (!Number.isSafeInteger(backoffMaxMs) || backoffMaxMs < intervalMs) throw new Error('Invalid backoff cap');
  let failures = 0;
  while (!signal?.aborted) {
    const began = clock();
    try {
      await run();
      failures = 0;
    } catch (error) {
      failures = Math.min(failures + 1, 20);
      onError?.(error);
    }
    if (signal?.aborted) break;
    const elapsed = Math.max(0, clock() - began);
    const delay = failures
      ? Math.min(backoffMaxMs, intervalMs * 2 ** (failures - 1))
      : Math.max(0, intervalMs - elapsed);
    if (delay > 0) {
      try { await wait(delay, signal); }
      catch (error) { if (signal?.aborted) break; throw error; }
    }
  }
}
