//! Generated-shadow builds only. Never linked into the canonical canister.
use candid::CandidType;
use serde::Deserialize;
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Default)]
struct Counters {
    calls: u64,
    instructions: u64,
    linear_heap_pages: u64,
}
thread_local! { static COUNTERS: RefCell<BTreeMap<&'static str,Counters>> = const {RefCell::new(BTreeMap::new())}; }
pub(crate) struct Span {
    phase: &'static str,
    start: u64,
}
impl Span {
    pub(crate) fn new(phase: &'static str) -> Self {
        Self {
            phase,
            start: ic_cdk::api::instruction_counter(),
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let instructions = ic_cdk::api::instruction_counter().saturating_sub(self.start);
        #[cfg(target_arch = "wasm32")]
        let pages = core::arch::wasm32::memory_size::<0>() as u64;
        #[cfg(not(target_arch = "wasm32"))]
        let pages = 0;
        COUNTERS.with_borrow_mut(|all| {
            let count = all.entry(self.phase).or_default();
            count.calls = count.calls.saturating_add(1);
            count.instructions = count.instructions.saturating_add(instructions);
            count.linear_heap_pages = count.linear_heap_pages.max(pages);
        });
    }
}
#[derive(CandidType, Deserialize)]
struct Metric {
    phase: String,
    calls: u64,
    instructions: u64,
    linear_heap_pages: u64,
}
#[ic_cdk::query]
fn debug_optimisation_metrics() -> Vec<Metric> {
    COUNTERS.with_borrow(|all| {
        all.iter()
            .map(|(phase, c)| Metric {
                phase: phase.to_string(),
                calls: c.calls,
                instructions: c.instructions,
                linear_heap_pages: c.linear_heap_pages,
            })
            .collect()
    })
}
#[ic_cdk::update]
fn debug_reset_optimisation_metrics() {
    COUNTERS.with_borrow_mut(BTreeMap::clear);
}
