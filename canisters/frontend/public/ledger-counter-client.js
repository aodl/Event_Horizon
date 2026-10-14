import { Actor, HttpAgent } from '@icp-sdk/core/agent';
import { ICP_API_HOST } from './pricing-client.js';

export const COUNT_SOURCES = Object.freeze(['icp-chain-length', 'icrc3-log-length']);
export const eligibleLedger = entry => entry?.status === 'live'
  && Boolean(entry.observedLedgerCanisterId) && COUNT_SOURCES.includes(entry.ledgerCountSource);

// Record width subtyping discards the other wire fields, including archive callbacks.
// The real SDK decoder is tested against independent complete response schemas.
export function ledgerCountIdl(source, IDL) {
  const icp = source === 'icp-chain-length';
  if (!COUNT_SOURCES.includes(source)) throw new Error('Unsupported ledger count source.');
  const range = IDL.Record({ start: icp ? IDL.Nat64 : IDL.Nat, length: icp ? IDL.Nat64 : IDL.Nat });
  return IDL.Service({ [icp ? 'query_blocks' : 'icrc3_get_blocks']: IDL.Func(
    [icp ? range : IDL.Vec(range)],
    [IDL.Record(icp ? { chain_length: IDL.Nat64 } : { log_length: IDL.Nat })], ['query'],
  ) });
}

export function validateTotal(total, source) {
  if (typeof total !== 'bigint' || total < 0n
      || (source === 'icp-chain-length' && total > (1n << 64n) - 1n)) {
    throw new Error('Invalid ledger total.');
  }
  return total;
}

const abortError = () => new DOMException('Ledger request retired.', 'AbortError');

export function createLedgerCounterClient({
  createAgent = options => HttpAgent.create(options),
  createActor = (factory, options) => Actor.createActor(factory, options),
  fetchImpl = (...args) => globalThis.fetch(...args),
} = {}) {
  let session = null;
  function retire(owned) {
    owned?.lifetime.abort();
    if (session === owned) session = null;
  }
  async function query(entry, { signal } = {}) {
    if (!eligibleLedger(entry)) throw new Error('Ledger count unavailable for this instance.');
    signal?.throwIfAborted();
    const key = `${entry.observedLedgerCanisterId}/${entry.ledgerCountSource}`;
    if (session?.key !== key) retire(session);
    if (!session) session = { key, lifetime: new AbortController(), attempt: null, ready: null };
    const owned = session;
    if (owned.attempt) throw new Error('Ledger counter requests must be single flight.');
    const attempt = { signal };
    owned.attempt = attempt;
    let onAbort;
    const aborted = new Promise((_, reject) => {
      onAbort = () => { retire(owned); reject(abortError()); };
      signal?.addEventListener('abort', onAbort, { once: true });
    });
    try {
      if (!owned.ready) {
        // Every transport request (including initialization and signature-key reads)
        // belongs to this session. An aborted session can never borrow a later signal.
        const transport = (url, options = {}) => {
          owned.lifetime.signal.throwIfAborted();
          const activeSignal = owned.attempt?.signal;
          if (!owned.attempt) throw abortError();
          activeSignal?.throwIfAborted();
          return fetchImpl(url, { ...options, signal: owned.lifetime.signal });
        };
        owned.ready = Promise.resolve().then(() => createAgent({
          host: ICP_API_HOST, fetch: transport, retryTimes: 0,
        })).then(agent => {
          owned.lifetime.signal.throwIfAborted();
          return createActor(({ IDL }) => ledgerCountIdl(entry.ledgerCountSource, IDL), {
            agent, canisterId: entry.observedLedgerCanisterId,
          });
        });
      }
      const actor = await Promise.race([owned.ready, aborted]);
      signal?.throwIfAborted();
      owned.lifetime.signal.throwIfAborted();
      const range = { start: 0n, length: 0n };
      const icp = entry.ledgerCountSource === 'icp-chain-length';
      const response = await Promise.race([
        icp ? actor.query_blocks(range) : actor.icrc3_get_blocks([range]), aborted,
      ]);
      signal?.throwIfAborted();
      owned.lifetime.signal.throwIfAborted();
      return validateTotal(response?.[icp ? 'chain_length' : 'log_length'], entry.ledgerCountSource);
    } catch (error) {
      // Also abort sibling SDK work after a query or verification failure.
      retire(owned);
      throw error;
    } finally {
      signal?.removeEventListener('abort', onAbort);
      if (owned.attempt === attempt) owned.attempt = null;
    }
  }
  return { query, dispose: () => retire(session) };
}
