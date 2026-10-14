// Independent full wire definitions from ICP ledger.did and ICRC-3 README.
// These are intentionally separate from the production count projections.
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
const blob = IDL.Vec(IDL.Nat8), tokens = IDL.Record({ e8s: IDL.Nat64 });
const timestamp = IDL.Record({ timestamp_nanos: IDL.Nat64 });
const operation = IDL.Variant({
  Mint: IDL.Record({ to: blob, amount: tokens }),
  Burn: IDL.Record({ from: blob, spender: IDL.Opt(blob), amount: tokens }),
  Transfer: IDL.Record({ from: blob, to: blob, amount: tokens, fee: tokens, spender: IDL.Opt(blob) }),
  Approve: IDL.Record({ from: blob, spender: blob, allowance_e8s: IDL.Int,
    allowance: tokens, fee: tokens, expires_at: IDL.Opt(timestamp), expected_allowance: IDL.Opt(tokens) }),
});
const transaction = IDL.Record({ memo: IDL.Nat64, icrc1_memo: IDL.Opt(blob), operation: IDL.Opt(operation), created_at_time: timestamp });
const block = IDL.Record({ parent_hash: IDL.Opt(blob), transaction, timestamp });
const icpRange = IDL.Record({ start: IDL.Nat64, length: IDL.Nat64 });
const archiveError = IDL.Variant({ BadFirstBlockIndex: IDL.Record({ requested_index: IDL.Nat64, first_valid_index: IDL.Nat64 }), Other: IDL.Record({ error_code: IDL.Nat64, error_message: IDL.Text }) });
const archiveReply = IDL.Variant({ Ok: IDL.Record({ blocks: IDL.Vec(block) }), Err: archiveError });
export const icpWire = IDL.Record({ chain_length: IDL.Nat64, certificate: IDL.Opt(blob), blocks: IDL.Vec(block),
  first_block_index: IDL.Nat64, archived_blocks: IDL.Vec(IDL.Record({ start: IDL.Nat64, length: IDL.Nat64,
    callback: IDL.Func([icpRange], [archiveReply], ['query']) })), extra_future_field: IDL.Text });
const value = IDL.Rec();
value.fill(IDL.Variant({ Blob: blob, Text: IDL.Text, Nat: IDL.Nat, Int: IDL.Int,
  Array: IDL.Vec(value), Map: IDL.Vec(IDL.Tuple(IDL.Text, value)) }));
const genericRange = IDL.Record({ start: IDL.Nat, length: IDL.Nat });
export const genericWire = IDL.Rec();
genericWire.fill(IDL.Record({ log_length: IDL.Nat, blocks: IDL.Vec(IDL.Record({ id: IDL.Nat, block: value })),
  archived_blocks: IDL.Vec(IDL.Record({ args: IDL.Vec(genericRange),
    callback: IDL.Func([IDL.Vec(genericRange)], [genericWire], ['query']) })), extra_future_field: IDL.Opt(value) }));
export const archivePrincipal = Principal.fromText('aaaaa-aa');
export function icpReply(total, populated = false) {
  return { chain_length: total, certificate: [new Uint8Array([1, 2])], first_block_index: 999n,
    blocks: populated ? [{ parent_hash: [], timestamp: { timestamp_nanos: 1n },
      transaction: { memo: 0n, icrc1_memo: [], operation: [{ Mint: { to: new Uint8Array(32), amount: { e8s: 1n } } }], created_at_time: { timestamp_nanos: 1n } } }] : [],
    archived_blocks: [{ start: 0n, length: 100n, callback: [archivePrincipal, 'query_blocks'] }], extra_future_field: 'ignored' };
}
export function genericReply(total, populated = false) {
  const recursive = { Map: [['nested', { Array: [{ Nat: 1n << 100n }, { Int: -1n }, { Blob: new Uint8Array([3]) }, { Text: 'text' }] }]] };
  return { log_length: total, blocks: populated ? [{ id: 0n, block: recursive }] : [],
    archived_blocks: [{ args: [{ start: 0n, length: 100n }], callback: [archivePrincipal, 'icrc3_get_blocks'] }], extra_future_field: [recursive] };
}
