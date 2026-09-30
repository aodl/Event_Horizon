use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

use crate::{
    cadence::{next_mode, PollingMode},
    config, funding, logging, polling, pricing, state,
};

thread_local! {
    static POLL_TIMER: RefCell<Option<ic_cdk_timers::TimerId>> = const { RefCell::new(None) };
    static FUNDING_TIMER: RefCell<Option<ic_cdk_timers::TimerId>> = const { RefCell::new(None) };
    static PRICING_TIMER: RefCell<Option<ic_cdk_timers::TimerId>> = const { RefCell::new(None) };
    static POLL_RUNNING: Cell<bool> = const { Cell::new(false) };
    static FUNDING_RUNNING: Cell<bool> = const { Cell::new(false) };
    static PRICING_RUNNING: Cell<bool> = const { Cell::new(false) };
    #[cfg(feature = "debug_api")]
    static DEBUG_POST_AWAIT_TRAPS: Cell<u8> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_LIQUID_CYCLES_OVERRIDE: Cell<Option<u128>> = const { Cell::new(None) };
    #[cfg(feature = "debug_api")]
    static DEBUG_OVERLAP: Cell<u8> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_POLL_STARTS: Cell<u64> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_FUNDING_STARTS: Cell<u64> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_PRICING_STARTS: Cell<u64> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_POLL_BUSY: Cell<u64> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_FUNDING_BUSY: Cell<u64> = const { Cell::new(0) };
    #[cfg(feature = "debug_api")]
    static DEBUG_PRICING_BUSY: Cell<u64> = const { Cell::new(0) };
}

#[derive(Clone, Copy)]
enum Lane {
    Poll,
    Funding,
    Pricing,
}

impl Lane {
    fn try_acquire(self) -> Option<LaneGuard> {
        let acquired = match self {
            Self::Poll => POLL_RUNNING.with(|flag| {
                if flag.get() {
                    false
                } else {
                    flag.set(true);
                    true
                }
            }),
            Self::Funding => FUNDING_RUNNING.with(|flag| {
                if flag.get() {
                    false
                } else {
                    flag.set(true);
                    true
                }
            }),
            Self::Pricing => PRICING_RUNNING.with(|flag| {
                if flag.get() {
                    false
                } else {
                    flag.set(true);
                    true
                }
            }),
        };
        // Construct the armed guard only after acquisition; a rejected contender
        // must not drop a guard that releases another worker's lane.
        if acquired {
            Some(LaneGuard {
                lane: self,
                armed: true,
            })
        } else {
            None
        }
    }

    fn clear_running(self) {
        match self {
            Self::Poll => POLL_RUNNING.with(|flag| flag.set(false)),
            Self::Funding => FUNDING_RUNNING.with(|flag| flag.set(false)),
            Self::Pricing => PRICING_RUNNING.with(|flag| flag.set(false)),
        }
    }

    fn has_timer(self) -> bool {
        match self {
            Self::Poll => POLL_TIMER.with_borrow(|slot| slot.is_some()),
            Self::Funding => FUNDING_TIMER.with_borrow(|slot| slot.is_some()),
            Self::Pricing => PRICING_TIMER.with_borrow(|slot| slot.is_some()),
        }
    }

    fn schedule_recovery(self) {
        let delay = Duration::from_secs(config::RESERVE_RECHECK_SECONDS);
        match self {
            Self::Poll => schedule_poll(delay),
            Self::Funding => schedule_funding(delay),
            Self::Pricing => schedule_pricing(delay),
        }
    }

    #[cfg(feature = "debug_api")]
    const fn debug_bit(self) -> u8 {
        match self {
            Self::Poll => 1,
            Self::Funding => 2,
            Self::Pricing => 4,
        }
    }
}

/// Owns one scheduler lane until its worker has completed and a replacement timer is safe.
///
/// The pinned CDK cancels and drops a protected async task when a continuation traps. The
/// destructor therefore performs only synchronous heap cleanup: release this lane and restore
/// one delayed attempt if no legitimate replacement timer already exists.
struct LaneGuard {
    lane: Lane,
    armed: bool,
}

impl LaneGuard {
    fn finish(mut self) {
        self.lane.clear_running();
        self.armed = false;
    }
}

impl Drop for LaneGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.lane.clear_running();
        if !self.lane.has_timer() {
            self.lane.schedule_recovery();
        }
    }
}

#[cfg(feature = "debug_api")]
fn take_debug_post_await_trap(lane: Lane) -> bool {
    DEBUG_POST_AWAIT_TRAPS.with(|faults| {
        let value = faults.get();
        let bit = lane.debug_bit();
        let armed = value & bit != 0;
        if armed {
            // Consume before the awaited boundary. A trap in the continuation must not
            // roll this back and turn a one-shot fault into a permanent fault.
            faults.set(value & !bit);
        }
        armed
    })
}

#[cfg(not(feature = "debug_api"))]
const fn take_debug_post_await_trap(_lane: Lane) -> bool {
    false
}

