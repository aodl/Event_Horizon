#![allow(dead_code)]
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::call::Call;

use crate::config::{REQUIRED_ENDOWMENT_E8S, RESERVE_PROTECTION_CYCLES};

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
pub enum CommitmentRoute {
    CyclesTopUp {
        canister_id: Principal,
    },
    RawIcp {
        destination_canister_id: Principal,
        memo: Vec<u8>,
    },
    NeuronStake {
        neuron_id: u64,
        memo: Option<Vec<u8>>,
    },
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct Args {
    routes: Vec<CommitmentRoute>,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct Summary {
    route: CommitmentRoute,
    qualifying_commitment_count: u64,
    total_qualifying_committed_e8s: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct CommitmentIndexFault {
    observed_at_ts: u64,
    last_cursor_tx_id: Option<u64>,
    offending_tx_id: u64,
    message: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
struct Response {
    items: Vec<Summary>,
    truncated: bool,
    complete_from_genesis: bool,
    commitment_index_fault: Option<CommitmentIndexFault>,
}

pub async fn route_is_admitted(
    historian: Principal,
    destination: Principal,
    memo: Vec<u8>,
) -> Result<bool, String> {
    let route = CommitmentRoute::RawIcp {
        destination_canister_id: destination,
        memo,
    };
    let args = Args {
        routes: vec![route.clone()],
    };
    let call = Call::bounded_wait(historian, "get_commitment_route_summaries").with_arg(&args);
    if ic_cdk::api::canister_liquid_cycle_balance()
        < RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
    {
        return Err("reserve_protection".to_string());
    }
    let response = call
        .await
        .map_err(|e| format!("historian transport: {e:?}"))?
        .candid::<Response>()
        .map_err(|e| format!("historian decode: {e:?}"))?;

    if response.truncated
        || !response.complete_from_genesis
        || response.commitment_index_fault.is_some()
    {
        return Ok(false);
    }
    let Some(summary) = response.items.into_iter().find(|item| item.route == route) else {
        return Ok(false);
    };
    Ok(summary.total_qualifying_committed_e8s >= REQUIRED_ENDOWMENT_E8S)
}
