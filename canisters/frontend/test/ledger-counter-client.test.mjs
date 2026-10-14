import test from 'node:test';
import assert from 'node:assert/strict';
import { IDL } from '@icp-sdk/core/candid';
import { HttpAgent } from '@icp-sdk/core/agent';
import { createLedgerCounterClient, ledgerCountIdl } from '../public/ledger-counter-client.js';
import { INSTANCES } from '../public/instances.js';
import { ICP_API_HOST } from '../public/pricing-client.js';
import { icpWire, genericWire, icpReply, genericReply } from './ledger-wire.mjs';
import { deferred } from './helpers.mjs';
import { flush } from './counter-clock.mjs';
const live = INSTANCES[0];
const generic = { ...live, name: 'Reviewed', ledgerCountSource: 'icrc3-log-length' };
const methodType = source => ledgerCountIdl(source, IDL)._fields[0][1];

for (const [entry, wire, reply, field, method, scalar] of [
  [live, icpWire, icpReply, 'chain_length', 'query_blocks', IDL.Nat64],
  [generic, genericWire, genericReply, 'log_length', 'icrc3_get_blocks', IDL.Nat],
]) {
  test(`${method} uses reviewed principal/host, one zero-length argument and exact scalar types`, async () => {
    let agents = 0, actors = 0, calls = 0;
    const client = createLedgerCounterClient({ createAgent: async options => {
      agents++; assert.equal(options.host, ICP_API_HOST); assert.equal(options.retryTimes, 0);
      assert.equal(options.fetchRootKey, undefined); assert.equal(options.verifyQuerySignatures, undefined); return {};
    }, createActor(factory, options) {
      actors++; assert.equal(options.canisterId, entry.observedLedgerCanisterId);
      const signature = factory({ IDL })._fields[0][1];
      const range = IDL.Record({ start: scalar, length: scalar });
      const expected = method === 'query_blocks' ? range : IDL.Vec(range);
      return { async [method](...args) {
        calls++; assert.equal(args.length, 1);
        assert.deepEqual(IDL.decode([expected], IDL.encode(signature.argTypes, args)), method === 'query_blocks'
          ? [{ start: 0n, length: 0n }] : [[{ start: 0n, length: 0n }]]);
        return { [field]: 0n };
      } };
    } });
    for (let i = 0; i < 3; i++) assert.equal(await client.query(entry, { signal: new AbortController().signal }), 0n);
    assert.equal(agents, 1); assert.equal(actors, 1); assert.equal(calls, 3); client.dispose();
  });
  test(`${method} projection decodes independent full wire reply with archives and extra fields`, () => {
    for (const populated of [false, true]) for (const total of [0n, 1n << 53n, (1n << 64n) - 1n]) {
      const bytes = IDL.encode([wire], [reply(total, populated)]);
      const decoded = IDL.decode(methodType(entry.ledgerCountSource).retTypes, bytes);
      assert.deepEqual(decoded, [{ [field]: total }]);
    }
    if (method === 'icrc3_get_blocks') {
      const huge = 1n << 120n;
      assert.deepEqual(IDL.decode(methodType(entry.ledgerCountSource).retTypes,
        IDL.encode([wire], [reply(huge, true)])), [{ log_length: huge }]);
    }
  });
  test(`${method} rejects missing/wrong wire scalar and malformed Candid`, () => {
    const ret = methodType(entry.ledgerCountSource).retTypes;
    assert.throws(() => IDL.decode(ret, IDL.encode([IDL.Record({})], [{}])));
    assert.throws(() => IDL.decode(ret, IDL.encode([IDL.Record({ [field]: IDL.Text })], [{ [field]: '100' }])));
    assert.throws(() => IDL.decode(ret, new Uint8Array([1, 2, 3])));
    const wrong = scalar === IDL.Nat64 ? IDL.Nat : IDL.Nat64;
    assert.throws(() => IDL.decode(ret, IDL.encode([IDL.Record({ [field]: wrong })], [{ [field]: 10n }])));
  });
}

test('unsupported entries do not initialize; missing method/reply fail without adapter fallback', async () => {
  let init = 0;
  const client = createLedgerCounterClient({ createAgent: async () => { init++; return {}; }, createActor: () => ({}) });
  for (const entry of [INSTANCES[1], { ...live, ledgerCountSource: null }, { ...live, ledgerCountSource: 'bad' }]) await assert.rejects(client.query(entry));
  assert.equal(init, 0); await assert.rejects(client.query(live)); assert.equal(init, 1);
  for (const reply of [{}, { chain_length: -1n }, { chain_length: '100' }]) {
    const c = createLedgerCounterClient({ createAgent: async () => ({}), createActor: () => ({ query_blocks: async () => reply }) });
    await assert.rejects(c.query(live), /Invalid ledger total/); c.dispose();
  }
  client.dispose();
});

