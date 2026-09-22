use crate::{
    account::{account_identifier_bytes, principal_to_subaccount},
    clients::{cmc, ledger},
    config, logging,
    state::{self, CmcState},
};

fn transfer_arg(
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
) -> ledger::LegacyTransferArg {
    let runtime = config::runtime();
    let self_id = ic_cdk::api::canister_self();
    let cmc_subaccount = principal_to_subaccount(self_id);
    let cmc_account_identifier = account_identifier_bytes(runtime.cmc_canister, cmc_subaccount);
    ledger::LegacyTransferArg {
        memo: config::TOP_UP_CANISTER_MEMO,
        amount: ledger::Tokens { e8s: amount_e8s },
        fee: ledger::Tokens { e8s: fee_e8s },
        from_subaccount: None,
        to: cmc_account_identifier.to_vec(),
        created_at_time: Some(ledger::TimeStamp {
            timestamp_nanos: created_at_time_nanos,
        }),
    }
}

async fn resume_transfer(amount_e8s: u64, fee_e8s: u64, created_at_time_nanos: u64) -> bool {
    let runtime = config::runtime();
    let arg = transfer_arg(amount_e8s, fee_e8s, created_at_time_nanos);
    match ledger::legacy_transfer(runtime.ledger_canister, &arg).await {
        ledger::LegacyTransferOutcome::Accepted(block_index) => {
            state::write_cmc_state(CmcState::NotifyPending { block_index });
            true
        }
        ledger::LegacyTransferOutcome::RetrySameIdentity
        | ledger::LegacyTransferOutcome::Uncertain(_) => false,
        ledger::LegacyTransferOutcome::IdentityExpired => {
            logging::cmc_transfer_identity_expired();
            state::write_cmc_state(CmcState::Idle);
            false
        }
        ledger::LegacyTransferOutcome::Replan => {
            // The Ledger proved no debit. Drop this plan so the next maintenance tick can
            // re-read the live balance/fee and construct a fresh deterministic identity.
            state::write_cmc_state(CmcState::Idle);
            false
        }
    }
}

async fn resume_notify(block_index: u64) -> bool {
    let runtime = config::runtime();
    let self_id = ic_cdk::api::canister_self();
    match cmc::notify_top_up(runtime.cmc_canister, self_id, block_index).await {
        cmc::NotifyOutcome::Success => {
            state::write_cmc_state(CmcState::Idle);
            true
        }
        cmc::NotifyOutcome::Refunded => {
            state::write_cmc_state(CmcState::Idle);
            false
        }
        cmc::NotifyOutcome::RetryLater => false,
        cmc::NotifyOutcome::Terminal(reason) => {
            logging::cmc_terminal(&format!("block={block_index} {reason}"));
            state::write_cmc_state(CmcState::Idle);
            false
        }
    }
}

pub async fn run_funding_maintenance() -> bool {
    match state::read_cmc_state() {
        CmcState::NotifyPending { block_index } => {
            return resume_notify(block_index).await;
        }
        CmcState::TransferPending {
            amount_e8s,
            fee_e8s,
            created_at_time_nanos,
        } => {
            if resume_transfer(amount_e8s, fee_e8s, created_at_time_nanos).await {
                if let CmcState::NotifyPending { block_index } = state::read_cmc_state() {
                    return resume_notify(block_index).await;
                }
            }
            return false;
        }
        CmcState::Idle => {}
    }

    let runtime = config::runtime();
    let self_id = ic_cdk::api::canister_self();
    let balance = match ledger::icrc1_balance_of(
        runtime.ledger_canister,
        ledger::Account {
            owner: self_id,
            subaccount: None,
        },
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return false,
    };
    let fee = match ledger::icrc1_fee(runtime.ledger_canister).await {
        Ok(value) => value,
        Err(_) => return false,
    };
    if balance <= fee {
        return false;
    }

    let amount_e8s = balance - fee;
    let created_at_time_nanos = ic_cdk::api::time();
    state::write_cmc_state(CmcState::TransferPending {
        amount_e8s,
        fee_e8s: fee,
        created_at_time_nanos,
    });
    if resume_transfer(amount_e8s, fee, created_at_time_nanos).await {
        if let CmcState::NotifyPending { block_index } = state::read_cmc_state() {
            return resume_notify(block_index).await;
        }
    }
    false
}
