import test from 'node:test';
import assert from 'node:assert/strict';
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
import { archiveIdl, backendIdl, historianIdl, ledgerIdl } from '../src/idl.mjs';

// Independent full ledger wire fixture, following the existing
// canisters/frontend/test/ledger-wire.mjs and the official ICP .did files.
// This does not import the adapter's record definitions to encode the replies.
const blob = IDL.Vec(IDL.Nat8);
const tokens = IDL.Record({ e8s: IDL.Nat64 });
const timestamp = IDL.Record({ timestamp_nanos: IDL.Nat64 });
const operation = IDL.Variant({
  Mint: IDL.Record({ to: blob, amount: tokens }),
  Burn: IDL.Record({ from: blob, spender: IDL.Opt(blob), amount: tokens }),
  Transfer: IDL.Record({ from: blob, to: blob, amount: tokens, fee: tokens, spender: IDL.Opt(blob) }),
  Approve: IDL.Record({ from: blob, spender: blob, allowance_e8s: IDL.Int,
    allowance: tokens, fee: tokens, expires_at: IDL.Opt(timestamp), expected_allowance: IDL.Opt(tokens) }),
});
const transaction = IDL.Record({ memo: IDL.Nat64, icrc1_memo: IDL.Opt(blob),
  operation: IDL.Opt(operation), created_at_time: timestamp });
const block = IDL.Record({ parent_hash: IDL.Opt(blob), transaction, timestamp });
const range = IDL.Record({ start: IDL.Nat64, length: IDL.Nat64 });
const archiveError = IDL.Variant({
  BadFirstBlockIndex: IDL.Record({ requested_index: IDL.Nat64, first_valid_index: IDL.Nat64 }),
  Other: IDL.Record({ error_code: IDL.Nat64, error_message: IDL.Text }),
});
const archiveReply = IDL.Variant({ Ok: IDL.Record({ blocks: IDL.Vec(block) }), Err: archiveError });
const wire = IDL.Record({
  chain_length: IDL.Nat64, certificate: IDL.Opt(blob), blocks: IDL.Vec(block),
  first_block_index: IDL.Nat64, archived_blocks: IDL.Vec(IDL.Record({
    start: IDL.Nat64, length: IDL.Nat64, callback: IDL.Func([range], [archiveReply], ['query']),
  })), extra_future_field: IDL.Text,
});
const method = (factory, name) => factory({ IDL })._fields.find(([key]) => key === name)[1];
const encodeReply = value => IDL.encode([wire], [value]);
const decodeReply = bytes => IDL.decode(method(ledgerIdl, 'query_blocks').retTypes, bytes)[0];
const addr = new Uint8Array(32).fill(1);
const max = (1n << 64n) - 1n;
const wrapBlock = op => ({
  parent_hash: [], timestamp: { timestamp_nanos: max }, transaction: {
    memo: max, icrc1_memo: [new Uint8Array([65, 46, 48])],
    operation: op ? [op] : [], created_at_time: { timestamp_nanos: max - 1n },
  },
});

test('full official ledger replies decode including all operation variants and full nat64 values', () => {
  const blocks = [
    wrapBlock({ Transfer: { from: addr, to: addr, amount: { e8s: max },
      fee: { e8s: 10_000n }, spender: [addr] } }),
    wrapBlock({ Mint: { to: addr, amount: { e8s: 8n } } }),
    wrapBlock({ Burn: { from: addr, amount: { e8s: 7n }, spender: [] } }),
    wrapBlock({ Approve: { from: addr, spender: addr, allowance_e8s: -1n,
      allowance: { e8s: max }, fee: { e8s: 10_000n }, expires_at: [], expected_allowance: [] } }),
    wrapBlock(null),
  ];
  const original = {
    chain_length: max, certificate: [new Uint8Array([1, 2, 3])], blocks,
    first_block_index: max - 5n,
    archived_blocks: [{ start: 0n, length: 256n,
      callback: [Principal.fromText('r5m5y-diaaa-aaaaa-qanaa-cai'), 'get_blocks'] }],
    extra_future_field: 'ignored by width subtyping',
  };
  const decoded = decodeReply(encodeReply(original));
  assert.equal(decoded.chain_length, max);
  assert.equal(decoded.first_block_index, max - 5n);
  assert.deepEqual(decoded.blocks, blocks);
  assert.equal(decoded.archived_blocks[0].callback[1], 'get_blocks');
  assert.equal(Object.hasOwn(decoded, 'certificate'), false);
  assert.equal(Object.hasOwn(decoded, 'extra_future_field'), false);
});

