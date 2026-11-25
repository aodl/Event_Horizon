import test from 'node:test';
import assert from 'node:assert/strict';
import { Principal } from '@icp-sdk/core/principal';
import { accountIdentifier, neuronSubaccount, numberedSubaccount } from '../src/accounts.mjs';
import { initialState, runPass, validateState } from '../src/core.mjs';

const config = {
  backend: 'eo6ei-gaaaa-aaaar-qchra-cai',
  ledger: 'ryjl3-tyaaa-aaaaa-aaaba-cai',
  faucet: 'acjuz-liaaa-aaaar-qb4qq-cai',
  historian: 'j5gs6-uiaaa-aaaar-qb5cq-cai',
  governance: 'rrkah-fqaaa-aaaaa-aaaaq-cai',
  startBlock: 100n,
  pageSize: 1000,
  maxPages: 4,
};
const subject = 'qaa6y-5yaaa-aaaaa-aaafa-cai';
const other = 'r5m5y-diaaa-aaaaa-qanaa-cai';
const principal = text => Principal.fromText(text);
const memo = text => Uint8Array.from(Buffer.from(text));
const account = (owner, number = 0n) => Buffer.from(accountIdentifier(owner, numberedSubaccount(number)), 'hex');
const block = (operation, text) => ({ transaction: {
  operation: operation === undefined ? [] : [operation],
  icrc1_memo: text === undefined ? [] : [typeof text === 'string' ? memo(text) : text],
  memo: 123n,
} });
const transfer = (to, amount = 10n, from = account(other), text) => block({ Transfer: {
  to, from, amount: { e8s: amount }, fee: { e8s: 10000n }, spender: [],
} }, text);
const payout = text => transfer(account(config.backend), 100n, account(config.faucet), text);
const preview = ({ scope = 'subaccount', owner = subject, start = 7n, end = start, minimum = 10n } = {}) => ({
  subscriber: principal(other),
  subject: scope === 'global' ? [] : [principal(owner)],
  scope: { [scope]: null },
  start: scope === 'global' ? [] : [start],
  end: scope === 'global' ? [] : [end],
  target_count: scope === 'global' ? 0 : Number(end - start + 1n),
  minimum_units: scope === 'global' || minimum === undefined ? [] : [minimum],
  alias_revision: [], aliases_refreshed_at: [],
});
const pricing = { initialized: true, current: { account_icp: 10n, range_icp: 20n, global_icp: 1000n } };
const summary = (rawMemo, total = 100_000_000_000n) => ({
  items: [{ route: { RawIcp: { destination_canister_id: principal(config.backend), memo: rawMemo } },
    total_qualifying_committed_e8s: total, qualifying_commitment_count: 1n }],
  complete_from_genesis: true, truncated: false, commitment_index_fault: [],
});

function fixture(blocks, { state = initialState(config), overrides = {}, settings = config } = {}) {
  const calls = { pokes: 0, saved: [], reads: [], previews: [], pricing: 0, summaries: [] };
  const client = {
    async readPage(start, length, boundary) {
      calls.reads.push({ start, length, boundary });
      return { chainLength: settings.startBlock + BigInt(blocks.length),
        blocks: blocks.slice(Number(start - settings.startBlock), Number(start - settings.startBlock + length)) };
    },
    async previewSubscription(text) { calls.previews.push(text); return { Ok: preview() }; },
    async pricing() { calls.pricing += 1; return pricing; },
    async routeSummary(rawMemo) { calls.summaries.push(rawMemo); return summary(rawMemo); },
    async poke() { calls.pokes += 1; },
    ...overrides,
  };
  const args = { state, config: settings, client, persist: async value => { calls.saved.push(structuredClone(value)); } };
  return { calls, args, run: extra => runPass({ ...args, ...extra }) };
}

test('unrelated and below-threshold transactions advance the checkpoint without waking', async () => {
  const state = initialState(config);
  state.accounts[accountIdentifier(subject, numberedSubaccount(7n))] = '10';
  const f = fixture([transfer(account(other, 7n), 10n), transfer(account(subject, 7n), 9n)], { state });
  const result = await f.run();
  assert.equal(result.poked, false);
  assert.equal(result.toBlock, '102');
  assert.equal(f.calls.saved.length, 1);
  assert.equal(state.nextBlock, '100');
  assert.equal(f.calls.pricing, 0);
});

test('matching uses individual amounts inclusively without floating-point loss', async () => {
  const state = initialState(config);
  const target = accountIdentifier(subject, numberedSubaccount((1n << 64n) - 1n));
  state.accounts[target] = '9007199254740993';
  const f = fixture([
    transfer(Buffer.from(target, 'hex'), 9007199254740992n),
    transfer(Buffer.from(target, 'hex'), 9007199254740993n),
    transfer(Buffer.from(target, 'hex'), (1n << 64n) - 1n),
  ], { state });
  assert.equal((await f.run()).poked, true);
  assert.equal(f.calls.pokes, 1);
  state.accounts[target] = (1n << 80n).toString();
  assert.equal((await fixture([transfer(Buffer.from(target, 'hex'), (1n << 64n) - 1n)], { state }).run()).poked, false);
});

