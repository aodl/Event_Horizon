#![allow(dead_code)]
use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::call::Call;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct NotifyTopUpArg {
    canister_id: Principal,
    block_index: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
enum NotifyTopUpResult {
    Ok(Nat),
    Err(NotifyError),
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
enum NotifyError {
    Refunded {
        reason: String,
        block_index: Option<u64>,
    },
    Processing,
    TransactionTooOld(u64),
    InvalidTransaction(String),
    Other {
        error_code: u64,
        error_message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotifyOutcome {
    Success,
    Refunded,
    RetryLater,
    Terminal(String),
}

pub async fn notify_top_up(
    cmc: Principal,
    canister_id: Principal,
    block_index: u64,
) -> NotifyOutcome {
    let response = match Call::bounded_wait(cmc, "notify_top_up")
        .with_arg(&NotifyTopUpArg {
            canister_id,
            block_index,
        })
        .await
    {
        Ok(response) => response,
        Err(_) => return NotifyOutcome::RetryLater,
    };
    let result = match response.candid::<NotifyTopUpResult>() {
        Ok(result) => result,
        Err(_) => return NotifyOutcome::RetryLater,
    };
    match result {
        NotifyTopUpResult::Ok(_) => NotifyOutcome::Success,
        NotifyTopUpResult::Err(NotifyError::Refunded { .. }) => NotifyOutcome::Refunded,
        NotifyTopUpResult::Err(NotifyError::Processing)
        | NotifyTopUpResult::Err(NotifyError::Other { .. }) => NotifyOutcome::RetryLater,
        NotifyTopUpResult::Err(NotifyError::TransactionTooOld(block)) => {
            NotifyOutcome::Terminal(format!("transaction_too_old oldest={block}"))
        }
        NotifyTopUpResult::Err(NotifyError::InvalidTransaction(message)) => {
            NotifyOutcome::Terminal(format!("invalid_transaction {message}"))
        }
    }
}
