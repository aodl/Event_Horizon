//! Intentionally sparse logging. Public canister logs have a small rolling buffer.

use std::cell::Cell;

thread_local! {
    static OBSERVED_LEDGER_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static ICP_LEDGER_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static SHARED_LEDGER_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static HISTORIAN_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static OBSERVED_HISTORY_GAP_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ADMISSION_HISTORY_GAP_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static SHARED_CURSOR_INVALID: Cell<bool> = const { Cell::new(false) };
}

pub fn history_gap(stream: &str, start: u64, end_exclusive: u64) {
    if end_exclusive <= start {
        return;
    }
    let key = if stream == "observed" {
        &OBSERVED_HISTORY_GAP_ACTIVE
    } else {
        &ADMISSION_HISTORY_GAP_ACTIVE
    };
    key.with(|flag| {
        if !flag.replace(true) {
            // A long archived prefix can span many bounded Ledger pages. Log the
            // exceptional transition once rather than evicting useful public logs
            // with one line per skipped page.
            ic_cdk::println!(
                "HISTORY_GAP stream={} first_skipped={}..{}",
                stream,
                start,
                end_exclusive - 1
            );
        }
    });
}

pub fn observed_ledger_failure(message: &str) {
    OBSERVED_LEDGER_UNAVAILABLE.with(|flag| {
        if !flag.replace(true) {
            ic_cdk::println!("OBSERVED_LEDGER_UNAVAILABLE {}", message);
        }
    });
}

pub fn observed_ledger_recovered() {
    OBSERVED_LEDGER_UNAVAILABLE.with(|f| f.set(false));
    OBSERVED_HISTORY_GAP_ACTIVE.with(|f| f.set(false));
}
pub fn icp_admission_ledger_failure(message: &str) {
    ICP_LEDGER_UNAVAILABLE.with(|f| {
        if !f.replace(true) {
            ic_cdk::println!("ICP_ADMISSION_LEDGER_UNAVAILABLE {}", message)
        }
    });
}
pub fn icp_admission_ledger_recovered() {
    ICP_LEDGER_UNAVAILABLE.with(|f| f.set(false));
    ADMISSION_HISTORY_GAP_ACTIVE.with(|f| f.set(false));
}
pub fn ledger_recovered_all() {
    observed_ledger_recovered();
    icp_admission_ledger_recovered();
    SHARED_LEDGER_UNAVAILABLE.with(|f| f.set(false));
}

pub fn shared_ledger_failure(message: &str) {
    SHARED_LEDGER_UNAVAILABLE.with(|flag| {
        if !flag.replace(true) {
            ic_cdk::println!("SHARED_ICP_LEDGER_UNAVAILABLE {}", message);
        }
    });
}

pub fn shared_cursor_invariant(message: &str) {
    SHARED_CURSOR_INVALID.with(|flag| {
        if !flag.replace(true) {
            ic_cdk::println!("SHARED_CURSOR_INVARIANT {}", message);
        }
    });
}
pub fn shared_cursor_recovered() {
    SHARED_CURSOR_INVALID.with(|flag| flag.set(false));
}

pub fn historian_failure(message: &str) {
    HISTORIAN_UNAVAILABLE.with(|flag| {
        if !flag.replace(true) {
            ic_cdk::println!("HISTORIAN_ADMISSION_UNAVAILABLE {}", message);
        }
    });
}

pub fn historian_recovered() {
    HISTORIAN_UNAVAILABLE.with(|flag| flag.set(false));
}

pub fn poll_mode_change(
    from: crate::cadence::PollingMode,
    to: crate::cadence::PollingMode,
    balance: u128,
) {
    ic_cdk::println!("POLL_MODE_CHANGE from={from:?} to={to:?} liquid_cycles={balance}");
}

pub fn daily_health() {
    let day = ic_cdk::api::time() / 1_000_000_000 / 86_400;
    let mut meta = crate::state::read_metadata();
    if meta.last_health_log_day == Some(day) {
        return;
    }
    let instance = crate::state::read_instance_config();
    let symbol = instance
        .observed_profile
        .as_ref()
        .map_or("pending", |p| p.symbol.as_str());
    let runtime = crate::config::runtime();
    let reader = if runtime.observed_ledger == runtime.icp_ledger {
        "icp_legacy"
    } else {
        "icrc3"
    };
    ic_cdk::println!("HEALTH day={} observed_ledger={} reader={} symbol={} mode={:?} liquid_cycles={} observed_cursor={} admission_cursor={} watched_accounts={} global_subscribers={} pricing_initialized={}", day, instance.observed_ledger, reader, symbol, meta.polling_mode, ic_cdk::api::canister_liquid_cycle_balance(), meta.observed_next_block, meta.admission_next_block, crate::state::subscription_count(), crate::state::global_subscribers().len(), crate::pricing::get_pricing().initialized);
    meta.last_health_log_day = Some(day);
    crate::state::write_metadata(meta);
}

pub fn cmc_terminal(reason: &str) {
    ic_cdk::println!("CMC_TERMINAL_FAILURE {}", reason);
}

pub fn cmc_transfer_identity_expired() {
    ic_cdk::println!("CMC_TRANSFER_IDENTITY_EXPIRED");
}

pub fn surplus_transfer_identity_expired() {
    ic_cdk::println!("SURPLUS_TRANSFER_IDENTITY_EXPIRED");
}

pub fn surplus_financial_invariant(reason: &str) {
    ic_cdk::println!("SURPLUS_FINANCIAL_INVARIANT {}", reason);
}
