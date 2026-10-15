import {Actor, HttpAgent} from '@icp-sdk/core/agent';
import {Principal} from '@icp-sdk/core/principal';
import {ICP_API_HOST} from './pricing-client.js';
import {tierProgress} from './operating-tier.js';

// Native management queries only: no canister update, wallet or backend endpoint.
export const operatingStatusIdl = ({IDL}) => {
  const request = IDL.Record({canister_id:IDL.Principal});
  return IDL.Service({
    canister_status: IDL.Func([request], [IDL.Record({cycles:IDL.Nat})], ['query']),
    fetch_canister_logs: IDL.Func([request], [IDL.Record({canister_log_records:IDL.Vec(IDL.Record({
      idx:IDL.Nat64, timestamp_nanos:IDL.Nat64, content:IDL.Vec(IDL.Nat8),
    }))})], ['query']),
  });
};
const modes = 'ReserveProtection|Economy|Standard|Fast|VeryFast|Continuous';
export function latestOperatingReport(records, observedLedger) {
  let latest = null;
  for (const record of records) {
    if (typeof record.idx !== 'bigint' || record.idx < 0n || typeof record.timestamp_nanos !== 'bigint') continue;
    let text;
    try { text = new TextDecoder('utf-8', {fatal:true}).decode(Uint8Array.from(record.content)); } catch { continue; }
    if (text.startsWith('HEALTH ') && !text.split(/\s+/).includes(`observed_ledger=${observedLedger}`)) continue;
    const match = text.startsWith('HEALTH ')
      ? text.match(new RegExp(`(?:^| )mode=(${modes}) liquid_cycles=([0-9]+)(?: |$)`))
      : text.match(new RegExp(`^POLL_MODE_CHANGE from=(?:${modes}) to=(${modes}) liquid_cycles=([0-9]+)$`));
    if (!match || (latest && record.idx <= latest.idx)) continue;
    const mode = match[1], liquidCycles = BigInt(match[2]);
    tierProgress(mode, liquidCycles);
    const millis = record.timestamp_nanos / 1_000_000n;
    if (millis < 0n || millis > 8_640_000_000_000_000n) continue;
    latest = {idx:record.idx, mode, liquidCycles, observedAt:new Date(Number(millis)).toISOString()};
  }
  if (!latest) throw Error('No retained operating-tier report.');
  return latest;
}
export function createOperatingStatusClient({createAgent = options => HttpAgent.create(options),
  createActor = (factory, options) => Actor.createActor(factory, options), fetchImpl = (...args) => fetch(...args),
  now = () => new Date().toISOString()} = {}) {
  let session = null;
  const retire = owned => { owned?.abort.abort(); if (session === owned) session = null; };
  return {
    async query(entry, {signal} = {}) {
      signal?.throwIfAborted();
      if (entry?.status !== 'live' || !entry.backendCanisterId) throw Error('Instance not live.');
      if (session?.key !== entry.backendCanisterId) retire(session);
      if (!session) session = {key:entry.backendCanisterId, abort:new AbortController(), ready:null, attempt:null};
      const owned = session;
      if (owned.attempt) throw Error('Operating status requests must be single flight.');
      const attempt = {signal};
      owned.attempt = attempt;
      let onAbort;
      const aborted = new Promise((_, reject) => {
        onAbort = () => { retire(owned); reject(new DOMException('Operating status request retired.', 'AbortError')); };
        signal?.addEventListener('abort', onAbort, {once:true});
      });
      try {
        if (!owned.ready) owned.ready = Promise.resolve().then(() => createAgent({host:ICP_API_HOST,retryTimes:0,
          fetch:(url,options={}) => {
            owned.abort.signal.throwIfAborted();
            if (!owned.attempt) throw new DOMException('Operating status request retired.', 'AbortError');
            owned.attempt.signal?.throwIfAborted();
            return fetchImpl(url,{...options,signal:owned.abort.signal});
          },
        })).then(agent => {
          owned.abort.signal.throwIfAborted();
          return createActor(operatingStatusIdl,{agent,canisterId:'aaaaa-aa',effectiveCanisterId:Principal.fromText(entry.backendCanisterId)});
        });
        const actor = await Promise.race([owned.ready, aborted]);
        owned.abort.signal.throwIfAborted();
        const request = {canister_id:Principal.fromText(entry.backendCanisterId)};
        owned.abort.signal.throwIfAborted();
        const [balance, logs] = await Promise.race([
          Promise.all([actor.canister_status(request),actor.fetch_canister_logs(request)]), aborted,
        ]);
        owned.abort.signal.throwIfAborted(); signal?.throwIfAborted();
        if (typeof balance.cycles !== 'bigint' || balance.cycles < 0n) throw Error('Invalid cycles balance.');
        return {...latestOperatingReport(logs.canister_log_records,entry.observedLedgerCanisterId),balanceCycles:balance.cycles,reported:true,checkedAt:now()};
      } catch (error) { retire(owned); throw error; }
      finally { signal?.removeEventListener('abort', onAbort); if (owned.attempt === attempt) owned.attempt = null; }
    },
    dispose() { retire(session); },
  };
}
