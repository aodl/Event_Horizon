import test from 'node:test';
import assert from 'node:assert/strict';
import { setTimeout as delay } from 'node:timers/promises';
import { Actor } from '@icp-sdk/core/agent';
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
import {
  createBoundedFetch, createIcpClient, identityFromSeed, readPrefix, validateInstance,
} from '../src/icp.mjs';

const config = {
  backend: 'eo6ei-gaaaa-aaaar-qchra-cai', ledger: 'ryjl3-tyaaa-aaaaa-aaaba-cai',
  faucet: 'acjuz-liaaa-aaaar-qb4qq-cai', historian: 'j5gs6-uiaaa-aaaar-qb5cq-cai',
  governance: 'rrkah-fqaaa-aaaaa-aaaaq-cai', host: 'https://icp-api.io',
};
const principal = text => Principal.fromText(text);
const callback = [principal('r5m5y-diaaa-aaaaa-qanaa-cai'), 'get_blocks'];
const seed = Buffer.from(Array.from({ length: 32 }, (_, i) => i + 1)).toString('base64');
const block = index => ({ index });
const response = (overrides = {}) => ({
  chain_length: 20n, first_block_index: 10n, blocks: [block(10), block(11)],
  archived_blocks: [], ...overrides,
});
const prefix = (reply, extra = {}) => readPrefix(reply, {
  start: 10n, length: 8n,
  queryArchive: () => assert.fail('Unexpected archive query'),
  ...extra,
});

function instance() {
  return {
    observed_ledger: principal(config.ledger), icp_ledger: principal(config.ledger),
    jupiter_faucet: principal(config.faucet), jupiter_historian: principal(config.historian),
    sns_root: [], icp_admission_notifier: [],
    observed_profile: [{
      symbol: 'ICP', decimals: 8, supports_icrc2_transfer_from: true,
      neuron_governance: [principal(config.governance)],
    }],
  };
}

test('worker identity is deterministic, distinct from anonymous, and rejects malformed seeds', () => {
  assert.equal(identityFromSeed(seed).getPrincipal().toText(),
    'ro3zk-qqs5u-lntt3-rz2jc-iuhjc-e6a25-gjzrq-l7vml-phczr-uaisn-6qe');
  assert.equal(identityFromSeed(undefined), undefined);
  for (const bad of ['', 'invalid', `${seed}\n`, Buffer.alloc(32).toString('base64'),
    Buffer.alloc(31, 1).toString('base64'), 123]) {
    assert.throws(() => identityFromSeed(bad), /seed/);
  }
});

test('instance verification checks canonical ledger and protocol anchors', () => {
  assert.deepEqual(validateInstance(instance(), config), instance());
  assert.doesNotThrow(() => validateInstance({ ...instance(), observed_profile: [] }, config));
  for (const field of ['observed_ledger', 'icp_ledger', 'jupiter_faucet', 'jupiter_historian']) {
    assert.throws(() => validateInstance({ ...instance(), [field]: principal(config.backend) }, config),
      /protocol anchor/);
  }
  for (const field of ['sns_root', 'icp_admission_notifier']) {
    assert.throws(() => validateInstance({ ...instance(), [field]: [principal(config.backend)] }, config),
      /canonical ICP/);
  }
  for (const profile of [
    { ...instance().observed_profile[0], decimals: 10 },
    { ...instance().observed_profile[0], neuron_governance: [] },
    { ...instance().observed_profile[0], neuron_governance: [principal(config.backend)] },
  ]) {
    assert.throws(() => validateInstance({ ...instance(), observed_profile: [profile] }, config),
      /token\/governance/);
  }
});

test('root short prefixes and fixed scan boundaries retain exact contiguous coverage', async () => {
  assert.deepEqual(await prefix(response()), { chainLength: 20n, blocks: [block(10), block(11)] });
  const full = response({ blocks: [10, 11, 12, 13].map(block) });
  assert.deepEqual((await prefix(full, { boundary: 12n })).blocks, [block(10), block(11)]);
  assert.deepEqual((await prefix(full, { boundary: 10n })).blocks, []);
});

test('empty root first index is ignored and an archive short prefix advances only actual blocks', async () => {
  const calls = [];
  const result = await prefix(response({
    blocks: [], first_block_index: (1n << 64n) - 1n,
    archived_blocks: [{ start: 10n, length: 8n, callback }],
  }), {
    queryArchive: async (ref, start, length) => {
      calls.push({ ref, start, length });
      return { Ok: { blocks: [block(10), block(11)] } };
    },
  });
  assert.deepEqual(calls, [{ ref: callback, start: 10n, length: 8n }]);
  assert.deepEqual(result, { chainLength: 20n, blocks: [block(10), block(11)] });
});

