import test from 'node:test';
import assert from 'node:assert/strict';
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
import { HttpAgent } from '@icp-sdk/core/agent';
import { createOperatingStatusClient, latestOperatingReport, operatingStatusIdl } from '../public/operating-status-client.js';
import { INSTANCES } from '../public/instances.js';
import { ICP_API_HOST } from '../public/pricing-client.js';
import { deferred } from './helpers.mjs';
import { flush } from './counter-clock.mjs';

const live = INSTANCES[0], ledger = live.observedLedgerCanisterId;
const record = (idx, text, timestamp = 1760486400000000000n) => ({idx:BigInt(idx), timestamp_nanos:timestamp, content:new TextEncoder().encode(text)});
const health = (mode = 'Economy', cycles = '4450000000000') => `HEALTH day=20376 observed_ledger=${ledger} reader=ICP symbol=ICP neuron_support=true mode=${mode} liquid_cycles=${cycles} observed_cursor=12`;
const logs = {canister_log_records:[record(1, health())]};
const status = {cycles:4860000000000n};
const actor = () => ({canister_status:async () => status, fetch_canister_logs:async () => logs});

test('latest report is chosen by index, keeps explicit hysteretic mode and validates Ledger context', () => {
  const parsed = latestOperatingReport([
    record(5, 'POLL_MODE_CHANGE from=Economy to=Standard liquid_cycles=5000000000001'),
    record(2, health()), record(6, health().replace(ledger, 'other-ledger')),
    record(7, 'not a mode report'), record(8, health(), -1000000n),
    record(-1, health()), {...record(9, health()), content:new Uint8Array([255])},
  ], ledger);
  assert.equal(parsed.mode, 'Standard'); assert.equal(parsed.liquidCycles, 5000000000001n);
  assert.equal(parsed.idx, 5n); assert.equal(parsed.observedAt, '2025-10-15T00:00:00.000Z');
  // Balance alone must not promote an explicit Economy report.
  assert.equal(latestOperatingReport([record(1, health('Economy', '100000000000000'))], ledger).mode, 'Economy');
  for (const text of [health('Unknown'), health('Fast', '-1'), health().replace(ledger, 'other'), '']) {
    assert.throws(() => latestOperatingReport([record(1, text)], ledger), /No retained/);
  }
});

test('native queries use the reviewed backend, effective routing, fixed host and reused verified agent', async () => {
  let agents = 0, actors = 0, queries = 0;
  const client = createOperatingStatusClient({now:() => 'checked', createAgent:async options => {
    agents++; assert.equal(options.host, ICP_API_HOST); assert.equal(options.retryTimes, 0);
    assert.equal(options.fetchRootKey, undefined); assert.equal(options.verifyQuerySignatures, undefined);
    return {};
  }, createActor(factory, options) {
    actors++; assert.equal(options.canisterId, 'aaaaa-aa');
    assert.equal(options.effectiveCanisterId.toText(), live.backendCanisterId);
    const methods = factory({IDL})._fields;
    assert.deepEqual(methods.map(([name]) => name), ['canister_status', 'fetch_canister_logs']);
    return Object.fromEntries(methods.map(([name, type]) => [name, async (...args) => {
      queries++; assert.deepEqual(type.annotations, ['query']); assert.equal(args.length, 1);
      const [decoded] = IDL.decode([IDL.Record({canister_id:IDL.Principal})], IDL.encode(type.argTypes, args));
      assert.equal(decoded.canister_id.toText(), live.backendCanisterId);
      return name === 'canister_status' ? status : logs;
    }]));
  }});
  for (let i = 0; i < 2; i++) {
    const result = await client.query(live);
    assert.equal(result.balanceCycles, status.cycles); assert.equal(result.liquidCycles, 4450000000000n);
    assert.equal(result.reported, true); assert.equal(result.checkedAt, 'checked');
  }
  assert.equal(agents, 1); assert.equal(actors, 1); assert.equal(queries, 4); client.dispose();
});

test('real Candid decoder accepts independent native wire records and rejects incompatible scalars', () => {
  const methods = Object.fromEntries(operatingStatusIdl({IDL})._fields);
  // Independent wire fields, including compound fields omitted by the projection.
  const wireStatus = IDL.Record({cycles:IDL.Nat, reserved_cycles:IDL.Nat,
    status:IDL.Variant({running:IDL.Null, stopping:IDL.Null, stopped:IDL.Null}),
    module_hash:IDL.Opt(IDL.Vec(IDL.Nat8)), memory_size:IDL.Nat,
    query_stats:IDL.Record({num_calls_total:IDL.Nat, num_instructions_total:IDL.Nat,
      request_payload_bytes_total:IDL.Nat, response_payload_bytes_total:IDL.Nat}),
  });
  const full = {cycles:1n << 100n, reserved_cycles:10n, status:{running:null}, module_hash:[new Uint8Array(32)], memory_size:12n,
    query_stats:{num_calls_total:1n, num_instructions_total:2n, request_payload_bytes_total:3n, response_payload_bytes_total:4n}};
  assert.deepEqual(IDL.decode(methods.canister_status.retTypes, IDL.encode([wireStatus], [full])), [{cycles:1n << 100n}]);
  const wireLogs = IDL.Record({canister_log_records:IDL.Vec(IDL.Record({idx:IDL.Nat64, timestamp_nanos:IDL.Nat64, content:IDL.Vec(IDL.Nat8)}))});
  for (const reply of [logs, {canister_log_records:[]}]) {
    const [decoded] = IDL.decode(methods.fetch_canister_logs.retTypes, IDL.encode([wireLogs], [reply]));
    assert.deepEqual(decoded, reply);
  }
  for (const bytes of [new Uint8Array([1, 2, 3]), IDL.encode([IDL.Record({cycles:IDL.Nat64})], [{cycles:0n}]),
    IDL.encode([IDL.Record({})], [{}])]) assert.throws(() => IDL.decode(methods.canister_status.retTypes, bytes));
  const request = methods.fetch_canister_logs.argTypes;
  const range = IDL.Record({start:IDL.Nat64, end:IDL.Nat64});
  const nativeRequest = IDL.Record({canister_id:IDL.Principal,
    filter:IDL.Opt(IDL.Variant({by_idx:range, by_timestamp_nanos:range}))});
  const [decoded] = IDL.decode([nativeRequest], IDL.encode(request, [{canister_id:Principal.fromText(live.backendCanisterId)}]));
  assert.deepEqual(decoded.filter, []); // Omitted optional fields stay absent, not fabricated nulls.
});

