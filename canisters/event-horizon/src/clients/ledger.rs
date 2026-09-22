use candid::{CandidType, Deserialize, Int, Nat, Principal};
use ic_cdk::call::{Call, CallErrorExt};

use crate::config::RESERVE_PROTECTION_CYCLES;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct Tokens {
    pub e8s: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct TimeStamp {
    pub timestamp_nanos: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub memo: u64,
    pub icrc1_memo: Option<Vec<u8>>,
    pub operation: Option<Operation>,
    pub created_at_time: TimeStamp,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub enum Operation {
    Mint {
        to: Vec<u8>,
        amount: Tokens,
    },
    Burn {
        from: Vec<u8>,
        spender: Option<Vec<u8>>,
        amount: Tokens,
    },
    Transfer {
        from: Vec<u8>,
        to: Vec<u8>,
        amount: Tokens,
        fee: Tokens,
        spender: Option<Vec<u8>>,
    },
    Approve {
        from: Vec<u8>,
        spender: Vec<u8>,
        allowance_e8s: Int,
        allowance: Tokens,
        fee: Tokens,
        expires_at: Option<TimeStamp>,
        expected_allowance: Option<Tokens>,
    },
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub parent_hash: Option<Vec<u8>>,
    pub transaction: Transaction,
    pub timestamp: TimeStamp,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct GetBlocksArgs {
    pub start: u64,
    pub length: u64,
}

/// Event Horizon intentionally does not decode or call archive callbacks. Candid record
/// subtyping permits this narrow range representation containing only the location.
#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct ArchivedBlocksRange {
    pub start: u64,
    pub length: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct QueryBlocksResponse {
    pub chain_length: u64,
    pub blocks: Vec<Block>,
    pub first_block_index: u64,
    pub archived_blocks: Vec<ArchivedBlocksRange>,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

/// Legacy ICP Ledger transfer argument. CMC top-ups use the legacy transfer endpoint and
/// account identifier convention, not ICRC-1 Account/memo fields.
#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct LegacyTransferArg {
    pub memo: u64,
    pub amount: Tokens,
    pub fee: Tokens,
    pub from_subaccount: Option<[u8; 32]>,
    pub to: Vec<u8>,
    pub created_at_time: Option<TimeStamp>,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub enum LegacyTransferError {
    BadFee { expected_fee: Tokens },
    InsufficientFunds { balance: Tokens },
    TxTooOld { allowed_window_nanos: u64 },
    TxCreatedInFuture,
    TxDuplicate { duplicate_of: u64 },
}

pub type LegacyTransferResult = Result<u64, LegacyTransferError>;

fn nat_to_u64(n: &Nat) -> Result<u64, String> {
    u64::try_from(n.0.clone()).map_err(|_| format!("Nat does not fit u64: {n}"))
}

pub async fn query_blocks(
    ledger: Principal,
    start: u64,
    length: u64,
) -> Result<QueryBlocksResponse, String> {
    let args = GetBlocksArgs { start, length };
    let call = Call::bounded_wait(ledger, "query_blocks").with_arg(&args);
    if ic_cdk::api::canister_liquid_cycle_balance()
        < RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
    {
        return Err("reserve_protection".to_string());
    }
    call.await
        .map_err(|e| format!("query_blocks transport: {e:?}"))?
        .candid::<QueryBlocksResponse>()
        .map_err(|e| format!("query_blocks decode: {e:?}"))
}

pub async fn icrc1_balance_of(ledger: Principal, account: Account) -> Result<u64, String> {
    let value = Call::bounded_wait(ledger, "icrc1_balance_of")
        .with_arg(&account)
        .await
        .map_err(|e| format!("balance transport: {e:?}"))?
        .candid::<Nat>()
        .map_err(|e| format!("balance decode: {e:?}"))?;
    nat_to_u64(&value)
}

pub async fn icrc1_fee(ledger: Principal) -> Result<u64, String> {
    let value = Call::bounded_wait(ledger, "icrc1_fee")
        .await
        .map_err(|e| format!("fee transport: {e:?}"))?
        .candid::<Nat>()
        .map_err(|e| format!("fee decode: {e:?}"))?;
    nat_to_u64(&value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyTransferOutcome {
    Accepted(u64),
    RetrySameIdentity,
    Replan,
    IdentityExpired,
    Uncertain(String),
}

/// Performs the legacy ICP transfer used for CMC top-ups.
///
/// Transport/decode ambiguity retains the exact transfer identity. Definite no-debit errors
/// that can only be repaired by changing the fee/time/amount request a fresh plan. Duplicate
/// semantics recover the original accepted block index.
pub async fn legacy_transfer(ledger: Principal, arg: &LegacyTransferArg) -> LegacyTransferOutcome {
    let response = match Call::unbounded_wait(ledger, "transfer").with_arg(arg).await {
        Ok(response) => response,
        Err(e) if e.is_clean_reject() => return LegacyTransferOutcome::Replan,
        Err(e) => return LegacyTransferOutcome::Uncertain(format!("transfer transport: {e:?}")),
    };
    let result = match response.candid::<LegacyTransferResult>() {
        Ok(result) => result,
        Err(e) => return LegacyTransferOutcome::Uncertain(format!("transfer decode: {e:?}")),
    };
    match result {
        Ok(block) => LegacyTransferOutcome::Accepted(block),
        Err(LegacyTransferError::TxDuplicate { duplicate_of }) => {
            LegacyTransferOutcome::Accepted(duplicate_of)
        }
        Err(LegacyTransferError::TxCreatedInFuture) => LegacyTransferOutcome::RetrySameIdentity,
        Err(LegacyTransferError::TxTooOld { .. }) => LegacyTransferOutcome::IdentityExpired,
        Err(LegacyTransferError::BadFee { .. })
        | Err(LegacyTransferError::InsufficientFunds { .. }) => LegacyTransferOutcome::Replan,
    }
}