test('mint, burn, approvals and empty operations do not satisfy specific interests', async () => {
  const state = initialState(config);
  const to = account(subject, 7n);
  state.accounts[Buffer.from(to).toString('hex')] = '0';
  const blocks = [block({ Mint: { to, amount: { e8s: 50n } } }),
    block({ Burn: { from: to, amount: { e8s: 50n } } }), block({ Approve: {} }), block(undefined)];
  assert.equal((await fixture(blocks, { state }).run()).poked, false);
  state.globalInterest = true;
  const f = fixture(blocks, { state });
  assert.equal((await f.run()).poked, true);
  assert.equal(f.calls.pokes, 1);
});

test('only the authentic default Faucet route discovers candidates', async () => {
  const f = fixture([
    transfer(account(config.backend), 100n, account(other), 'A.7'),
    transfer(account(config.backend, 1n), 100n, account(config.faucet), 'A.7'),
    transfer(account(config.backend), 100n, account(config.faucet, 1n), 'A.7'),
    transfer(account(config.governance), 100n, account(other), 'E.A.7'),
  ]);
  assert.equal((await f.run()).poked, false);
  assert.deepEqual(f.calls.previews, []);
  assert.deepEqual(f.calls.saved[0].accounts, {});
});

test('an authentic payout wakes even for invalid, absent, non-ASCII or oversized ICRC memos', async () => {
  const f = fixture([payout(undefined), payout(''), payout(new Uint8Array([255])), payout('x'.repeat(33)), payout('bad')], {
    overrides: { async previewSubscription() { return { Err: 'invalid declaration' }; } },
  });
  const result = await f.run();
  assert.equal(result.poked, true);
  assert.equal(result.candidateAccounts, 0);
  assert.equal(f.calls.summaries.length, 0);
  // The numeric legacy memo alone never creates a candidate.
  assert.equal(f.calls.saved[0].globalInterest, false);
});

test('funded candidates merge minimum thresholds and use the original exact route bytes', async () => {
  const seen = [];
  const f = fixture([payout('A.7:2'), payout('B.7:1'), payout('A.7:2')], {
    overrides: {
      async previewSubscription(text) { return { Ok: preview({ minimum: text === 'B.7:1' ? 100_000_000n : 200_000_000n }) }; },
      async routeSummary(rawMemo) { seen.push(Buffer.from(rawMemo).toString()); return summary(rawMemo); },
    },
  });
  await f.run();
  assert.deepEqual(seen, ['A.7:2', 'B.7:1']);
  assert.equal(f.calls.pricing, 1);
  assert.equal(f.calls.saved[0].accounts[accountIdentifier(subject, numberedSubaccount(7n))], '100000000');
  assert.equal(f.calls.pokes, 1);
});

test('ranges use the range price and the preview subject, including neuron accounts', async () => {
  for (const scope of ['subaccount', 'neuron_nonce']) {
    const make = total => fixture([payout(scope === 'subaccount' ? 'A@B.7-8' : 'A@B.n7-8')], {
      overrides: {
        async previewSubscription() { return { Ok: preview({ scope, start: 7n, end: 8n, minimum: 0n }) }; },
        async routeSummary(rawMemo) { return summary(rawMemo, total); },
      },
    });
    const belowRange = make(1_000_000_000n);
    assert.equal((await belowRange.run()).candidateAccounts, 0);
    const eligible = make(2_000_000_000n);
    assert.equal((await eligible.run()).candidateAccounts, 2);
    const expected = [7n, 8n].map(number => scope === 'subaccount'
      ? accountIdentifier(subject, numberedSubaccount(number))
      : accountIdentifier(config.governance, neuronSubaccount(subject, number)));
    assert.deepEqual(Object.keys(eligible.calls.saved[0].accounts), expected);
  }
});

test('an underfunded global payout does not make unrelated subsequent activity wake', async () => {
  const f = fixture([payout('A')], { overrides: {
    async previewSubscription() { return { Ok: preview({ scope: 'global' }) }; },
    async routeSummary(rawMemo) { return summary(rawMemo, 99_999_999_999n); },
  } });
  assert.equal((await f.run()).poked, true);
  const next = f.calls.saved[0];
  assert.equal(next.globalInterest, false);
  const second = fixture([payout('A'), block({ Mint: {} })], { state: next });
  assert.equal((await second.run()).poked, false);
});

