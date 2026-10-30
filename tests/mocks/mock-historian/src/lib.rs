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
#[derive(Clone, Debug, CandidType, Deserialize)]
struct ObservedRoute {
    destination: Principal,
    memo: Vec<u8>,
}
#[derive(Clone, Default, CandidType, Deserialize)]
struct RouteGate {
    entered: u64,
    waiting: bool,
    routes: Vec<ObservedRoute>,
}
thread_local! {
    static ROUTE_GATE: RefCell<RouteGate> = RefCell::new(RouteGate::default());
    static ROUTE_GATE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ROUTE_GATE_ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[ic_cdk::update]
fn debug_route_gate(hold: bool) {
    if hold {
        ROUTE_GATE.with_borrow_mut(|g| {
            *g = RouteGate {
                waiting: true,
                ..Default::default()
            }
        });
        ROUTE_GATE_ARMED.set(true);
        ROUTE_GATE_ENABLED.set(true);
    } else {
        ROUTE_GATE.with_borrow_mut(|g| g.waiting = false);
    }
}
#[ic_cdk::query]
fn debug_route_gate_state() -> RouteGate {
    ROUTE_GATE.with_borrow(Clone::clone)
}
#[ic_cdk::update]
fn debug_route_gate_tick() {}

// Only the controlled-interleaving fixture changes method mode. A replicated
// query cannot await; the update wrapper delegates to the identical pure response.
#[cfg(feature = "interleaving_gate")]
#[ic_cdk::update]
async fn get_commitment_route_summaries(args: Args) -> Response {
    // Test-only controlled interleaving. Default fixtures do not enter this lane.
    if ROUTE_GATE_ENABLED.get() {
        ROUTE_GATE.with_borrow_mut(|g| {
            g.entered += 1;
            for route in &args.routes {
                if let CommitmentRoute::RawIcp {
                    destination_canister_id,
                    memo,
                } = route
                {
                    assert!(g.routes.len() < 16);
                    g.routes.push(ObservedRoute {
                        destination: *destination_canister_id,
                        memo: memo.clone(),
                    });
                }
            }
        });
        if ROUTE_GATE_ARMED.replace(false) {
            assert!(
                ic_cdk::api::in_replicated_execution(),
                "gate requires an inter-canister call"
            );
            while ROUTE_GATE.with_borrow(|g| g.waiting) {
                ic_cdk::call::Call::bounded_wait(
                    ic_cdk::api::canister_self(),
                    "debug_route_gate_tick",
                )
                .await
                .expect("mock gate tick");
            }
        }
    }
    route_response(args)
}
#[cfg(not(feature = "interleaving_gate"))]
#[ic_cdk::query]
fn get_commitment_route_summaries(args: Args) -> Response {
    route_response(args)
}
fn route_response(args: Args) -> Response {
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

#[derive(Clone, Debug, CandidType, Deserialize)]
struct CanisterAlias {
    alias: String,
    canister_id: Principal,
}
#[derive(CandidType, Deserialize)]
struct ListCanisterAliasesArgs {
    start_after: Option<String>,
    limit: Option<u32>,
}
#[derive(CandidType, Deserialize)]
struct ListCanisterAliasesResponse {
    revision: Vec<u8>,
    items: Vec<CanisterAlias>,
    next_start_after: Option<String>,
}
#[derive(Clone, Default, CandidType, Deserialize)]
struct AliasFixture {
    items: Vec<CanisterAlias>,
    mode: String,
    page_size: Option<u32>,
}
thread_local! {static ALIAS_FIXTURE:RefCell<Option<AliasFixture>>=const{RefCell::new(None)};static ALIAS_CALLS:std::cell::Cell<u64>=const{std::cell::Cell::new(0)};}
fn alias_digest(items: &[CanisterAlias]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"JUPITER_CANISTER_ALIASES_V1\0");
    h.update((items.len() as u32).to_be_bytes());
    for i in items {
        h.update([i.alias.len() as u8]);
        h.update(i.alias.as_bytes());
        h.update([i.canister_id.as_slice().len() as u8]);
        h.update(i.canister_id.as_slice());
    }
    h.finalize().to_vec()
}
#[ic_cdk::query(manual_reply = true)]
fn list_canister_aliases(
    args: ListCanisterAliasesArgs,
) -> std::marker::PhantomData<ListCanisterAliasesResponse> {
    ALIAS_CALLS.with(|c| c.set(c.get() + 1));
    let f = ALIAS_FIXTURE
        .with_borrow(|v| v.clone())
        .unwrap_or_else(|| AliasFixture {
            items: vec![CanisterAlias {
                alias: "E".into(),
                canister_id: Principal::from_text("eo6ei-gaaaa-aaaar-qchra-cai").unwrap(),
            }],
            ..Default::default()
        });
    if f.mode == "trap" {
        ic_cdk::trap("alias fixture unavailable")
    }
    if f.mode == "decode" {
        ic_cdk::api::msg_reply(candid::encode_one(7u8).unwrap());
        return std::marker::PhantomData;
    }
    let mut revision = alias_digest(&f.items);
    if f.mode == "oversize" {
        revision.resize(65_536, 0);
    }
    if f.mode == "revision" || (f.mode == "mixed" && args.start_after.is_some()) {
        revision[0] ^= 1;
    }
    let start = args
        .start_after
        .as_ref()
        .map_or(0, |a| f.items.partition_point(|i| i.alias <= *a));
    let size = args.limit.unwrap_or(128).min(f.page_size.unwrap_or(128)) as usize;
    let mut items = f.items[start..f.items.len().min(start + size)].to_vec();
    let mut next = (start + items.len() < f.items.len())
        .then(|| items.last().map(|i| i.alias.clone()).unwrap_or_default());
    match f.mode.as_str() {
        "invalid" => {
            if let Some(i) = items.first_mut() {
                i.alias = "lower".into();
            }
        }
        "principal" => {
            if let Some(i) = items.first_mut() {
                i.canister_id = Principal::anonymous();
            }
        }
        "cursor" => next = Some(args.start_after.unwrap_or_default()),
        "empty" => {
            items.clear();
            next = Some("A".into())
        }
        "unsorted" => items.reverse(),
        "duplicate" => {
            if let Some(i) = items.first().cloned() {
                items.push(i)
            }
        }
        _ => {}
    }
    ic_cdk::api::msg_reply(
        candid::encode_one(ListCanisterAliasesResponse {
            revision,
            items,
            next_start_after: next,
        })
        .unwrap(),
    );
    std::marker::PhantomData
}
#[ic_cdk::update]
fn debug_alias_fixture(value: AliasFixture) {
    ALIAS_FIXTURE.with_borrow_mut(|v| *v = Some(value));
}
#[ic_cdk::query]
fn debug_alias_calls() -> u64 {
    ALIAS_CALLS.with(std::cell::Cell::get)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod alias_contract_tests {
    #[test]
    fn published_alias_contract_matches_generated_mock_candid() {
        let generated = super::__export_service();
        let expected = include_str!("../../../../candid/jupiter-historian.subset.did");
        for name in [
            "CanisterAlias",
            "ListCanisterAliasesArgs",
            "ListCanisterAliasesResponse",
        ] {
            let declaration = format!("type {name} = record {{");
            let fields = |s: &str| {
                s.split(&declaration)
                    .nth(1)
                    .unwrap()
                    .split('}')
                    .next()
                    .unwrap()
                    .split(';')
                    .map(|s| {
                        s.split_whitespace()
                            .collect::<String>()
                            .replace("blob", "vecnat8")
                    })
                    .filter(|s| !s.is_empty())
                    .collect::<std::collections::BTreeSet<_>>()
            };
            assert_eq!(fields(&generated), fields(expected));
        }
        assert!(generated.split_whitespace().collect::<String>().replace(",)", ")").contains(
            "list_canister_aliases:(ListCanisterAliasesArgs)->(ListCanisterAliasesResponse)query;"
        ));
    }
}