test('planned, private/unsupported status, missing reports and invalid balances fail without fallback', async () => {
  let agents = 0;
  const client = createOperatingStatusClient({createAgent:async () => {agents++; return {};}, createActor:actor});
  await assert.rejects(client.query(INSTANCES[1])); assert.equal(agents, 0); client.dispose();
  for (const badActor of [
    {}, {...actor(), canister_status:async () => {throw Error('private');}},
    {...actor(), fetch_canister_logs:async () => ({canister_log_records:[]})},
    ...[{}, {cycles:-1n}, {cycles:1}].map(reply => ({...actor(), canister_status:async () => reply})),
  ]) {
    const c = createOperatingStatusClient({createAgent:async () => ({}), createActor:() => badActor});
    await assert.rejects(c.query(live)); c.dispose();
  }
});

test('initialization failure is evicted; overlapping logical calls fail and the next attempt recovers', async () => {
  let agents = 0; const pending = deferred();
  const c = createOperatingStatusClient({createAgent:async () => {if (++agents === 1) throw Error('init'); return pending.promise;}, createActor:actor});
  await assert.rejects(c.query(live), /init/);
  const second = c.query(live); await flush(); await assert.rejects(c.query(live), /single flight/);
  pending.resolve({}); assert.equal((await second).balanceCycles, status.cycles);
  await c.query(live); assert.equal(agents, 2); c.dispose();
});

for (const phase of ['initialization', 'query']) test(`abort during ${phase} cancels owned transport and prevents orphan retries`, async () => {
  let liveFetches = 0, calls = 0, firstTransport, agents = 0;
  const c = createOperatingStatusClient({fetchImpl:(_url, {signal}) => {
    calls++; liveFetches++;
    return new Promise((_, reject) => signal.addEventListener('abort', () => {liveFetches--; reject(Error('aborted'));}, {once:true}));
  }, createAgent:async options => {
    agents++; firstTransport ??= options.fetch;
    if (agents === 1 && phase === 'initialization') await options.fetch('https://icp-api.io/init');
    return options;
  }, createActor:(_factory, {agent}) => ({...actor(), canister_status:async () => {
    if (agents === 1) await agent.fetch('https://icp-api.io/query'); return status;
  }})});
  const abort = new AbortController(), pending = c.query(live, {signal:abort.signal});
  await flush(); assert.equal(liveFetches, 1); abort.abort(); await assert.rejects(pending); assert.equal(liveFetches, 0);
  assert.equal((await c.query(live)).balanceCycles, status.cycles);
  assert.throws(() => firstTransport('https://icp-api.io/late-retry'), /abort/i);
  assert.equal(calls, 1); c.dispose();
});

for (const phase of ['initialization', 'query']) for (const terminal of ['resolve', 'reject']) test(`retired ${phase} late ${terminal} cannot evict the newer session`, async () => {
  let agents = 0; const late = deferred(), abort = new AbortController();
  const c = createOperatingStatusClient({createAgent:async () => {agents++; return phase === 'initialization' && agents === 1 ? late.promise : {};},
    createActor:() => ({...actor(), canister_status:async () => phase === 'query' && agents === 1 ? late.promise : status})});
  const first = c.query(live, {signal:abort.signal}); await flush(); abort.abort(); await assert.rejects(first);
  await c.query(live); late[terminal](terminal === 'resolve' ? (phase === 'initialization' ? {} : status) : Error('late'));
  await flush(); await c.query(live); assert.equal(agents, 2); c.dispose();
});

test('pinned agent retains its embedded root key and signature verification; abort stops all query work', async () => {
  let calls = 0;
  const c = createOperatingStatusClient({createAgent:async options => {
    const agent = await HttpAgent.create(options); assert.equal(agent.rootKey.length, 133);
    assert.equal(agent.config.verifyQuerySignatures, undefined); return agent;
  }, fetchImpl:(_url, {signal}) => {
    calls++;
    return new Promise((_, reject) => signal.addEventListener('abort', () => reject(Error('aborted')), {once:true}));
  }});
  const abort = new AbortController(), pending = c.query(live, {signal:abort.signal}); await flush();
  assert.ok(calls > 0 && calls <= 3); abort.abort(); await assert.rejects(pending); await flush();
  const stopped = calls; await flush(); assert.equal(calls, stopped); c.dispose();
});
