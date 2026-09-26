// The protocol fixtures share block-construction/debug helpers, while exported
// ledger methods are selected at compile time. The canonical ICP package is
// built without `generic_icrc3`, so it cannot successfully expose ICRC-3.
include!("../../mock-icp-ledger/src/lib.rs");
