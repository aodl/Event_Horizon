# Resolved canonical ICP Ledger contract

Date resolved: 2026-09-26 UTC

This filename is retained as an audit trail. The architectural decision is no
longer open.

## Operator-supplied live evidence

The operator supplied the following read-only live-mainnet evidence from a
network-enabled build environment. Codex did not generate these results inside
its sandbox and did not perform any additional live command.

```text
date
    2026-09-26

canonical ICP Ledger
    ryjl3-tyaaa-aaaaa-aaaba-cai

icrc1_supported_standards
    ICRC-1
    ICRC-2
    ICRC-21

icrc3_supported_block_types query
    Canister has no query method 'icrc3_supported_block_types'
    IC0536
```

The canonical ICP Ledger therefore does not satisfy Event Horizon's generic
ICRC-3 observation contract. The missing required endpoint is decisive; no
further ICRC-3 representation proof for ICP is needed.

## Pinned official DFINITY evidence

The earlier investigation also inspected the locally cached official
`dfinity/ic` source at commit
`d3b3351fa343a893aea2cfa8d39ac6b0d507b477`, dated 2026-04-22, in
`rs/ledger_suite/icp/ledger.did`. That Candid declares:

```candid
query_blocks : (GetBlocksArgs) -> (QueryBlocksResponse) query;
query_encoded_blocks : (GetBlocksArgs) -> (QueryEncodedBlocksResponse) query;
icrc1_supported_standards : () ->
  (vec record { name : text; url : text }) query;
icrc2_approve : (ApproveArgs) -> (ApproveResult);
icrc2_allowance : (AllowanceArgs) -> (Allowance) query;
icrc2_transfer_from : (TransferFromArgs) -> (TransferFromResult);
```

It does not declare `icrc3_supported_block_types` or `icrc3_get_blocks`. A
second cached official revision,
`80b1d5e27bfdb96d6e50fb2ecee5ff80a77900e9`, has the same relevant service
surface. This pinned source/Candid evidence is consistent with the decisive
operator-supplied live result.

## Approved architecture

```text
canonical ICP
    legacy query_blocks adapter

non-ICP Observed Ledger
    ICRC-1 + ICRC-3 + 1xfer
    optional 2xfer

all instances
    one Event Horizon backend Wasm
```

Dispatch is immutable principal equality against the compiled canonical ICP
Ledger principal. It is not protocol fallback, probing, or a configurable
reader choice. Canonical ICP is the only supported legacy ledger.

For the ICP instance, one legacy page stream drives Jupiter admission and
Observed Ledger activity in authoritative block order. For a non-ICP instance,
the Protocol ICP Ledger remains the legacy admission and funding anchor while
the configured Observed Ledger supplies activity through ICRC-3.

## Why the previous mock was not proof

The provisional `mock-icp-ledger` exposed ICRC-3 methods and synthesized the
generic representation Event Horizon expected. Those tests only proved
compatibility with that fixture; they could not prove the deployed canonical
ICP contract. The final test architecture keeps a canonical ICP mock without
successful ICRC-3 endpoints and a distinct generic ICRC-3 mock.

## Safety statement

This resolution and corrective implementation involved no deployment,
installation, reinstall, upgrade, controller change, blackholing, transfer,
top-up, alias publication, or other mainnet mutation.