test('archive queries are clipped to the captured boundary and advertised range', async () => {
  for (const [rangeLength, boundary, expected] of [[8n, 12n, 2n], [3n, 18n, 3n]]) {
    const result = await prefix(response({
      first_block_index: 16n, blocks: rangeLength < 8n ? [block(16), block(17)] : [],
      archived_blocks: [{ start: 10n, length: rangeLength, callback }],
    }), {
      boundary,
      queryArchive: async (ref, start, length) => {
        assert.deepEqual(ref, callback);
        assert.equal(start, 10n);
        assert.equal(length, expected);
        return { Ok: { blocks: Array.from({ length: Number(length) }, (_, i) => block(10 + i)) } };
      },
    });
    assert.equal(BigInt(result.blocks.length), expected);
  }
});

test('an archive boundary yields one prefix and does not invent a second callback', async () => {
  let calls = 0;
  const result = await prefix(response({
    blocks: [], first_block_index: 999n,
    archived_blocks: [
      { start: 10n, length: 2n, callback },
      { start: 12n, length: 6n, callback: [callback[0], 'another_archive_query'] },
    ],
  }), {
    queryArchive: async (ref, start, length) => {
      calls++;
      assert.deepEqual(ref, callback);
      assert.equal(start, 10n);
      assert.equal(length, 2n);
      return { Ok: { blocks: [block(10), block(11)] } };
    },
  });
  assert.equal(calls, 1);
  assert.equal(result.blocks.length, 2);
});

test('holes, stale tips, overlapping coverage and malformed ranges fail without skipping', async () => {
  const badReplies = [
    response({ first_block_index: 12n }),
    response({ chain_length: 9n, blocks: [] }),
    response({ first_block_index: 9n }),
    response({ blocks: Array.from({ length: 9 }, (_, i) => block(10 + i)) }),
    response({ blocks: [], archived_blocks: [
      { start: 10n, length: 4n, callback }, { start: 12n, length: 3n, callback },
    ] }),
    response({ archived_blocks: [{ start: 10n, length: 2n, callback }] }),
    response({ blocks: [], archived_blocks: [{ start: 10n, length: 0n, callback }] }),
    response({ blocks: [], archived_blocks: [{ start: 9n, length: 2n, callback }] }),
    response({ blocks: [], archived_blocks: [{ start: 10n, length: 9n, callback }] }),
    response({ blocks: [], archived_blocks: [{ start: 10n, length: 2n,
      callback: [Principal.anonymous(), 'get_blocks'] }] }),
    response({ blocks: [], archived_blocks: [{ start: 10n, length: 2n,
      callback: [principal('aaaaa-aa'), 'get_blocks'] }] }),
  ];
  for (const reply of badReplies) await assert.rejects(prefix(reply));
});

test('archive errors and empty/oversized prefixes retain a retryable cursor', async () => {
  const reply = response({ blocks: [], archived_blocks: [{ start: 10n, length: 2n, callback }] });
  for (const answer of [
    { Err: { BadFirstBlockIndex: { requested_index: 10n, first_valid_index: 12n } } },
    { Err: { Other: { error_code: 1n, error_message: 'temporary outage' } } },
    { Ok: { blocks: [] } }, { Ok: { blocks: [1, 2, 3] } },
  ]) {
    await assert.rejects(prefix(reply, { queryArchive: async () => answer }), /archive/);
  }
  await assert.rejects(prefix(reply, { queryArchive: async () => { throw Error('offline'); } }), /offline/);
});

test('cursor arithmetic remains exact above Number.MAX_SAFE_INTEGER and at nat64 limit', async () => {
  const max = (1n << 64n) - 1n;
  const result = await readPrefix(response({
    chain_length: max, first_block_index: max - 2n, blocks: [block('a'), block('b')],
  }), { start: max - 2n, length: 2n });
  assert.equal(result.chainLength, max);
  assert.equal(result.blocks.length, 2);
  await assert.rejects(readPrefix(response(), { start: 10, length: 1n }), /BigInt/);
  await assert.rejects(readPrefix(response({ chain_length: max }), { start: max, length: 1n }), /overflows/);
});

