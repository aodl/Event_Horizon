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
}

fn schedule_pricing(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, async {
        PRICING_TIMER.with_borrow_mut(|slot| {
            slot.take();
        });
        let acquired = PRICING_RUNNING.with(|flag| {
            if flag.get() {
                false
            } else {
                flag.set(true);
                true
            }
        });
        if !acquired {
            schedule_pricing(Duration::from_secs(1));
            return;
        }
        pricing::run_maintenance().await;
        PRICING_RUNNING.with(|flag| flag.set(false));
        let now = ic_cdk::api::time() / 1_000_000_000;
        schedule_pricing(Duration::from_secs(pricing::seconds_until_next_event(now)));
    });
    PRICING_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

fn schedule_poll(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, async {
        POLL_TIMER.with_borrow_mut(|slot| {
            slot.take();
        });
        let acquired = POLL_RUNNING.with(|flag| {
            if flag.get() {
                false
            } else {
                flag.set(true);
                true
            }
        });
        if !acquired {
            schedule_poll(Duration::from_secs(1));
            return;
        }
        polling::run_poll().await;
        POLL_RUNNING.with(|flag| flag.set(false));
        schedule_from_balance();
    });
    POLL_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

fn schedule_funding(delay: Duration) {
    let timer_id = ic_cdk_timers::set_timer(delay, async {
        FUNDING_TIMER.with_borrow_mut(|slot| {
            slot.take();
        });
        let acquired = FUNDING_RUNNING.with(|flag| {
            if flag.get() {
                false
            } else {
                flag.set(true);
                true
            }
        });
        if !acquired {
            schedule_funding(Duration::from_secs(60));
            return;
        }
        let cycles_minted = funding::run_funding_maintenance().await;
        FUNDING_RUNNING.with(|flag| flag.set(false));
        if cycles_minted {
            schedule_from_balance();
        }
        schedule_funding(Duration::from_secs(config::FUNDING_MAINTENANCE_SECONDS));
    });
    FUNDING_TIMER.with_borrow_mut(|slot| {
        if let Some(previous) = slot.replace(timer_id) {
            ic_cdk_timers::clear_timer(previous);
        }
    });
}

pub fn schedule_from_balance() {
    let balance = ic_cdk::api::canister_liquid_cycle_balance();
    let mut meta = state::read_metadata();
    let mode = next_mode(meta.polling_mode, balance);
    if mode != meta.polling_mode {
        logging::poll_mode_change(meta.polling_mode, mode, balance);
        meta.polling_mode = mode;
        state::write_metadata(meta);
    }
    let delay = match mode.delay_seconds() {
        Some(seconds) => Duration::from_secs(seconds),
        None => Duration::from_secs(config::RESERVE_RECHECK_SECONDS),
    };
    schedule_poll(delay);
}

pub fn start() {
    let balance = ic_cdk::api::canister_liquid_cycle_balance();
    let mut meta = state::read_metadata();
    let mode = next_mode(meta.polling_mode, balance);
    if mode != meta.polling_mode {
        logging::poll_mode_change(meta.polling_mode, mode, balance);
        meta.polling_mode = mode;
        state::write_metadata(meta);
    }
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
