use crate::config::RESERVE_PROTECTION_CYCLES;
#[cfg(feature = "debug_api")]
thread_local! { static QUERY_BLOCKS_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
use candid::{CandidType, Deserialize, Int, Nat, Principal};
use ic_cdk::call::{Call, CallErrorExt};

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
#[derive(CandidType, Deserialize)]
struct BlockRange {
    blocks: Vec<Block>,
}

#[derive(Debug, CandidType, Deserialize)]
enum ArchiveError {
    BadFirstBlockIndex {
        requested_index: u64,
        first_valid_index: u64,
    },
    Other {
        error_code: u64,
        error_message: String,
    },
}

type ArchiveResult = Result<BlockRange, ArchiveError>;
candid::define_function!(pub ArchiveCallback : (GetBlocksArgs) -> (ArchiveResult) query);

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct ArchivedBlocksRange {
    pub start: u64,
    pub length: u64,
    pub callback: ArchiveCallback,
}
#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct QueryBlocksResponse {
    pub chain_length: u64,
    pub blocks: Vec<Block>,
    pub first_block_index: u64,
    pub archived_blocks: Vec<ArchivedBlocksRange>,
}

pub async fn query_blocks(
    ledger: Principal,
    start: u64,
    length: u64,
) -> Result<QueryBlocksResponse, String> {
    #[cfg(feature = "debug_api")]
    QUERY_BLOCKS_CALLS.with(|calls| calls.set(calls.get() + 1));
    read_blocks(ledger, "query_blocks", start, length)
        .await?
        .candid()
        .map_err(|e| format!("query_blocks decode: {e:?}"))
}

async fn read_blocks(
    ledger: Principal,
    method: &str,
    start: u64,
    length: u64,
) -> Result<ic_cdk::call::Response, String> {
    let call = Call::bounded_wait(ledger, method).with_arg(&GetBlocksArgs { start, length });
    if ic_cdk::api::canister_liquid_cycle_balance()
        < RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
    {
        return Err(crate::config::RESERVE_PROTECTION_ERROR.into());
    }
    call.await.map_err(|e| format!("{method} transport: {e:?}"))
}

pub async fn archive_blocks(
    callback: &ArchiveCallback,
    start: u64,
    length: u64,
) -> Result<Vec<Block>, String> {
    read_blocks(callback.0.principal, &callback.0.method, start, length)
        .await?
        .candid::<ArchiveResult>()
        .map_err(|e| format!("archive decode: {e:?}"))?
        .map(|range| range.blocks)
        .map_err(|e| format!("archive error: {e:?}"))
}

