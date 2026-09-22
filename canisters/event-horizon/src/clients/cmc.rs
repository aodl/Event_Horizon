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

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub struct IcpXdrConversionRate {
    pub timestamp_seconds: u64,
    pub xdr_permyriad_per_icp: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct IcpXdrConversionRateResponse {
    data: IcpXdrConversionRate,
    hash_tree: Vec<u8>,
    certificate: Vec<u8>,
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

pub async fn get_icp_xdr_conversion_rate(
    cmc: Principal,
) -> Result<(IcpXdrConversionRate, u128), String> {
    let call = Call::bounded_wait(cmc, "get_icp_xdr_conversion_rate");
    let cost = call.get_cost();
    let response = call
        .await
        .map_err(|e| format!("pricing transport: {e:?}"))?
        .candid::<IcpXdrConversionRateResponse>()
        .map_err(|e| format!("pricing decode: {e:?}"))?;
    if response.data.xdr_permyriad_per_icp == 0 {
        return Err("pricing rate is zero".to_string());
    }
    Ok((response.data, cost))
}
