#![allow(dead_code)]
use std::cell::RefCell;

use candid::{CandidType, Deserialize, Int, Nat, Principal};
use ic_cdk::call::Call;
use sha2::{Digest, Sha224};

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
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct GetBlocksArgs {
    pub start: u64,
    pub length: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct ArchivedRange {
    pub start: u64,
    pub length: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct QueryBlocksResponse {
    pub chain_length: u64,
    pub blocks: Vec<Block>,
    pub first_block_index: u64,
    pub archived_blocks: Vec<ArchivedRange>,
}
#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

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
type LegacyTransferResult = Result<u64, LegacyTransferError>;

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct DebugAppendTransfer {
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub amount_e8s: u64,
    pub icrc1_memo: Option<Vec<u8>>,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct DebugSetBalance {
    pub account: Account,
    pub e8s: u64,
}
#[derive(Clone, Copy, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub enum DebugLegacyBehavior {
    Normal,
    DropResponseAfterAccept,
    BadFee,
    InsufficientFunds,
    TooOld,
    CreatedInFuture,
}

// Big-endian ASCII `SURPLUS1`, matching Event Horizon's protocol memo.
const SURPLUS_TRANSFER_MEMO: u64 = u64::from_be_bytes(*b"SURPLUS1");

#[derive(Clone)]
struct Balance {
    account: Account,
    e8s: u64,
}
#[derive(Clone)]
struct LegacyDedup {
    caller: Principal,
    arg: LegacyTransferArg,
    block: u64,
}
struct State {
    blocks: Vec<Block>,
    first_local: u64,
    suppress_archive_info: bool,
    fee_e8s: u64,
    balances: Vec<Balance>,
    legacy_dedup: Vec<LegacyDedup>,
    legacy_behavior: DebugLegacyBehavior,
    surplus_legacy_behavior: DebugLegacyBehavior,
    accepted_legacy_transfers: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            blocks: vec![],
            first_local: 0,
            suppress_archive_info: false,
            fee_e8s: 10_000,
            balances: vec![],
            legacy_dedup: vec![],
            legacy_behavior: DebugLegacyBehavior::Normal,
            surplus_legacy_behavior: DebugLegacyBehavior::Normal,
            accepted_legacy_transfers: 0,
        }
    }
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn account_identifier(owner: Principal, subaccount: [u8; 32]) -> [u8; 32] {
    let mut h = Sha224::new();
    h.update(b"\x0Aaccount-id");
    h.update(owner.as_slice());
    h.update(subaccount);
    let hash = h.finalize();
    let mut out = [0u8; 32];
    out[..4].copy_from_slice(&crc32fast::hash(&hash).to_be_bytes());
    out[4..].copy_from_slice(&hash);
    out
}
fn balance_of(state: &State, account: &Account) -> u64 {
    state
        .balances
        .iter()
        .find(|b| b.account == *account)
        .map(|b| b.e8s)
        .unwrap_or(0)
}
fn set_balance(state: &mut State, account: Account, value: u64) {
    if let Some(b) = state.balances.iter_mut().find(|b| b.account == account) {
        b.e8s = value;
    } else {
        state.balances.push(Balance {
            account,
            e8s: value,
        });
    }
}
fn append_transfer(
    state: &mut State,
    from: Vec<u8>,
    to: Vec<u8>,
    amount: u64,
    fee: u64,
    memo: Option<Vec<u8>>,
    legacy_memo: u64,
) -> u64 {
    let idx = state.blocks.len() as u64;
    let now = ic_cdk::api::time();
    state.blocks.push(Block {
        parent_hash: None,
        transaction: Transaction {
            memo: legacy_memo,
            icrc1_memo: memo,
            operation: Some(Operation::Transfer {
                from,
                to,
                amount: Tokens { e8s: amount },
                fee: Tokens { e8s: fee },
                spender: None,
            }),
            created_at_time: TimeStamp {
                timestamp_nanos: now,
            },
        },
        timestamp: TimeStamp {
            timestamp_nanos: now,
        },
    });
    idx
}

#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

#[ic_cdk::query]
fn query_blocks(args: GetBlocksArgs) -> QueryBlocksResponse {
    STATE.with(|s| {
        let s = s.borrow();
        let chain = s.blocks.len() as u64;
        if args.length == 0 {
            return QueryBlocksResponse {
                chain_length: chain,
                blocks: vec![],
                first_block_index: args.start,
                archived_blocks: vec![],
            };
        }
        let request_end = args.start.saturating_add(args.length).min(chain);
        let archive_end = s.first_local.min(request_end);
        let mut archived = vec![];
        if args.start < archive_end && !s.suppress_archive_info {
            archived.push(ArchivedRange {
                start: args.start,
                length: archive_end - args.start,
            });
        }
        let first = args.start.max(s.first_local).min(request_end);
        let blocks = (first..request_end)
            .map(|i| s.blocks[i as usize].clone())
            .collect();
        QueryBlocksResponse {
            chain_length: chain,
            blocks,
            first_block_index: first,
            archived_blocks: archived,
        }
    })
}

#[ic_cdk::query]
fn icrc1_balance_of(account: Account) -> Nat {
    STATE.with(|s| Nat::from(balance_of(&s.borrow(), &account)))
}
#[ic_cdk::query]
fn icrc1_fee() -> Nat {
    STATE.with(|s| Nat::from(s.borrow().fee_e8s))
}

#[ic_cdk::update]
async fn transfer(arg: LegacyTransferArg) -> LegacyTransferResult {
    let caller = ic_cdk::api::msg_caller();
    let accepted: Result<(u64, bool), LegacyTransferError> = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(found) = s
            .legacy_dedup
            .iter()
            .find(|d| d.caller == caller && d.arg == arg)
        {
            return Err(LegacyTransferError::TxDuplicate {
                duplicate_of: found.block,
            });
        }
        let behavior = if arg.memo == SURPLUS_TRANSFER_MEMO {
            s.surplus_legacy_behavior
        } else {
            s.legacy_behavior
        };
        match behavior {
            DebugLegacyBehavior::BadFee => {
                return Err(LegacyTransferError::BadFee {
                    expected_fee: Tokens { e8s: s.fee_e8s },
                })
            }
            DebugLegacyBehavior::InsufficientFunds => {
                return Err(LegacyTransferError::InsufficientFunds {
                    balance: Tokens { e8s: 0 },
                })
            }
            DebugLegacyBehavior::TooOld => {
                return Err(LegacyTransferError::TxTooOld {
                    allowed_window_nanos: 60_000_000_000,
                })
            }
            DebugLegacyBehavior::CreatedInFuture => {
                return Err(LegacyTransferError::TxCreatedInFuture)
            }
            DebugLegacyBehavior::Normal | DebugLegacyBehavior::DropResponseAfterAccept => {}
        }
        if arg.fee.e8s != s.fee_e8s {
            return Err(LegacyTransferError::BadFee {
                expected_fee: Tokens { e8s: s.fee_e8s },
            });
        }
        if arg.to.len() != 32 {
            return Err(LegacyTransferError::InsufficientFunds {
                balance: Tokens { e8s: 0 },
            });
        }
        let source = Account {
            owner: caller,
            subaccount: arg.from_subaccount.map(|x| x.to_vec()),
        };
        let balance = balance_of(&s, &source);
        let need = arg.amount.e8s.saturating_add(arg.fee.e8s);
        if balance < need {
            return Err(LegacyTransferError::InsufficientFunds {
                balance: Tokens { e8s: balance },
            });
        }
        set_balance(&mut s, source, balance - need);
        let from_id = account_identifier(caller, arg.from_subaccount.unwrap_or([0; 32])).to_vec();
        let block = append_transfer(
            &mut s,
            from_id,
            arg.to.clone(),
            arg.amount.e8s,
            arg.fee.e8s,
            None,
            arg.memo,
        );
        s.legacy_dedup.push(LegacyDedup {
            caller,
            arg: arg.clone(),
            block,
        });
        s.accepted_legacy_transfers += 1;
        Ok((
            block,
            behavior == DebugLegacyBehavior::DropResponseAfterAccept,
        ))
    });
    let (block, lose_response) = accepted?;
    if lose_response {
        // Crossing an await commits the accepted transfer before the subsequent trap,
        // faithfully modelling an accepted ledger transfer whose response is lost.
        Call::unbounded_wait(ic_cdk::api::canister_self(), "debug_noop")
            .await
            .unwrap_or_else(|e| ic_cdk::trap(format!("debug barrier failed: {e:?}")));
        ic_cdk::trap("forced lost legacy transfer response");
    }
    Ok(block)
}