#[cfg(feature = "debug_api")]
pub fn debug_query_blocks_calls() -> u64 {
    QUERY_BLOCKS_CALLS.with(std::cell::Cell::get)
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

#[cfg(test)]
#[allow(dead_code)]
mod contract_tests {
    use super::*;

    // Independently mirrors ledger.did and ledger_archive.did at dfinity/ic
    // da6233202a0ae1bc778c37a14dc28a2aea2c6590.
    #[derive(CandidType)]
    struct WireTokens {
        e8s: u64,
    }

    #[derive(CandidType)]
    struct WireTimeStamp {
        timestamp_nanos: u64,
    }

    #[derive(CandidType)]
    enum WireOperation {
        Mint {
            to: Vec<u8>,
            amount: WireTokens,
        },
        Burn {
            from: Vec<u8>,
            spender: Option<Vec<u8>>,
            amount: WireTokens,
        },
        Transfer {
            from: Vec<u8>,
            to: Vec<u8>,
            amount: WireTokens,
            fee: WireTokens,
            spender: Option<Vec<u8>>,
        },
        Approve {
            from: Vec<u8>,
            spender: Vec<u8>,
            allowance_e8s: Int,
            allowance: WireTokens,
            fee: WireTokens,
            expires_at: Option<WireTimeStamp>,
            expected_allowance: Option<WireTokens>,
        },
    }

    #[derive(CandidType)]
    struct WireTransaction {
        memo: u64,
        icrc1_memo: Option<Vec<u8>>,
        operation: Option<WireOperation>,
        created_at_time: WireTimeStamp,
    }

    #[derive(CandidType)]
    struct WireBlock {
        parent_hash: Option<Vec<u8>>,
        transaction: WireTransaction,
        timestamp: WireTimeStamp,
    }

    #[derive(CandidType)]
    struct WireBlockRange {
        blocks: Vec<WireBlock>,
    }

    #[derive(CandidType)]
    enum WireQueryArchiveError {
        BadFirstBlockIndex {
            requested_index: u64,
            first_valid_index: u64,
        },
        Other {
            error_code: u64,
            error_message: String,
        },
    }

    #[derive(CandidType)]
    enum WireQueryArchiveResult {
        Ok(WireBlockRange),
        Err(WireQueryArchiveError),
    }

    #[derive(CandidType, Deserialize)]
    struct WireGetBlocksArgs {
        start: u64,
        length: u64,
    }

    candid::define_function!(WireQueryArchiveFn : (WireGetBlocksArgs) -> (WireQueryArchiveResult) query);

    #[derive(CandidType)]
    struct WireArchivedBlocksRange {
        start: u64,
        length: u64,
        callback: WireQueryArchiveFn,
    }

    #[derive(CandidType)]
    struct WireQueryBlocksResponse {
        chain_length: u64,
        certificate: Option<Vec<u8>>,
        blocks: Vec<WireBlock>,
        first_block_index: u64,
        archived_blocks: Vec<WireArchivedBlocksRange>,
    }

    #[test]
    fn official_empty_legacy_reply_decodes_into_consumer_projection() {
        let wire = WireQueryBlocksResponse {
            chain_length: 38_803_000,
            certificate: Some(vec![1, 2, 3]),
            blocks: vec![],
            // Officially unspecified because blocks is empty.
            first_block_index: u64::MAX,
            archived_blocks: vec![WireArchivedBlocksRange {
                start: 38_801_807,
                length: 256,
                callback: WireQueryArchiveFn::new(Principal::anonymous(), "get_blocks".to_string()),
            }],
        };
        let encoded = candid::encode_one(wire).unwrap();
        let decoded: QueryBlocksResponse = candid::decode_one(&encoded).unwrap();
        assert_eq!(decoded.chain_length, 38_803_000);
        assert_eq!(decoded.first_block_index, u64::MAX);
        assert!(decoded.blocks.is_empty());
        assert_eq!(
            decoded.archived_blocks,
            vec![ArchivedBlocksRange {
                start: 38_801_807,
                length: 256,
                callback: ArchiveCallback::new(Principal::anonymous(), "get_blocks".to_string()),
            }]
        );
    }
    #[test]
    fn official_archive_requests_and_results_decode_in_both_directions() {
        let bytes = candid::encode_one(GetBlocksArgs {
            start: u64::MAX - 1,
            length: 1,
        })
        .unwrap();
        let args: WireGetBlocksArgs = candid::decode_one(&bytes).unwrap();
        assert_eq!((args.start, args.length), (u64::MAX - 1, 1));

        let success = WireQueryArchiveResult::Ok(WireBlockRange {
            blocks: vec![WireBlock {
                parent_hash: None,
                timestamp: WireTimeStamp { timestamp_nanos: 9 },
                transaction: WireTransaction {
                    memo: 0,
                    icrc1_memo: Some(vec![1, 2]),
                    operation: None,
                    created_at_time: WireTimeStamp { timestamp_nanos: 8 },
                },
            }],
        });
        let encode = |value| candid::encode_one(value).unwrap();
        let blocks = candid::decode_one::<ArchiveResult>(&encode(success))
            .unwrap()
            .unwrap()
            .blocks;
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].transaction.icrc1_memo, Some(vec![1, 2]));

        let bad_first = WireQueryArchiveResult::Err(WireQueryArchiveError::BadFirstBlockIndex {
            requested_index: 20,
            first_valid_index: 25,
        });
        assert!(matches!(
            candid::decode_one::<ArchiveResult>(&encode(bad_first)).unwrap(),
            Err(ArchiveError::BadFirstBlockIndex {
                requested_index: 20,
                first_valid_index: 25
            })
        ));
        let other = WireQueryArchiveResult::Err(WireQueryArchiveError::Other {
            error_code: 42,
            error_message: "unavailable".to_string(),
        });
        assert!(matches!(
            candid::decode_one::<ArchiveResult>(&encode(other)).unwrap(),
            Err(ArchiveError::Other { error_code: 42, error_message }) if error_message == "unavailable"
        ));
        assert!(candid::decode_one::<ArchiveResult>(&candid::encode_one(42u64).unwrap()).is_err());
    }
}