fn maybe_debug_trap_after_await(armed: bool, lane: &str) {
    #[cfg(feature = "debug_api")]
    if armed {
        ic_cdk::trap(format!("forced {lane} scheduler trap after await"));
    }
    #[cfg(not(feature = "debug_api"))]
    let _ = (armed, lane);
}

#[cfg(feature = "debug_api")]
fn debug_note_start(lane: Lane) {
    let counter = match lane {
        Lane::Poll => &DEBUG_POLL_STARTS,
        Lane::Funding => &DEBUG_FUNDING_STARTS,
        Lane::Pricing => &DEBUG_PRICING_STARTS,
    };
    counter.with(|value| value.set(value.get().saturating_add(1)));
}

#[cfg(not(feature = "debug_api"))]
const fn debug_note_start(_lane: Lane) {}

#[cfg(feature = "debug_api")]
fn debug_note_busy(lane: Lane) {
    let counter = match lane {
        Lane::Poll => &DEBUG_POLL_BUSY,
        Lane::Funding => &DEBUG_FUNDING_BUSY,
        Lane::Pricing => &DEBUG_PRICING_BUSY,
    };
    counter.with(|value| value.set(value.get().saturating_add(1)));
}

#[cfg(not(feature = "debug_api"))]
const fn debug_note_busy(_lane: Lane) {}

#[cfg(feature = "debug_api")]
fn take_debug_overlap(lane: Lane) -> bool {
    DEBUG_OVERLAP.with(|overlap| {
        let value = overlap.get();
        let bit = lane.debug_bit();
        let armed = value & bit != 0;
        if armed {
            overlap.set(value & !bit);
        }
        armed
    })
}

#[cfg(not(feature = "debug_api"))]
const fn take_debug_overlap(_lane: Lane) -> bool {
    false
}

async fn maybe_debug_exercise_overlap(lane: Lane) {
    if take_debug_overlap(lane) {
        #[cfg(feature = "debug_api")]
        ic_cdk::call::Call::unbounded_wait(ic_cdk::api::canister_self(), "debug_scheduler_overlap")
            .with_arg(match lane {
                Lane::Poll => "poll",
                Lane::Funding => "funding",
                Lane::Pricing => "pricing",
            })
            .await
            .unwrap_or_else(|error| ic_cdk::trap(format!("scheduler barrier failed: {error:?}")));
    }
}

async fn run_pricing_timer() {
    PRICING_TIMER.with_borrow_mut(|slot| {
        slot.take();
    });
    let Some(guard) = Lane::Pricing.try_acquire() else {
        debug_note_busy(Lane::Pricing);
        schedule_pricing(Duration::from_secs(1));
        return;
    };
    debug_note_start(Lane::Pricing);
    maybe_debug_exercise_overlap(Lane::Pricing).await;
    let trap_after_await = take_debug_post_await_trap(Lane::Pricing);
    pricing::run_maintenance().await;
    maybe_debug_trap_after_await(trap_after_await, "pricing");
    let now = ic_cdk::api::time() / 1_000_000_000;
    schedule_pricing(Duration::from_secs(pricing::seconds_until_next_event(now)));
    guard.finish();
}

fn schedule_pricing(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, run_pricing_timer());
    PRICING_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

async fn run_poll_timer() {
    POLL_TIMER.with_borrow_mut(|slot| {
        slot.take();
    });
    let Some(guard) = Lane::Poll.try_acquire() else {
        debug_note_busy(Lane::Poll);
        schedule_poll(Duration::from_secs(1));
        return;
    };
    debug_note_start(Lane::Poll);
    maybe_debug_exercise_overlap(Lane::Poll).await;
    if refresh_polling_mode() == PollingMode::ReserveProtection {
        schedule_poll(Duration::from_secs(config::RESERVE_RECHECK_SECONDS));
        guard.finish();
        return;
    }
    let trap_after_await = take_debug_post_await_trap(Lane::Poll);
    polling::run_poll().await;
    maybe_debug_trap_after_await(trap_after_await, "poll");
    schedule_from_balance();
    guard.finish();
}

fn schedule_poll(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, run_poll_timer());
    POLL_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

async fn run_funding_timer() {
    FUNDING_TIMER.with_borrow_mut(|slot| {
        slot.take();
    });
    let Some(guard) = Lane::Funding.try_acquire() else {
        debug_note_busy(Lane::Funding);
        schedule_funding(Duration::from_secs(60));
        return;
    };
    debug_note_start(Lane::Funding);
    maybe_debug_exercise_overlap(Lane::Funding).await;
    let trap_after_await = take_debug_post_await_trap(Lane::Funding);
    let cycles_minted = funding::run_funding_maintenance().await;
    maybe_debug_trap_after_await(trap_after_await, "funding");
    if cycles_minted {
        schedule_from_balance();
    }
    schedule_funding(Duration::from_secs(config::FUNDING_MAINTENANCE_SECONDS));
    guard.finish();
}

fn schedule_funding(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, run_funding_timer());
    FUNDING_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