test('a sufficiently funded global candidate persists and later non-transfer activity wakes', async () => {
  const f = fixture([payout('A')], { overrides: {
    async previewSubscription() { return { Ok: preview({ scope: 'global' }) }; },
  } });
  assert.equal((await f.run()).globalInterest, true);
  const next = fixture([payout('A'), block({ Approve: {} })], { state: f.calls.saved[0] });
  assert.equal((await next.run()).poked, true);
  assert.equal(next.calls.pricing, 0);
});

test('complete exact-route funding is required, including non-truncated fault-free evidence', async () => {
  const modifications = [
    r => { r.truncated = true; }, r => { r.complete_from_genesis = false; },
    r => { r.commitment_index_fault = [{}]; }, r => { r.items = []; },
    r => { r.items.push({ ...r.items[0] }); },
    r => { r.items[0].route.RawIcp.destination_canister_id = principal(other); },
    r => { r.items[0].route.RawIcp.memo = memo('other'); },
  ];
  for (const modify of modifications) {
    const f = fixture([payout('A.7')], { overrides: {
      async routeSummary(rawMemo) { const value = summary(rawMemo); modify(value); return value; },
    } });
    const result = await f.run();
    assert.equal(result.poked, true);
    assert.equal(result.candidateAccounts, 0);
  }
});

test('learned candidates affect later blocks and repeated scans preserve them', async () => {
  const f = fixture([transfer(account(subject, 7n)), payout('A.7')]);
  await f.run();
  const second = fixture([transfer(account(subject, 7n)), payout('A.7'), transfer(account(subject, 7n))], {
    state: f.calls.saved[0],
  });
  const result = await second.run();
  assert.equal(result.fromBlock, '102');
  assert.equal(result.poked, true);
  assert.deepEqual(second.calls.previews, []);
});

test('paging fixes the initial boundary and caps work without discarding the remaining backlog', async () => {
  const settings = { ...config, pageSize: 2, maxPages: 2 };
  const f = fixture(Array.from({ length: 5 }, () => block(undefined)), { settings });
  const first = await f.run();
  assert.equal(first.toBlock, '104');
  assert.equal(first.lagBlocks, '1');
  assert.deepEqual(f.calls.reads, [
    { start: 100n, length: 2n, boundary: undefined },
    { start: 102n, length: 2n, boundary: 105n },
  ]);
  const resumed = fixture(Array.from({ length: 5 }, () => block(undefined)), { settings, state: f.calls.saved[0] });
  assert.equal((await resumed.run()).toBlock, '105');
});

test('a scan does not extend its captured tip when new blocks arrive during pagination', async () => {
  const settings = { ...config, pageSize: 1 };
  let count = 0;
  const f = fixture([], { settings, overrides: {
    async readPage(start, length, boundary) {
      count += 1;
      assert.equal(start, 99n + BigInt(count));
      assert.equal(length, 1n);
      assert.equal(boundary, count === 1 ? undefined : 102n);
      return { chainLength: count === 1 ? 102n : 103n, blocks: [block(undefined)] };
    },
  } });
  const result = await f.run();
  assert.equal(count, 2);
  assert.equal(result.toBlock, '102');
  assert.equal(result.tip, '102');
});

test('ledger cursors above Number.MAX_SAFE_INTEGER remain exact through checkpointing', async () => {
  const settings = { ...config, startBlock: 9007199254740993n };
  const f = fixture([block(undefined), block(undefined)], { settings, state: initialState(settings) });
  const result = await f.run();
  assert.equal(result.fromBlock, '9007199254740993');
  assert.equal(result.toBlock, '9007199254740995');
  assert.equal(f.calls.saved[0].nextBlock, '9007199254740995');
});

test('elapsed scan budget checkpoints a completed prefix within a busy declaration page', async () => {
  let time = 0;
  const seen = [];
  const f = fixture(['A.7', 'A.8', 'A.9', 'A.10'].map(payout), { overrides: {
    async previewSubscription(text) {
      time += 6000;
      seen.push(text);
      return { Ok: preview({ start: BigInt(text.split('.')[1]) }) };
    },
  } });
  const first = await f.run({ now: () => time });
  assert.equal(first.toBlock, '102');
  assert.equal(first.blocksRead, 2);
  assert.equal(first.lagBlocks, '2');
  assert.equal(first.candidateAccounts, 2);
  assert.equal(f.calls.pokes, 1);
  assert.equal(f.calls.saved[0].nextBlock, '102');

  const second = await f.run({ state: f.calls.saved[0], now: () => time });
  assert.equal(second.fromBlock, '102');
  assert.equal(second.toBlock, '104');
  assert.equal(second.candidateAccounts, 4);
  assert.deepEqual(seen, ['A.7', 'A.8', 'A.9', 'A.10']);
  assert.equal(f.calls.pokes, 2);
});

