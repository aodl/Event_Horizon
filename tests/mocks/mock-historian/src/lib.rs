#![allow(dead_code)]
use candid::{CandidType, Deserialize, Principal};
use std::cell::RefCell;

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq)]
enum CommitmentRoute {
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
#[derive(Clone, Debug, CandidType, Deserialize)]
struct Args {
    routes: Vec<CommitmentRoute>,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
struct Summary {
    route: CommitmentRoute,
    qualifying_commitment_count: u64,
    total_qualifying_committed_e8s: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
struct Fault {
    observed_at_ts: u64,
    last_cursor_tx_id: Option<u64>,
    offending_tx_id: u64,
    message: String,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
struct Response {
    items: Vec<Summary>,
    truncated: bool,
    complete_from_genesis: bool,
    indexed_through_staking_tx_id: Option<u64>,
    oldest_indexed_staking_tx_id: Option<u64>,
    last_index_run_ts: Option<u64>,
    commitment_index_fault: Option<Fault>,
    revision: Option<u64>,
    observed_head_staking_tx_id: Option<u64>,
    next_staking_start_tx_id: Option<u64>,
}
#[derive(Clone, Debug, CandidType, Deserialize)]
struct SetRoute {
    destination_canister_id: Principal,
    memo: Vec<u8>,
    total_e8s: u64,
}
#[derive(Clone)]
struct Rollup {
    destination: Principal,
    memo: Vec<u8>,
    total: u64,
}
struct State {
    complete: bool,
    fault: bool,
    rollups: Vec<Rollup>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            complete: true,
            fault: false,
            rollups: vec![],
        }
    }
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}
#[ic_cdk::init]
fn init() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}
#[ic_cdk::query]
fn get_commitment_route_summaries(args: Args) -> Response {
    STATE.with(|s| {
        let s = s.borrow();
        let items = args
            .routes
            .into_iter()
            .map(|route| {
                let total = match &route {
                    CommitmentRoute::RawIcp {
                        destination_canister_id,
                        memo,
                    } => s
                        .rollups
                        .iter()
                        .find(|r| r.destination == *destination_canister_id && r.memo == *memo)
                        .map(|r| r.total)
                        .unwrap_or(0),
                    _ => 0,
                };
                Summary {
                    route,
                    qualifying_commitment_count: if total > 0 { 1 } else { 0 },
                    total_qualifying_committed_e8s: total,
                }
            })
            .collect();
        Response {
            items,
            truncated: false,
            complete_from_genesis: s.complete,
            indexed_through_staking_tx_id: Some(1),
            oldest_indexed_staking_tx_id: Some(0),
            last_index_run_ts: Some(ic_cdk::api::time()),
            commitment_index_fault: s.fault.then(|| Fault {
                observed_at_ts: ic_cdk::api::time(),
                last_cursor_tx_id: Some(0),
                offending_tx_id: 0,
                message: "forced".into(),
            }),
            revision: Some(1),
            observed_head_staking_tx_id: Some(1),
            next_staking_start_tx_id: None,
        }
    })
}
#[ic_cdk::update]
fn debug_set_route(arg: SetRoute) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(r) = s
            .rollups
            .iter_mut()
            .find(|r| r.destination == arg.destination_canister_id && r.memo == arg.memo)
        {
            r.total = arg.total_e8s
        } else {
            s.rollups.push(Rollup {
                destination: arg.destination_canister_id,
                memo: arg.memo,
                total: arg.total_e8s,
            })
        }
    })
}
#[ic_cdk::update]
fn debug_set_complete(value: bool) {
    STATE.with(|s| s.borrow_mut().complete = value)
}
#[ic_cdk::update]
fn debug_set_fault(value: bool) {
    STATE.with(|s| s.borrow_mut().fault = value)
}

ic_cdk::export_candid!();