fn liquid_cycles() -> u128 {
    #[cfg(feature = "debug_api")]
    if let Some(value) = DEBUG_LIQUID_CYCLES_OVERRIDE.with(Cell::get) {
        return value;
    }
    ic_cdk::api::canister_liquid_cycle_balance()
}

fn refresh_polling_mode() -> PollingMode {
    let balance = liquid_cycles();
    let mut meta = state::read_metadata();
    let mode = next_mode(meta.polling_mode, balance);
    if mode != meta.polling_mode {
        logging::poll_mode_change(meta.polling_mode, mode, balance);
        meta.polling_mode = mode;
        state::write_metadata(meta);
    }
    mode
}

pub fn schedule_from_balance() {
    let mode = refresh_polling_mode();
    let delay = match mode.delay_seconds() {
        Some(seconds) => Duration::from_secs(seconds),
        None => Duration::from_secs(config::RESERVE_RECHECK_SECONDS),
    };
    schedule_poll(delay);
}

pub fn start() {
    let mode = refresh_polling_mode();
    if mode == PollingMode::ReserveProtection {
        schedule_poll(Duration::from_secs(config::RESERVE_RECHECK_SECONDS));
    } else {
        // Install/upgrade should restore liveness promptly; normal cadence starts after this poll.
        schedule_poll(Duration::ZERO);
    }
    schedule_funding(Duration::ZERO);
    schedule_pricing(Duration::ZERO);
}

#[cfg(feature = "debug_api")]
pub async fn debug_poll_once() {
    polling::run_poll().await;
}

#[cfg(feature = "debug_api")]
pub async fn debug_funding_once() {
    if funding::run_funding_maintenance().await {
        schedule_from_balance();
    }
}

#[cfg(feature = "debug_api")]
pub async fn debug_pricing_once() {
    pricing::run_maintenance().await;
}

#[cfg(feature = "debug_api")]
pub fn debug_start() {
    start();
}

#[cfg(feature = "debug_api")]
pub fn debug_timer_count() -> u8 {
    u8::from(POLL_TIMER.with_borrow(|slot| slot.is_some()))
        + u8::from(FUNDING_TIMER.with_borrow(|slot| slot.is_some()))
        + u8::from(PRICING_TIMER.with_borrow(|slot| slot.is_some()))
}

#[cfg(feature = "debug_api")]
pub fn debug_arm_post_await_trap(lane: &str) {
    let lane = match lane {
        "poll" => Lane::Poll,
        "funding" => Lane::Funding,
        "pricing" => Lane::Pricing,
        _ => ic_cdk::trap("unknown scheduler lane"),
    };
    DEBUG_POST_AWAIT_TRAPS.with(|faults| faults.set(faults.get() | lane.debug_bit()));
}

#[cfg(feature = "debug_api")]
pub fn debug_lane_state(lane: &str) -> (bool, bool) {
    match lane {
        "poll" => (
            POLL_RUNNING.with(Cell::get),
            POLL_TIMER.with_borrow(|slot| slot.is_some()),
        ),
        "funding" => (
            FUNDING_RUNNING.with(Cell::get),
            FUNDING_TIMER.with_borrow(|slot| slot.is_some()),
        ),
        "pricing" => (
            PRICING_RUNNING.with(Cell::get),
            PRICING_TIMER.with_borrow(|slot| slot.is_some()),
        ),
        _ => ic_cdk::trap("unknown scheduler lane"),
    }
}

#[cfg(feature = "debug_api")]
pub fn debug_set_liquid_cycles_override(value: Option<u128>) {
    DEBUG_LIQUID_CYCLES_OVERRIDE.with(|slot| slot.set(value));
}

#[cfg(feature = "debug_api")]
pub fn debug_arm_overlap(lane: &str) {
    let lane = match lane {
        "poll" => Lane::Poll,
        "funding" => Lane::Funding,
        "pricing" => Lane::Pricing,
        _ => ic_cdk::trap("unknown scheduler lane"),
    };
    DEBUG_OVERLAP.with(|overlap| overlap.set(overlap.get() | lane.debug_bit()));
}

#[cfg(feature = "debug_api")]
pub async fn debug_run_lane(lane: &str) {
    match lane {
        "poll" => run_poll_timer().await,
        "funding" => run_funding_timer().await,
        "pricing" => run_pricing_timer().await,
        _ => ic_cdk::trap("unknown scheduler lane"),
    }
}

#[cfg(feature = "debug_api")]
pub fn debug_lane_counts(lane: &str) -> (u64, u64) {
    match lane {
        "poll" => (
            DEBUG_POLL_STARTS.with(Cell::get),
            DEBUG_POLL_BUSY.with(Cell::get),
        ),
        "funding" => (
            DEBUG_FUNDING_STARTS.with(Cell::get),
            DEBUG_FUNDING_BUSY.with(Cell::get),
        ),
        "pricing" => (
            DEBUG_PRICING_STARTS.with(Cell::get),
            DEBUG_PRICING_BUSY.with(Cell::get),
        ),
        _ => ic_cdk::trap("unknown scheduler lane"),
    }
}