test('idle ledger returns empty coverage without interpreting an unspecified root first index', async () => {
  const result = await prefix(response({ chain_length: 10n, blocks: [], first_block_index: 999n }));
  assert.deepEqual(result, { chainLength: 10n, blocks: [] });
});

test('transport forwards a deadline and aborts actual fetch work', async () => {
  let observed;
  const fetchImpl = async (_url, options) => {
    observed = options;
    await delay(1_000, undefined, { signal: options.signal });
    assert.fail('The timed-out fetch continued');
  };
  const fetch = createBoundedFetch(fetchImpl, { timeoutMs: 20 });
  await assert.rejects(fetch('https://icp-api.io', { method: 'POST' }), /abort/i);
  assert.equal(observed.method, 'POST');
  assert.equal(observed.signal.aborted, true);
});

test('invocation cancellation cannot borrow another invocation signal', async () => {
  const controller = new AbortController();
  let calls = 0;
  const fetch = createBoundedFetch(async () => { calls++; }, { signal: controller.signal });
  controller.abort();
  await assert.rejects(fetch('https://icp-api.io'), /abort/i);
  assert.equal(calls, 0);
});

test('anonymous check client rejects poke before making any network request', async () => {
  let calls = 0;
  const client = await createIcpClient(config, { fetchImpl: async () => { calls++; } });
  assert.equal(client.principal, Principal.anonymous().toText());
  await assert.rejects(client.poke(), /identity/);
  assert.equal(calls, 0);
});

test('client verifies queried configuration, preserves full-width cursor and exact raw route memo', async t => {
  const requests = [];
  const agents = [];
  t.mock.method(Actor, 'createActor', (factory, options) => {
    agents.push(options.agent);
    const methods = factory({ IDL })._fields.map(([name]) => name);
    if (methods.includes('query_blocks')) return {
      query_blocks: async range => {
        requests.push(['ledger', range]);
        return response({ chain_length: 1n << 60n, blocks: [], first_block_index: 0n });
      },
    };
    if (methods.includes('get_commitment_route_summaries')) return {
      get_commitment_route_summaries: async args => { requests.push(['route', args]); return { items: [] }; },
    };
    return { get_instance: async () => instance(), poke: async (...args) => requests.push(['poke', args]) };
  });
  const client = await createIcpClient({ ...config, seedBase64: seed });
  assert.deepEqual(await client.getInstance(), instance());
  assert.equal(await client.tip(), 1n << 60n);
  const memo = new Uint8Array([65, 46, 48]);
  await client.routeSummary(memo);
  await client.poke();
  assert.deepEqual(requests[0], ['ledger', { start: 0n, length: 0n }]);
  const route = requests[1][1].routes[0].RawIcp;
  assert.equal(route.destination_canister_id.toText(), config.backend);
  assert.deepEqual(route.memo, memo);
  assert.deepEqual(requests[2], ['poke', []]);
  for (const agent of agents) {
    assert.equal(agent.config.shouldFetchRootKey, false);
    assert.equal(agent.config.verifyQuerySignatures, true);
    assert.equal(agent.config.useQueryNonces, true);
    assert.equal(agent.config.retryTimes, 0);
  }
  assert.notEqual(agents.at(-1), agents[0], 'Poke owns a separate immutable deadline');
});

test('poke deadline cancels its polling strategy promptly', async t => {
  t.mock.method(Actor, 'createActor', (_factory, options) => ({
    poke: async () => options.pollingOptions.strategy(),
  }));
  const client = await createIcpClient({ ...config, seedBase64: seed, pokeTimeoutMs: 20 });
  await assert.rejects(client.poke(), /abort/i);
});

test('client supports configured off-chain page sizes and clamps requests at the nat64 ceiling', async t => {
  const requests = [];
  t.mock.method(Actor, 'createActor', () => ({
    query_blocks: async range => {
      requests.push(range);
      return response({ chain_length: range.start, blocks: [] });
    },
  }));
  const client = await createIcpClient(config);
  for (const length of [1000n, 2000n]) {
    assert.deepEqual(await client.readPage(10n, length), { chainLength: 10n, blocks: [] });
    assert.deepEqual(requests.at(-1), { start: 10n, length });
  }
  const max = (1n << 64n) - 1n;
  await client.readPage(max, 1000n);
  assert.deepEqual(requests.at(-1), { start: max, length: 0n });
  await assert.rejects(client.readPage(10n, 2001n), /2000/);
});
