//! Intentionally sparse logging. Public canister logs have a small rolling buffer.

use std::cell::Cell;

thread_local! {
    static OBSERVED_LEDGER_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static ICP_LEDGER_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static HISTORIAN_UNAVAILABLE: Cell<bool> = const { Cell::new(false) };
    static IN_RESERVE_PROTECTION: Cell<bool> = const { Cell::new(false) };
    static OBSERVED_HISTORY_GAP_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ADMISSION_HISTORY_GAP_ACTIVE: Cell<bool> = const { Cell::new(false) };
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

pub fn reserve_mode(active: bool, balance: u128) {
    IN_RESERVE_PROTECTION.with(|flag| {
        let previous = flag.replace(active);
        if active && !previous {
            ic_cdk::println!("ENTER_RESERVE_PROTECTION cycles={}", balance);
        } else if !active && previous {
            ic_cdk::println!("EXIT_RESERVE_PROTECTION cycles={}", balance);
        }
    });
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