#[ic_cdk::update]
fn debug_noop() {}

#[ic_cdk::update]
fn debug_append_transfer(arg: DebugAppendTransfer) -> u64 {
    STATE.with(|s| {
        append_transfer(
            &mut s.borrow_mut(),
            arg.from,
            arg.to,
            arg.amount_e8s,
            10_000,
            arg.icrc1_memo,
            0,
        )
    })
}
#[ic_cdk::update]
fn debug_set_first_local_block(index: u64) {
    STATE.with(|s| {
        let len = s.borrow().blocks.len() as u64;
        s.borrow_mut().first_local = index.min(len);
    });
}
#[ic_cdk::update]
fn debug_suppress_archive_info(value: bool) {
    STATE.with(|s| s.borrow_mut().suppress_archive_info = value);
}
#[ic_cdk::update]
fn debug_set_balance(arg: DebugSetBalance) {
    STATE.with(|s| set_balance(&mut s.borrow_mut(), arg.account, arg.e8s));
}
#[ic_cdk::update]
fn debug_set_legacy_behavior(value: DebugLegacyBehavior) {
    STATE.with(|s| s.borrow_mut().legacy_behavior = value);
}
#[ic_cdk::update]
fn debug_set_surplus_legacy_behavior(value: DebugLegacyBehavior) {
    STATE.with(|s| s.borrow_mut().surplus_legacy_behavior = value);
}
#[ic_cdk::query]
fn debug_chain_length() -> u64 {
    STATE.with(|s| s.borrow().blocks.len() as u64)
}
#[ic_cdk::query]
fn debug_accepted_legacy_transfers() -> u64 {
    STATE.with(|s| s.borrow().accepted_legacy_transfers)
}

#[ic_cdk::query]
fn debug_accepted_legacy_transfers_with_memo(memo: u64) -> u64 {
    STATE.with(|s| {
        s.borrow()
            .legacy_dedup
            .iter()
            .filter(|entry| entry.arg.memo == memo)
            .count() as u64
    })
}

#[ic_cdk::query]
fn debug_accepted_legacy_destinations_with_memo(memo: u64) -> Vec<Vec<u8>> {
    STATE.with(|s| {
        s.borrow()
            .legacy_dedup
            .iter()
            .filter(|entry| entry.arg.memo == memo)
            .map(|entry| entry.arg.to.clone())
            .collect()
    })
}

ic_cdk::export_candid!();