test('failed initialization is evicted and the next attempt recovers', async () => {
  let tries = 0;
  const client = createLedgerCounterClient({ createAgent: async () => { if (++tries === 1) throw Error('init'); return {}; },
    createActor: () => ({ query_blocks: async () => ({ chain_length: 100n }) }) });
  await assert.rejects(client.query(live), /init/); assert.equal(await client.query(live), 100n); assert.equal(tries, 2); client.dispose();
});

for (const phase of ['initialization', 'query']) test(`abort during ${phase} cancels owned fetch and prevents retry borrowing a new session`, async () => {
  let fetches = 0, inFlight = 0, max = 0, savedTransport, tries = 0;
  const abort = new AbortController();
  const client = createLedgerCounterClient({ fetchImpl: (_url, { signal }) => {
    fetches++; inFlight++; max = Math.max(max, inFlight);
    return new Promise((_, reject) => signal.addEventListener('abort', () => { inFlight--; reject(new DOMException('aborted', 'AbortError')); }, { once: true }));
  }, createAgent: async options => {
    tries++; savedTransport ??= options.fetch;
    if (tries === 1 && phase === 'initialization') await options.fetch('https://icp-api.io/init');
    return options;
  }, createActor: (_factory, { agent }) => ({ query_blocks: async () => {
    if (tries === 1) await agent.fetch('https://icp-api.io/query');
    return { chain_length: 200n };
  } }) });
  const pending = client.query(live, { signal: abort.signal }); await flush(); assert.equal(inFlight, 1);
  abort.abort(); await assert.rejects(pending); assert.equal(inFlight, 0);
  assert.equal(await client.query(live, { signal: new AbortController().signal }), 200n);
  assert.throws(() => savedTransport('https://icp-api.io/late-retry'), /abort/i);
  assert.equal(fetches, 1); assert.equal(max, 1); client.dispose();
});

for (const phase of ['initialization', 'query']) for (const terminal of ['resolve', 'reject']) test(`late ${terminal} during retired ${phase} cannot evict a newer initialized session`, async () => {
  const late = deferred(), abort = new AbortController(); let agents = 0;
  const client = createLedgerCounterClient({ createAgent: async () => { agents++; return phase === 'initialization' && agents === 1 ? late.promise : {}; },
    createActor: () => ({ query_blocks: async () => phase === 'query' && agents === 1 ? late.promise : { chain_length: 200n } }) });
  const first = client.query(live, { signal: abort.signal }); await flush(); abort.abort(); await assert.rejects(first);
  assert.equal(await client.query(live), 200n);
  late[terminal](terminal === 'resolve' ? (phase === 'query' ? { chain_length: 999n } : {}) : Error('late'));
  await flush(); assert.equal(await client.query(live), 200n); assert.equal(agents, 2); client.dispose();
});

test('actual pinned agent has verified mainnet defaults and zero transport retries on abort', async () => {
  let count = 0;
  const client = createLedgerCounterClient({ createAgent: async options => {
    const agent = await HttpAgent.create(options);
    assert.equal(agent.rootKey.length, 133); assert.equal(agent.config.verifyQuerySignatures, undefined);
    return agent;
  }, fetchImpl: (_url, { signal }) => {
    count++;
    return new Promise((_, reject) => signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')), { once: true }));
  } });
  const abort = new AbortController(); const pending = client.query(live, { signal: abort.signal }); await flush();
  // SDK query and signature-key read_state may run in parallel within ONE attempt.
  assert.ok(count >= 1 && count <= 2); abort.abort(); await assert.rejects(pending); await flush();
  const stopped = count; await flush(); assert.equal(count, stopped); client.dispose();
});

for (const phase of ['initialization', 'query']) test(`controller deadline cancels real owned ${phase} transport before scheduling another attempt`, async () => {
  const { createLedgerCounter } = await import('../public/ledger-counter.js');
  const { clock } = await import('./counter-clock.mjs');
  const t = clock(); let liveFetches = 0, requests = 0;
  const client = createLedgerCounterClient({ fetchImpl: (_url, { signal }) => {
    liveFetches++; requests++;
    return new Promise((_, reject) => signal.addEventListener('abort', () => { liveFetches--; reject(Error('aborted')); }, { once: true }));
  }, createAgent: async options => { if (phase === 'initialization') await options.fetch('https://icp-api.io/init'); return options; },
    createActor: (_factory, { agent }) => ({ query_blocks: async () => agent.fetch('https://icp-api.io/query') }) });
  const c = createLedgerCounter({ ...t, query: client.query, render() {} }); c.select(live); await flush();
  assert.equal(liveFetches, 1); await t.tick(10000); assert.equal(liveFetches, 0); assert.equal(requests, 1);
  await t.tick(30000); assert.equal(liveFetches, 1); assert.equal(requests, 2);
  c.dispose(); client.dispose(); await flush(); assert.equal(liveFetches, 0); assert.equal(t.timers.size, 0);
});