test('empty root response retains unspecified first index without dropping archive callbacks', () => {
  const decoded = decodeReply(encodeReply({
    chain_length: 20n, certificate: [], blocks: [], first_block_index: max,
    archived_blocks: [{ start: 10n, length: 8n,
      callback: [Principal.fromText('r5m5y-diaaa-aaaaa-qanaa-cai'), 'get_blocks'] }],
    extra_future_field: '',
  }));
  assert.equal(decoded.first_block_index, max);
  assert.equal(decoded.archived_blocks[0].start, 10n);
  assert.equal(decoded.archived_blocks[0].length, 8n);
});

test('official archive request/result variants interoperate with an advertised method name', () => {
  const api = method(archiveIdl('advertised_query'), 'advertised_query');
  assert.deepEqual(IDL.decode([range], IDL.encode(api.argTypes, [{ start: max - 1n, length: 1n }])),
    [{ start: max - 1n, length: 1n }]);
  for (const reply of [
    { Ok: { blocks: [wrapBlock(null)] } },
    { Err: { BadFirstBlockIndex: { requested_index: 1n, first_valid_index: 3n } } },
    { Err: { Other: { error_code: max, error_message: 'not ready' } } },
  ]) {
    assert.deepEqual(IDL.decode(api.retTypes, IDL.encode([archiveReply], [reply]))[0], reply);
  }
});

test('worker poke encodes genuinely zero arguments and expects a regular certified update reply', () => {
  const poke = method(backendIdl, 'poke');
  assert.deepEqual(poke.argTypes, []);
  assert.deepEqual(poke.retTypes, []);
  assert.deepEqual(poke.annotations, []);
  assert.equal(Buffer.from(IDL.encode(poke.argTypes, [])).toString('hex'), '4449444c0000');
  assert.deepEqual(IDL.decode(poke.retTypes, IDL.encode([], [])), []);
});

test('Historian wire retains completeness, faults, exact memo and full-width route totals', () => {
  // Independently mirrors candid/jupiter-historian.subset.did.
  const route = IDL.Variant({
    RawIcp: IDL.Record({ destination_canister_id: IDL.Principal, memo: blob }),
    CyclesTopUp: IDL.Record({ canister_id: IDL.Principal }),
    NeuronStake: IDL.Record({ neuron_id: IDL.Nat64, memo: IDL.Opt(blob) }),
  });
  const fault = IDL.Record({ observed_at_ts: IDL.Nat64,
    last_cursor_tx_id: IDL.Opt(IDL.Nat64), offending_tx_id: IDL.Nat64, message: IDL.Text });
  const replyType = IDL.Record({ items: IDL.Vec(IDL.Record({ route,
    qualifying_commitment_count: IDL.Nat64, total_qualifying_committed_e8s: IDL.Nat64 })),
  truncated: IDL.Bool, complete_from_genesis: IDL.Bool, commitment_index_fault: IDL.Opt(fault) });
  const reply = { items: [{ route: { RawIcp: {
    destination_canister_id: Principal.fromText('eo6ei-gaaaa-aaaar-qchra-cai'),
    memo: new Uint8Array([65, 46, 48]),
  } }, qualifying_commitment_count: max, total_qualifying_committed_e8s: max }],
  truncated: true, complete_from_genesis: false,
  commitment_index_fault: [{ observed_at_ts: 1n, last_cursor_tx_id: [2n],
    offending_tx_id: 3n, message: 'incomplete' }] };
  const api = method(historianIdl, 'get_commitment_route_summaries');
  assert.deepEqual(IDL.decode(api.retTypes, IDL.encode([replyType], [reply]))[0], reply);
});
