#[cfg(feature = "debug_api")]
use std::cell::Cell;

use crate::{
    account::{account_identifier_bytes, principal_to_subaccount},
    clients::{cmc, icp_ledger},
    config, logging,
    state::{self, FundingState, PlannedSurplus},
    surplus,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FundingPlan {
    retained_e8s: u64,
    retained_fee_e8s: u64,
    surplus_e8s: u64,
    surplus_fee_e8s: u64,
}

#[cfg(feature = "debug_api")]
thread_local! {
    static DEBUG_LIQUID_CYCLES_OVERRIDE: Cell<Option<u128>> = const { Cell::new(None) };
}

fn liquid_cycles() -> u128 {
    #[cfg(feature = "debug_api")]
    if let Some(value) = DEBUG_LIQUID_CYCLES_OVERRIDE.with(Cell::get) {
        return value;
    }
    ic_cdk::api::canister_liquid_cycle_balance()
}

#[cfg(feature = "debug_api")]
pub fn debug_set_liquid_cycles_override(value: Option<u128>) {
    DEBUG_LIQUID_CYCLES_OVERRIDE.with(|slot| slot.set(value));
}

fn plan_funding(balance_e8s: u64, fee_e8s: u64, diversion_percent: u8) -> Option<FundingPlan> {
    if balance_e8s <= fee_e8s {
        return None;
    }
    let cmc_only = || FundingPlan {
        retained_e8s: balance_e8s - fee_e8s,
        retained_fee_e8s: fee_e8s,
        surplus_e8s: 0,
        surplus_fee_e8s: 0,
    };
    if diversion_percent == 0 || balance_e8s <= fee_e8s.saturating_mul(2) {
        return Some(cmc_only());
    }

    let allocatable = balance_e8s - 2 * fee_e8s;
    let percent = diversion_percent.min(config::SURPLUS_MAX_PERCENT);
    let surplus_e8s = ((u128::from(allocatable) * u128::from(percent)) / 100) as u64;
    let retained_e8s = allocatable - surplus_e8s;
    if surplus_e8s == 0 || retained_e8s == 0 {
        return Some(cmc_only());
    }
    Some(FundingPlan {
        retained_e8s,
        retained_fee_e8s: fee_e8s,
        surplus_e8s,
        surplus_fee_e8s: fee_e8s,
    })
}

fn legacy_transfer_arg(
    destination: [u8; 32],
    memo: u64,
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
) -> icp_ledger::LegacyTransferArg {
    icp_ledger::LegacyTransferArg {
        memo,
        amount: icp_ledger::Tokens { e8s: amount_e8s },
        fee: icp_ledger::Tokens { e8s: fee_e8s },
        from_subaccount: None,
        to: destination.to_vec(),
        created_at_time: Some(icp_ledger::TimeStamp {
            timestamp_nanos: created_at_time_nanos,
        }),
    }
}

fn cmc_transfer_arg(
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
) -> icp_ledger::LegacyTransferArg {
    let runtime = config::runtime();
    let cmc_subaccount = principal_to_subaccount(ic_cdk::api::canister_self());
    legacy_transfer_arg(
        account_identifier_bytes(runtime.cmc_canister, cmc_subaccount),
        config::TOP_UP_CANISTER_MEMO,
        amount_e8s,
        fee_e8s,
        created_at_time_nanos,
    )
}

fn surplus_transfer_arg(
    destination: [u8; 32],
    memo: u64,
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
) -> icp_ledger::LegacyTransferArg {
    legacy_transfer_arg(
        destination,
        memo,
        amount_e8s,
        fee_e8s,
        created_at_time_nanos,
    )
}

fn configured_surplus_identity() -> Option<([u8; 32], u64)> {
    config::runtime().surplus_canister.map(|destination| {
        (
            account_identifier_bytes(destination, [0; 32]),
            config::SURPLUS_TRANSFER_MEMO,
        )
    })
}

async fn resume_cmc_transfer(
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
    planned_surplus: Option<PlannedSurplus>,
) -> bool {
    let runtime = config::runtime();
    let arg = cmc_transfer_arg(amount_e8s, fee_e8s, created_at_time_nanos);
    match icp_ledger::legacy_transfer(runtime.icp_ledger, &arg).await {
        icp_ledger::LegacyTransferOutcome::Accepted(block_index) => {
            state::write_funding_state(FundingState::CmcNotifyPending {
                block_index,
                planned_surplus,
            });
            true
        }
        icp_ledger::LegacyTransferOutcome::RetrySameIdentity
        | icp_ledger::LegacyTransferOutcome::Uncertain(_) => false,
        icp_ledger::LegacyTransferOutcome::IdentityExpired => {
            logging::cmc_transfer_identity_expired();
            state::write_funding_state(FundingState::Idle);
            false
        }
        icp_ledger::LegacyTransferOutcome::Replan => {
            state::write_funding_state(FundingState::Idle);
            false
        }
    }
}

async fn resume_surplus_transfer(
    destination: [u8; 32],
    memo: u64,
    amount_e8s: u64,
    fee_e8s: u64,
    created_at_time_nanos: u64,
) {
    let runtime = config::runtime();
    let arg = surplus_transfer_arg(
        destination,
        memo,
        amount_e8s,
        fee_e8s,
        created_at_time_nanos,
    );
    match icp_ledger::legacy_transfer(runtime.icp_ledger, &arg).await {
        icp_ledger::LegacyTransferOutcome::Accepted(_) => {
            state::write_funding_state(FundingState::Idle);
        }
        icp_ledger::LegacyTransferOutcome::RetrySameIdentity
        | icp_ledger::LegacyTransferOutcome::Uncertain(_) => {}
        icp_ledger::LegacyTransferOutcome::IdentityExpired => {
            logging::surplus_transfer_identity_expired();
            state::write_funding_state(FundingState::Idle);
        }
        icp_ledger::LegacyTransferOutcome::Replan => {
            state::write_funding_state(FundingState::Idle);
        }
    }
}

async fn resume_notify(block_index: u64, planned_surplus: Option<PlannedSurplus>) -> bool {
    let runtime = config::runtime();
    let self_id = ic_cdk::api::canister_self();
    match cmc::notify_top_up(runtime.cmc_canister, self_id, block_index).await {
        cmc::NotifyOutcome::Success => {
            let Some(planned_surplus) = planned_surplus else {
                state::write_funding_state(FundingState::Idle);
                return true;
            };
            if liquid_cycles() < config::SURPLUS_HEALTH_THRESHOLD_CYCLES {
                state::write_funding_state(FundingState::Idle);
                return true;
            }
            let created_at_time_nanos = ic_cdk::api::time();
            state::write_funding_state(FundingState::SurplusTransferPending {
                destination: planned_surplus.destination,
                memo: planned_surplus.memo,
                amount_e8s: planned_surplus.amount_e8s,
                fee_e8s: planned_surplus.fee_e8s,
                created_at_time_nanos,
            });
            resume_surplus_transfer(
                planned_surplus.destination,
                planned_surplus.memo,
                planned_surplus.amount_e8s,
                planned_surplus.fee_e8s,
                created_at_time_nanos,
            )
            .await;
            true
        }
        cmc::NotifyOutcome::Refunded => {
            state::write_funding_state(FundingState::Idle);
            false
        }
        cmc::NotifyOutcome::RetryLater => false,
        cmc::NotifyOutcome::Terminal(reason) => {
            logging::cmc_terminal(&format!("block={block_index} {reason}"));
            state::write_funding_state(FundingState::Idle);
            false
        }
    }
}

async fn continue_funding_state(current: FundingState) -> Option<bool> {
    match current {
        FundingState::CmcNotifyPending {
            block_index,
            planned_surplus,
        } => Some(resume_notify(block_index, planned_surplus).await),
        FundingState::CmcTransferPending {
            amount_e8s,
            fee_e8s,
            created_at_time_nanos,
            planned_surplus,
        } => {
            if resume_cmc_transfer(amount_e8s, fee_e8s, created_at_time_nanos, planned_surplus)
                .await
            {
                if let FundingState::CmcNotifyPending {
                    block_index,
                    planned_surplus,
                } = state::read_funding_state()
                {
                    return Some(resume_notify(block_index, planned_surplus).await);
                }
            }
            Some(false)
        }
        FundingState::SurplusTransferPending {
            destination,
            memo,
            amount_e8s,
            fee_e8s,
            created_at_time_nanos,
        } => {
            resume_surplus_transfer(
                destination,
                memo,
                amount_e8s,
                fee_e8s,
                created_at_time_nanos,
            )
            .await;
            Some(false)
        }
        FundingState::Idle => None,
    }
}

pub async fn run_funding_maintenance() -> bool {
    let runtime = config::runtime();
    let liquid_cycles = liquid_cycles();
    let policy = surplus::observe(
        state::read_surplus_policy(),
        runtime.surplus_canister.is_some(),
        ic_cdk::api::time() / 1_000_000_000,
        liquid_cycles,
    );
    state::write_surplus_policy(policy.clone());

    if let Some(result) = continue_funding_state(state::read_funding_state()).await {
        return result;
    }

    let self_id = ic_cdk::api::canister_self();
    let balance = match icp_ledger::icrc1_balance_of(
        runtime.icp_ledger,
        icp_ledger::Account {
            owner: self_id,
            subaccount: None,
        },
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return false,
    };
    let fee = match icp_ledger::icrc1_fee(runtime.icp_ledger).await {
        Ok(value) => value,
        Err(_) => return false,
    };
    let effective_percent =
        surplus::effective_percent(&policy, runtime.surplus_canister.is_some(), liquid_cycles);
    let Some(plan) = plan_funding(balance, fee, effective_percent) else {
        return false;
    };

    let created_at_time_nanos = ic_cdk::api::time();
    let planned_surplus = if plan.surplus_e8s > 0 {
        match configured_surplus_identity() {
            Some((destination, memo)) => Some(PlannedSurplus {
                destination,
                memo,
                amount_e8s: plan.surplus_e8s,
                fee_e8s: plan.surplus_fee_e8s,
            }),
            None => {
                logging::surplus_financial_invariant("split planned without surplus destination");
                return false;
            }
        }
    } else {
        None
    };
    let next = FundingState::CmcTransferPending {
        amount_e8s: plan.retained_e8s,
        fee_e8s: plan.retained_fee_e8s,
        created_at_time_nanos,
        planned_surplus,
    };
    state::write_funding_state(next.clone());
    continue_funding_state(next).await.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_arithmetic_covers_policy_levels_and_favors_retained() {
        let balance = 1_000_003;
        let fee = 10_000;
        for percent in [5, 50, 95] {
            let plan = plan_funding(balance, fee, percent).unwrap();
            assert_eq!(
                plan.retained_e8s + plan.surplus_e8s + plan.retained_fee_e8s + plan.surplus_fee_e8s,
                balance
            );
            let allocatable = balance - 2 * fee;
            assert_eq!(
                plan.surplus_e8s,
                ((u128::from(allocatable) * u128::from(percent)) / 100) as u64
            );
            assert_eq!(plan.retained_e8s, allocatable - plan.surplus_e8s);
        }
        let no_diversion = plan_funding(balance, fee, 0).unwrap();
        assert_eq!(no_diversion.retained_e8s + fee, balance);
        assert_eq!(no_diversion.surplus_e8s, 0);
    }

    #[test]
    fn split_uses_u128_and_preserves_a_retained_share_at_95_percent() {
        let plan = plan_funding(u64::MAX, 1, 95).unwrap();
        assert!(plan.retained_e8s > 0);
        assert_eq!(plan.retained_e8s + plan.surplus_e8s + 2, u64::MAX);
    }

    #[test]
    fn insufficient_or_dust_split_falls_back_to_cmc_only() {
        let at_two_fees = plan_funding(20_000, 10_000, 95).unwrap();
        assert_eq!(at_two_fees.surplus_e8s, 0);
        assert_eq!(at_two_fees.retained_e8s, 10_000);

        let dust = plan_funding(20_001, 10_000, 5).unwrap();
        assert_eq!(dust.surplus_e8s, 0);
        assert_eq!(dust.retained_e8s, 10_001);
        assert!(plan_funding(10_000, 10_000, 95).is_none());
    }
}
