// Wire contracts: dfinity/ic rs/ledger_suite/icp/{ledger,ledger_archive}.did,
// Event Horizon's production DID, and candid/jupiter-historian.subset.did.
// Keep all nat64 fields as BigInt; Candid record subtyping drops unused fields.
function ledgerTypes(IDL) {
  const Blob = IDL.Vec(IDL.Nat8);
  const Tokens = IDL.Record({ e8s: IDL.Nat64 });
  const Timestamp = IDL.Record({ timestamp_nanos: IDL.Nat64 });
  const Operation = IDL.Variant({
    Mint: IDL.Record({ to: Blob, amount: Tokens }),
    Burn: IDL.Record({ from: Blob, spender: IDL.Opt(Blob), amount: Tokens }),
    Transfer: IDL.Record({
      from: Blob, to: Blob, amount: Tokens, fee: Tokens, spender: IDL.Opt(Blob),
    }),
    Approve: IDL.Record({
      from: Blob, spender: Blob, allowance_e8s: IDL.Int, allowance: Tokens,
      fee: Tokens, expires_at: IDL.Opt(Timestamp), expected_allowance: IDL.Opt(Tokens),
    }),
  });
  const Transaction = IDL.Record({
    memo: IDL.Nat64, icrc1_memo: IDL.Opt(Blob), operation: IDL.Opt(Operation),
    created_at_time: Timestamp,
  });
  const Block = IDL.Record({
    parent_hash: IDL.Opt(Blob), transaction: Transaction, timestamp: Timestamp,
  });
  const Range = IDL.Record({ start: IDL.Nat64, length: IDL.Nat64 });
  const ArchiveResult = IDL.Variant({
    Ok: IDL.Record({ blocks: IDL.Vec(Block) }),
    Err: IDL.Variant({
      BadFirstBlockIndex: IDL.Record({
        requested_index: IDL.Nat64, first_valid_index: IDL.Nat64,
      }),
      Other: IDL.Record({ error_code: IDL.Nat64, error_message: IDL.Text }),
    }),
  });
  const ArchiveCallback = IDL.Func([Range], [ArchiveResult], ['query']);
  return { Block, Range, ArchiveResult, ArchiveCallback };
}

export function ledgerIdl({ IDL }) {
  const { Block, Range, ArchiveCallback } = ledgerTypes(IDL);
  const Reply = IDL.Record({
    chain_length: IDL.Nat64,
    blocks: IDL.Vec(Block),
    first_block_index: IDL.Nat64,
    archived_blocks: IDL.Vec(IDL.Record({
      start: IDL.Nat64, length: IDL.Nat64, callback: ArchiveCallback,
    })),
  });
  return IDL.Service({ query_blocks: IDL.Func([Range], [Reply], ['query']) });
}

export function archiveIdl(method) {
  return ({ IDL }) => {
    const { Range, ArchiveResult } = ledgerTypes(IDL);
    return IDL.Service({ [method]: IDL.Func([Range], [ArchiveResult], ['query']) });
  };
}

export function backendIdl({ IDL }) {
  const Price = IDL.Record({
    account_icp: IDL.Nat64, range_icp: IDL.Nat64, global_icp: IDL.Nat64,
  });
  const Pricing = IDL.Record({
    initialized: IDL.Bool,
    current: Price,
    current_effective_at: IDL.Nat64,
    next: IDL.Opt(Price),
    next_effective_at: IDL.Nat64,
    next_freeze_at: IDL.Nat64,
    observed_floor_xdr_permyriad: IDL.Nat64,
    floor_observed_at: IDL.Nat64,
    latest_xdr_permyriad: IDL.Nat64,
    latest_observed_at: IDL.Nat64,
    next_carried_forward_due_to_stale_rate: IDL.Bool,
  });
  const Profile = IDL.Record({
    symbol: IDL.Text, decimals: IDL.Nat8, supports_icrc2_transfer_from: IDL.Bool,
    neuron_governance: IDL.Opt(IDL.Principal),
  });
  const Instance = IDL.Record({
    observed_ledger: IDL.Principal, observed_profile: IDL.Opt(Profile),
    sns_root: IDL.Opt(IDL.Principal), icp_ledger: IDL.Principal, cmc: IDL.Principal,
    jupiter_faucet: IDL.Principal, jupiter_historian: IDL.Principal,
    surplus_canister: IDL.Opt(IDL.Principal),
    icp_admission_notifier: IDL.Opt(IDL.Principal),
  });
  const Preview = IDL.Record({
    subscriber: IDL.Principal, subject: IDL.Opt(IDL.Principal),
    scope: IDL.Variant({ global: IDL.Null, subaccount: IDL.Null, neuron_nonce: IDL.Null }),
    start: IDL.Opt(IDL.Nat64), end: IDL.Opt(IDL.Nat64), target_count: IDL.Nat32,
    minimum_units: IDL.Opt(IDL.Nat), alias_revision: IDL.Opt(IDL.Vec(IDL.Nat8)),
    aliases_refreshed_at: IDL.Opt(IDL.Nat64),
  });
  return IDL.Service({
    get_instance: IDL.Func([], [Instance], ['query']),
    get_pricing: IDL.Func([], [Pricing], ['query']),
    preview_subscription: IDL.Func(
      [IDL.Text], [IDL.Variant({ Ok: Preview, Err: IDL.Text })], ['query'],
    ),
    poke: IDL.Func([], [], []),
  });
}

export function historianIdl({ IDL }) {
  const Blob = IDL.Vec(IDL.Nat8);
  const Route = IDL.Variant({
    CyclesTopUp: IDL.Record({ canister_id: IDL.Principal }),
    RawIcp: IDL.Record({ destination_canister_id: IDL.Principal, memo: Blob }),
    NeuronStake: IDL.Record({ neuron_id: IDL.Nat64, memo: IDL.Opt(Blob) }),
  });
  const Fault = IDL.Record({
    observed_at_ts: IDL.Nat64, last_cursor_tx_id: IDL.Opt(IDL.Nat64),
    offending_tx_id: IDL.Nat64, message: IDL.Text,
  });
  const Response = IDL.Record({
    items: IDL.Vec(IDL.Record({
      route: Route, qualifying_commitment_count: IDL.Nat64,
      total_qualifying_committed_e8s: IDL.Nat64,
    })),
    truncated: IDL.Bool, complete_from_genesis: IDL.Bool,
    commitment_index_fault: IDL.Opt(Fault),
  });
  return IDL.Service({
    get_commitment_route_summaries: IDL.Func(
      [IDL.Record({ routes: IDL.Vec(Route) })], [Response], ['query'],
    ),
  });
}

export const idlFactories = Object.freeze({ ledgerIdl, archiveIdl, backendIdl, historianIdl });