test('an elapsed budget still completes the first block instead of endlessly replaying slow reads', async () => {
  let time = 0;
  const f = fixture([], { overrides: {
    async readPage() {
      time += 20_000;
      return { chainLength: 102n, blocks: [block(undefined), block(undefined)] };
    },
  } });
  const result = await f.run({ now: () => time });
  assert.equal(result.toBlock, '101');
  assert.equal(result.blocksRead, 1);
  assert.equal(result.lagBlocks, '1');
  assert.equal(f.calls.saved[0].nextBlock, '101');
});

test('an elapsed budget avoids starting another page once a complete block has been processed', async () => {
  let time = 0;
  let reads = 0;
  const f = fixture([], { settings: { ...config, pageSize: 1 }, overrides: {
    async readPage() {
      reads += 1;
      time += 20_000;
      return { chainLength: 102n, blocks: [block(undefined)] };
    },
  } });
  const result = await f.run({ now: () => time });
  assert.equal(result.toBlock, '101');
  assert.equal(reads, 1);
});

test('no-change tip checks do not rewrite the checkpoint or poke', async () => {
  const f = fixture([]);
  const result = await f.run();
  assert.equal(result.toBlock, '100');
  assert.equal(result.blocksRead, 0);
  assert.equal(f.calls.saved.length, 0);
  assert.equal(f.calls.pokes, 0);
});

test('failed poke and query transport leave the input and durable checkpoint untouched', async () => {
  for (const overrides of [
    { async poke() { throw new Error('poke failed'); } },
    { async previewSubscription() { throw new Error('preview failed'); } },
    { async routeSummary() { throw new Error('Historian failed'); } },
  ]) {
    const f = fixture([payout('A.7')], { overrides });
    const before = structuredClone(f.args.state);
    await assert.rejects(f.run(), /failed/);
    assert.deepEqual(f.args.state, before);
    assert.equal(f.calls.saved.length, 0);
  }
});

test('failed persistence can repeat an acknowledged wake safely on the next pass', async () => {
  const f = fixture([payout('A.7')]);
  await assert.rejects(f.run({ persist: async () => { throw new Error('storage failed'); } }), /storage failed/);
  assert.equal(f.calls.pokes, 1);
  assert.equal(f.args.state.nextBlock, '100');
  await f.run();
  assert.equal(f.calls.pokes, 2);
  assert.equal(f.calls.saved[0].nextBlock, '101');
});

test('dry run resolves and matches but performs no poke or checkpoint write', async () => {
  const f = fixture([payout('A.7')]);
  const before = structuredClone(f.args.state);
  const result = await f.run({ dryRun: true });
  assert.equal(result.wouldPoke, true);
  assert.equal(result.poked, false);
  assert.equal(result.candidateAccounts, 1);
  assert.equal(f.calls.saved.length, 0);
  assert.equal(f.calls.pokes, 0);
  assert.deepEqual(f.args.state, before);
});

test('missing, excessive or regressed ledger results cannot advance the checkpoint', async () => {
  for (const response of [
    { chainLength: 101n, blocks: [] },
    { chainLength: 99n, blocks: [] },
    { chainLength: 100n, blocks: [block(undefined)] },
    { chainLength: Number.MAX_SAFE_INTEGER, blocks: [] },
  ]) {
    const f = fixture([], { overrides: { async readPage() { return response; } } });
    await assert.rejects(f.run());
    assert.equal(f.calls.saved.length, 0);
  }
});

test('checkpoint validation rejects corruption and cross-instance reuse', () => {
  const corruptions = [
    s => { s.version = 2; }, s => { s.backend = other; }, s => { s.ledger = other; },
    s => { s.startBlock = '101'; }, s => { s.nextBlock = '99'; },
    s => { s.nextBlock = '0101'; }, s => { s.nextBlock = 101; },
    s => { s.nextBlock = (1n << 64n).toString(); },
    s => { s.globalInterest = 'false'; }, s => { s.extra = true; },
    s => { s.accounts['no-account'] = '0'; },
    s => { s.accounts[accountIdentifier(subject)] = '-1'; },
  ];
  for (const corrupt of corruptions) {
    const state = initialState(config);
    corrupt(state);
    assert.throws(() => validateState(state, config));
  }
  assert.equal(validateState(initialState(config), config).nextBlock, config.startBlock.toString());
});

test('independent workers replay from the same start and need no shared cursor', async () => {
  const blocks = [payout('A.7'), transfer(account(subject, 7n))];
  const first = fixture(blocks);
  const second = fixture(blocks);
  await first.run();
  assert.equal(second.args.state.nextBlock, '100');
  await second.run();
  assert.deepEqual(first.calls.saved, second.calls.saved);
  assert.equal(first.calls.pokes, 1);
  assert.equal(second.calls.pokes, 1);
});
