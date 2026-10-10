use crate::{
    account::account_identifier_bytes,
    clients::{historian, icp_ledger, icrc3, subscriber},
    config, logging,
    memo::{parse_subscription_memo, SubscriptionDeclaration},
    pricing, state,
    subscription::{merge_subscription, Subscription, WatchTarget},
};
use candid::{Nat, Principal};
use icrc_ledger_types::{
    icrc::generic_value::ICRC3Value,
    icrc3::blocks::{BlockWithId, GetBlocksResult},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

const MAX_SPECIFIC_TARGETS_PER_POKE: usize = 256;

#[derive(Default)]
struct MatchState {
    global: bool,
    subs: BTreeMap<WatchTarget, Nat>,
}
struct SharedGlobalOrder {
    existing: BTreeSet<Principal>,
    admitted_at: BTreeMap<Principal, u64>,
    last_live: Option<u64>,
}
enum PreparedAdmission {
    Global(Principal),
    Accounts(Vec<Subscription>),
}

#[derive(Default)]
struct PendingAdmissions {
    globals: BTreeSet<Principal>,
    subscriptions: BTreeMap<(Principal, WatchTarget), Subscription>,
    shared_overlay: BTreeMap<[u8; 32], Subscription>,
}

impl PendingAdmissions {
    fn stage(&mut self, admission: PreparedAdmission, shared: bool) {
        match admission {
            PreparedAdmission::Global(subscriber) => {
                self.globals.insert(subscriber);
            }
            PreparedAdmission::Accounts(subscriptions) => {
                for candidate in subscriptions {
                    let key = (candidate.subscriber, candidate.target);
                    let existing = self.subscriptions.get(&key).cloned().or_else(|| {
                        state::get_target_subscription(candidate.subscriber, candidate.target)
                    });
                    let effective = merge_subscription(existing, candidate);
                    if shared {
                        let governance = state::read_instance_config()
                            .observed_profile
                            .and_then(|p| p.neuron_governance);
                        let account = effective
                            .account(governance)
                            .expect("verified neuron governance");
                        self.shared_overlay.insert(
                            account_identifier_bytes(
                                account.owner,
                                *account.effective_subaccount(),
                            ),
                            effective.clone(),
                        );
                    }
                    self.subscriptions.insert(key, effective);
                }
            }
        }
    }

    fn shared_subscription(&self, account: [u8; 32]) -> Option<Subscription> {
        self.shared_overlay
            .get(&account)
            .cloned()
            .or_else(|| state::get_icp_subscription(account))
    }

    fn publish(self) {
        for subscription in self.subscriptions.into_values() {
            state::put_subscription(subscription);
        }
        for subscriber in self.globals {
            state::put_global_subscriber(subscriber);
        }
    }
}

#[cfg(feature = "debug_api")]
thread_local! {
    static DEBUG_ABORT_AFTER_STAGED_ADMISSION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(feature = "debug_api")]
pub fn debug_set_abort_after_staged_admission(enabled: bool) {
    DEBUG_ABORT_AFTER_STAGED_ADMISSION.with(|value| value.set(enabled));
}

fn debug_abort_after_staged_admission() -> bool {
    #[cfg(feature = "debug_api")]
    {
        return DEBUG_ABORT_AFTER_STAGED_ADMISSION.with(std::cell::Cell::get);
    }
    #[cfg(not(feature = "debug_api"))]
    false
}
#[derive(Clone, Copy)]
enum Stream {
    Admission,
    Observed,
    Shared,
}

fn position(m: &state::Metadata, s: Stream) -> Option<(bool, u64)> {
    match s {
        Stream::Admission => Some((m.admission_bootstrapped, m.admission_next_block)),
        Stream::Observed => Some((m.observed_bootstrapped, m.observed_next_block)),
        Stream::Shared
            if m.admission_bootstrapped != m.observed_bootstrapped
                || (m.admission_bootstrapped
                    && m.admission_next_block != m.observed_next_block) =>
        {
            None
        }
        Stream::Shared => Some((m.admission_bootstrapped, m.admission_next_block)),
    }
}
fn commit(s: Stream, ready: bool, next: u64) {
    state::modify_metadata(|m| match s {
        Stream::Admission => {
            m.admission_bootstrapped = ready;
            m.admission_next_block = next
        }
        Stream::Observed => {
            m.observed_bootstrapped = ready;
            m.observed_next_block = next
        }
        Stream::Shared => {
            m.admission_bootstrapped = ready;
            m.observed_bootstrapped = ready;
            m.admission_next_block = next;
            m.observed_next_block = next
        }
    })
}
fn name(s: Stream) -> &'static str {
    match s {
        Stream::Admission => "admission",
        Stream::Observed => "observed",
        Stream::Shared => "shared",
    }
}
fn u64nat(n: &Nat) -> Result<u64, String> {
    u64::try_from(n.0.clone()).map_err(|_| "Nat exceeds u64".into())
}
fn account_id(b: &[u8]) -> Option<[u8; 32]> {
    b.try_into().ok()
}
fn merge(target: &mut BTreeMap<Principal, MatchState>, source: BTreeMap<Principal, MatchState>) {
    for (p, m) in source {
        let e = target.entry(p).or_default();
        e.global |= m.global;
        for (target_key, amount) in m.subs {
            record_match(e, target_key, amount);
        }
    }
}

fn record_match(state: &mut MatchState, target: WatchTarget, amount: Nat) {
    if let Some(existing) = state.subs.get_mut(&target) {
        if amount > *existing {
            *existing = amount;
        }
    } else if state.subs.len() < MAX_SPECIFIC_TARGETS_PER_POKE {
        state.subs.insert(target, amount);
    }
}

async fn prepare_admission(
    from: &[u8],
    to: &[u8],
    memo: Option<&[u8]>,
    decimals: u8,
) -> Option<PreparedAdmission> {
    let r = config::runtime();
    if account_id(from) != Some(account_identifier_bytes(r.faucet_canister, [0; 32]))
        || account_id(to)
            != Some(account_identifier_bytes(
                ic_cdk::api::canister_self(),
                [0; 32],
            ))
    {
        return None;
    }
    let memo = memo?;
    let Ok(d) = parse_subscription_memo(memo) else {
        return None;
    };
    let neuron_requested = matches!(
        &d,
        SubscriptionDeclaration::Account {
            target: WatchTarget::NeuronNonce(_),
            ..
        } | SubscriptionDeclaration::Range { neuron: true, .. }
    );
    if neuron_requested
        && state::read_instance_config()
            .observed_profile
            .as_ref()
            .and_then(|p| p.neuron_governance)
            .is_none()
    {
        return None;
    }
    let class = match d {
        SubscriptionDeclaration::Global { .. } => pricing::PricingClass::Global,
        SubscriptionDeclaration::Account { .. } => pricing::PricingClass::Account,
        SubscriptionDeclaration::Range { .. } => pricing::PricingClass::Range,
    };
    let required = pricing::current_admission_e8s(class)?;
    match historian::route_is_admitted(
        r.historian_canister,
        ic_cdk::api::canister_self(),
        memo.to_vec(),
        required,
    )
    .await
    {
        Ok(true) => {
            let admission = match d {
                SubscriptionDeclaration::Global { subscriber } => {
                    Some(PreparedAdmission::Global(subscriber))
                }
                x @ SubscriptionDeclaration::Account { .. } => {
                    Subscription::from_declaration(x, decimals)
                        .ok()
                        .map(|subscription| PreparedAdmission::Accounts(vec![subscription]))
                }
                SubscriptionDeclaration::Range {
                    subscriber,
                    neuron,
                    start,
                    end,
                    minimum,
                } => minimum
                    .map(|x| x.to_units(decimals))
                    .transpose()
                    .ok()
                    .map(|minimum| {
                        let minimum_units = minimum.unwrap_or_else(|| Nat::from(0u8));
                        PreparedAdmission::Accounts(
                            (start..=end)
                                .map(|n| Subscription {
                                    subscriber,
                                    target: if neuron {
                                        WatchTarget::NeuronNonce(n)
                                    } else {
                                        WatchTarget::Subaccount(n)
                                    },
                                    minimum_units: minimum_units.clone(),
                                })
                                .collect(),
                        )
                    }),
            };
            logging::historian_recovered();
            return admission;
        }
        Ok(false) => logging::historian_recovered(),
        Err(e) if config::is_reserve_protection(&e) => {}
        Err(e) => logging::historian_failure(&e),
    }
    None
}
fn observe_icrc3(e: &icrc3::LedgerEvent, out: &mut BTreeMap<Principal, MatchState>) {
    if let icrc3::LedgerEvent::Transfer { to, amount, .. } = e {
        if let Some(s) = state::get_subscription(*to) {
            if s.matches(amount) {
                record_match(
                    out.entry(s.subscriber).or_default(),
                    s.target,
                    amount.clone(),
                );
            }
        }
    }
}
fn observe_icp(
    op: &Option<icp_ledger::Operation>,
    pending: &PendingAdmissions,
    out: &mut BTreeMap<Principal, MatchState>,
) {
    if let Some(icp_ledger::Operation::Transfer { to, amount, .. }) = op {
        if let Some(id) = account_id(to) {
            if let Some(s) = pending.shared_subscription(id) {
                if s.matches(&Nat::from(amount.e8s)) {
                    record_match(
                        out.entry(s.subscriber).or_default(),
                        s.target,
                        Nat::from(amount.e8s),
                    );
                }
            }
        }
    }
}
fn coalesce(ranges: &[(u64, u64)], start: u64) -> u64 {
    let mut end = start;
    loop {
        let n = ranges
            .iter()
            .filter(|(s, e)| *s <= end && end < *e)
            .fold(end, |x, (_, e)| x.max(*e));
        if n == end {
            return end;
        }
        end = n
    }
}
fn validated_archive_prefix(
    ranges: impl IntoIterator<Item = (u64, u64)>,
    start: u64,
    request_end: u64,
) -> Result<u64, String> {
    let mut validated = Vec::new();
    for (range_start, length) in ranges {
        let range_end = range_start.checked_add(length).ok_or_else(|| {
            format!("archive interval overflow start={range_start} length={length}")
        })?;
        let clipped_start = range_start.max(start);
        let clipped_end = range_end.min(request_end);
        if clipped_start < clipped_end {
            validated.push((clipped_start, clipped_end));
        }
    }
    Ok(coalesce(&validated, start))
}

fn request_end(start: u64, length: u64, boundary: u64) -> Result<u64, String> {
    start
        .checked_add(length)
        .map(|end| end.min(boundary))
        .ok_or_else(|| format!("request interval overflow start={start} length={length}"))
}

fn page_length(start: u64, remaining: Option<u64>) -> u64 {
    let representable = u64::MAX - start;
    remaining
        .unwrap_or(config::LEDGER_PAGE_SIZE)
        .min(config::LEDGER_PAGE_SIZE)
        .min(representable)
}

fn icrc_archived(r: &GetBlocksResult, start: u64, request_end: u64) -> Result<u64, String> {
    if r.archived_blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive groups".into());
    }
    let mut v = Vec::new();
    for a in &r.archived_blocks {
        let count = v
            .len()
            .checked_add(a.args.len())
            .ok_or("archive range count overflow")?;
        if count > config::LEDGER_PAGE_SIZE as usize {
            return Err("excessive archive ranges".into());
        }
        for q in &a.args {
            let (s, l) = q.as_start_and_length()?;
            v.push((s, l))
        }
    }
    validated_archive_prefix(v, start, request_end)
}

fn icrc_live_blocks(
    mut blocks: Vec<BlockWithId>,
    start: u64,
    request_end: u64,
) -> Result<Vec<(u64, ICRC3Value)>, String> {
    blocks.sort_by(|a, b| a.id.cmp(&b.id));
    let mut unique: Vec<(u64, ICRC3Value)> = Vec::new();
    for block in blocks {
        let id = u64nat(&block.id)?;
        if id < start || id >= request_end {
            continue;
        }
        if let Some((previous_id, previous_block)) = unique.last() {
            if *previous_id == id {
                if *previous_block != block.block {
                    return Err(format!("conflicting duplicate block id={id}"));
                }
                continue;
            }
        }
        unique.push((id, block.block));
    }
    Ok(unique)
}

async fn icrc_page(
    r: GetBlocksResult,
    boundary: u64,
    requested_start: u64,
    requested_length: u64,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<(u64, bool), String> {
    if r.blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive live blocks".into());
    }
    let original = position(&state::read_metadata(), Stream::Observed)
        .unwrap()
        .1;
    if original != requested_start {
        return Err(format!(
            "observed request cursor changed expected={requested_start} actual={original}"
        ));
    }
    let requested_end = request_end(requested_start, requested_length, boundary)?;
    let mut at = original;
    let archived = icrc_archived(&r, at, requested_end)?;
    if archived > at {
        logging::history_gap("observed", at, archived);
        at = archived;
        commit(Stream::Observed, true, at)
    }
    let blocks = icrc_live_blocks(r.blocks, at, requested_end)?;
    let mut local = BTreeMap::new();
    let mut live = false;
    for (id, block) in blocks {
        if id != at {
            return Err(format!("unexplained hole cursor={at} block={id}"));
        }
        let e = icrc3::decode_block(&block)?;
        observe_icrc3(&e, &mut local);
        live = true;
        at = at.checked_add(1).ok_or("observed cursor overflow")?;
    }
    if at == original && at < boundary {
        return Err("no live progress or archive evidence".into());
    }
    if live {
        commit(Stream::Observed, true, at);
        merge(out, local)
    }
    Ok((at, live))
}
async fn scan_icrc(
    ledger: Principal,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<bool, String> {
    let (ready, mut at) = position(&state::read_metadata(), Stream::Observed).unwrap();
    if !ready {
        let tip = u64nat(&icrc3::get_blocks(ledger, 0, 0).await?.log_length)?;
        commit(Stream::Observed, true, tip);
        return Ok(false);
    }
    let first_length = page_length(at, None);
    let first = icrc3::get_blocks(ledger, at, first_length).await?;
    let boundary = u64nat(&first.log_length)?;
    let mut next = Some((first, first_length));
    let mut activity = false;
    while at < boundary {
        let (r, requested_length) = match next.take() {
            Some(value) => value,
            None => {
                let length = page_length(at, Some(boundary - at));
                (icrc3::get_blocks(ledger, at, length).await?, length)
            }
        };
        let (n, live) = icrc_page(r, boundary, at, requested_length, out).await?;
        at = n;
        activity |= live
    }
    Ok(activity)
}

fn legacy_archived(
    r: &[icp_ledger::ArchivedBlocksRange],
    start: u64,
    request_end: u64,
) -> Result<u64, String> {
    if r.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive ranges".into());
    }
    validated_archive_prefix(
        r.iter().map(|range| (range.start, range.length)),
        start,
        request_end,
    )
}
async fn legacy_page(
    r: icp_ledger::QueryBlocksResponse,
    s: Stream,
    boundary: u64,
    requested: Range<u64>,
    decimals: u8,
    out: &mut BTreeMap<Principal, MatchState>,
    shared_globals: Option<&mut SharedGlobalOrder>,
) -> Result<(u64, bool), String> {
    if r.blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive live blocks".into());
    }
    let original = position(&state::read_metadata(), s)
        .ok_or("shared cursor invariant violated")?
        .1;
    if original != requested.start {
        return Err(format!(
            "{} request cursor changed expected={} actual={original}",
            name(s),
            requested.start,
        ));
    }
    let mut at = original;
    let archived = legacy_archived(&r.archived_blocks, at, requested.end)?;
    if archived > at {
        logging::history_gap(name(s), at, archived);
        at = archived;
        commit(s, true, at)
    }
    if !r.blocks.is_empty() && r.first_block_index > at {
        return Err(format!(
            "unexplained hole cursor={at} first={}",
            r.first_block_index
        ));
    }
    let mut local = BTreeMap::new();
    let mut pending = PendingAdmissions::default();
    let mut page_global_admissions = BTreeMap::new();
    let mut page_last_live = None;
    let mut live = false;
    for (offset, b) in r.blocks.into_iter().enumerate() {
        let id = r
            .first_block_index
            .checked_add(offset as u64)
            .ok_or("legacy block index overflow")?;
        if id < at {
            continue;
        }
        if id >= requested.end {
            break;
        }
        if id != at {
            return Err(format!("non-contiguous block expected={at} got={id}"));
        }
        if let Some(icp_ledger::Operation::Transfer { from, to, .. }) = &b.transaction.operation {
            if let Some(admission) =
                prepare_admission(from, to, b.transaction.icrc1_memo.as_deref(), decimals).await
            {
                if let PreparedAdmission::Global(subscriber) = &admission {
                    if let Some(order) = shared_globals.as_deref() {
                        if !order.existing.contains(subscriber) {
                            page_global_admissions.entry(*subscriber).or_insert(id);
                        }
                    }
                }
                pending.stage(admission, matches!(s, Stream::Shared));
                if debug_abort_after_staged_admission() {
                    return Err("debug abort after staged legacy admission".into());
                }
            }
        }
        if matches!(s, Stream::Shared) {
            observe_icp(&b.transaction.operation, &pending, &mut local);
            page_last_live = Some(id);
        }
        live = true;
        at = at.checked_add(1).ok_or("legacy cursor overflow")?;
    }
    if at == original && at < boundary {
        return Err("no live progress or archive evidence".into());
    }
    if live {
        // No inter-canister call may separate publication of admissions verified
        // in this page from the cursor that makes those admissions authoritative.
        pending.publish();
        commit(s, true, at);
        if let Some(order) = shared_globals {
            for (subscriber, admitted_at) in page_global_admissions {
                order.admitted_at.entry(subscriber).or_insert(admitted_at);
            }
            if let Some(last_live) = page_last_live {
                order.last_live = Some(last_live);
            }
        }
        merge(out, local)
    }
    Ok((at, live))
}
async fn scan_legacy(
    s: Stream,
    decimals: u8,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<bool, String> {
    let r = config::runtime();
    let Some((ready, mut at)) = position(&state::read_metadata(), s) else {
        logging::shared_cursor_invariant("bootstrap flags or cursors differ");
        return Err("shared cursor invariant violated".into());
    };
    if matches!(s, Stream::Shared) {
        logging::shared_cursor_recovered();
    }
    if !ready {
        let tip = icp_ledger::query_blocks(r.icp_ledger, 0, 0)
            .await?
            .chain_length;
        commit(s, true, tip);
        return Ok(false);
    }
    // Snapshot once per shared scan. Admissions take effect only for later blocks,
    // without walking the complete global registry for every transaction.
    let mut shared_globals = matches!(s, Stream::Shared).then(|| SharedGlobalOrder {
        existing: state::global_subscribers().into_iter().collect(),
        admitted_at: BTreeMap::new(),
        last_live: None,
    });
    let first_length = page_length(at, None);
    let first = icp_ledger::query_blocks(r.icp_ledger, at, first_length).await?;
    let boundary = first.chain_length;
    let mut next = Some((first, first_length));
    let mut activity = false;
    while at < boundary {
        let (response, requested_length) = match next.take() {
            Some(value) => value,
            None => {
                let length = page_length(at, Some(boundary - at));
                (
                    icp_ledger::query_blocks(r.icp_ledger, at, length).await?,
                    length,
                )
            }
        };
        let (n, live) = legacy_page(
            response,
            s,
            boundary,
            at..request_end(at, requested_length, boundary)?,
            decimals,
            out,
            shared_globals.as_mut(),
        )
        .await?;
        at = n;
        activity |= live
    }
    if let Some(order) = shared_globals {
        if let Some(last_live) = order.last_live {
            for subscriber in order.existing {
                out.entry(subscriber).or_default().global = true;
            }
            for (subscriber, admitted_at) in order.admitted_at {
                if last_live > admitted_at {
                    out.entry(subscriber).or_default().global = true;
                }
            }
        }
    }
    Ok(activity)
}
fn failure(s: Stream, e: &str) {
    if config::is_reserve_protection(e) || e == "shared cursor invariant violated" {
        return;
    }
    match s {
        Stream::Observed => logging::observed_ledger_failure(e),
        Stream::Admission => logging::icp_admission_ledger_failure(e),
        Stream::Shared => logging::shared_ledger_failure(e),
    }
}

pub async fn run_poll() {
    let r = config::runtime();
    let shared = r.observed_ledger == r.icp_ledger;
    if !state::read_metadata().admission_bootstrapped {
        let stream = if shared {
            Stream::Shared
        } else {
            Stream::Admission
        };
        if let Err(e) = scan_legacy(stream, 0, &mut BTreeMap::new()).await {
            failure(stream, &e);
            if shared {
                return;
            }
        }
    }
    let profile = match crate::instance::ensure_observed_profile().await {
        Ok(p) => p,
        Err(e) => {
            match e {
                crate::instance::ProfileError::ReserveProtection => {}
                crate::instance::ProfileError::ObservedLedger(message) => {
                    logging::observed_ledger_failure(&message)
                }
                crate::instance::ProfileError::SnsRoot(message) => {
                    logging::sns_root_failure(&message)
                }
                crate::instance::ProfileError::InvalidSns(message) => {
                    logging::sns_relation_invalid(&message)
                }
            }
            return;
        }
    };
    let mut out = BTreeMap::new();
    if shared {
        match scan_legacy(Stream::Shared, profile.decimals, &mut out).await {
            Ok(_) => logging::ledger_recovered_all(),
            Err(e) => {
                failure(Stream::Shared, &e);
                return;
            }
        }
    } else {
        let globals = state::global_subscribers();
        let activity = match scan_icrc(r.observed_ledger, &mut out).await {
            Ok(a) => {
                logging::observed_ledger_recovered();
                a
            }
            Err(e) => {
                failure(Stream::Observed, &e);
                out.clear();
                false
            }
        };
        match scan_legacy(Stream::Admission, profile.decimals, &mut BTreeMap::new()).await {
            Ok(_) => logging::icp_admission_ledger_recovered(),
            Err(e) => failure(Stream::Admission, &e),
        }
        if activity {
            for p in globals {
                out.entry(p).or_default().global = true
            }
        }
    }
    for (p, m) in out {
        let hint = if m.subs.is_empty() {
            vec![]
        } else {
            m.subs
                .into_iter()
                .map(|(target, max_amount)| subscriber::PokeMatch { target, max_amount })
                .collect()
        };
        let _ = subscriber::poke(p, hint);
    }
}

#[cfg(test)]
mod target_match_tests {
    use super::*;
    use icrc_ledger_types::icrc3::{
        archive::QueryArchiveFn,
        blocks::{ArchivedBlocks, BlockWithId, GetBlocksRequest},
    };
    use std::str::FromStr;

    #[test]
    fn maxima_order_and_arbitrary_precision_are_preserved() {
        let mut state = MatchState::default();
        record_match(&mut state, WatchTarget::NeuronNonce(1), Nat::from(9u8));
        record_match(&mut state, WatchTarget::Subaccount(7), Nat::from(100u16));
        record_match(&mut state, WatchTarget::Subaccount(7), Nat::from(500u16));
        record_match(&mut state, WatchTarget::Subaccount(7), Nat::from(300u16));
        let huge = Nat::from_str("18446744073709551616000000000000000000").unwrap();
        record_match(&mut state, WatchTarget::Subaccount(8), huge.clone());
        assert_eq!(state.subs[&WatchTarget::Subaccount(7)], Nat::from(500u16));
        assert_eq!(state.subs[&WatchTarget::Subaccount(8)], huge);
        assert_eq!(
            state.subs.keys().copied().collect::<Vec<_>>(),
            vec![
                WatchTarget::Subaccount(7),
                WatchTarget::Subaccount(8),
                WatchTarget::NeuronNonce(1)
            ]
        );
    }

    #[test]
    fn specific_target_bound_keeps_updates_for_retained_targets() {
        let mut state = MatchState::default();
        for n in 0..300 {
            record_match(&mut state, WatchTarget::Subaccount(n), Nat::from(n));
        }
        assert_eq!(state.subs.len(), MAX_SPECIFIC_TARGETS_PER_POKE);
        record_match(&mut state, WatchTarget::Subaccount(7), Nat::from(999u16));
        assert_eq!(state.subs[&WatchTarget::Subaccount(7)], Nat::from(999u16));
    }

    #[test]
    fn archive_interval_coverage_is_bounded_contiguous_and_order_independent() {
        struct Case {
            name: &'static str,
            ranges: Vec<(u64, u64)>,
            start: u64,
            request_end: u64,
            expected: u64,
        }
        let cases = [
            Case {
                name: "complete",
                ranges: vec![(10, 10)],
                start: 10,
                request_end: 20,
                expected: 20,
            },
            Case {
                name: "partial",
                ranges: vec![(10, 4)],
                start: 10,
                request_end: 20,
                expected: 14,
            },
            Case {
                name: "out of order adjacent overlapping repeated",
                ranges: vec![(15, 5), (10, 3), (13, 4), (10, 3), (12, 6)],
                start: 10,
                request_end: 20,
                expected: 20,
            },
            Case {
                name: "genuine gap",
                ranges: vec![(14, 6), (10, 3)],
                start: 10,
                request_end: 20,
                expected: 13,
            },
            Case {
                name: "zero length",
                ranges: vec![(10, 0), (9, 0)],
                start: 10,
                request_end: 20,
                expected: 10,
            },
            Case {
                name: "over broad evidence is clipped",
                ranges: vec![(0, 100)],
                start: 10,
                request_end: 20,
                expected: 20,
            },
            Case {
                name: "boundary equal to cursor",
                ranges: vec![(0, 100)],
                start: 20,
                request_end: 20,
                expected: 20,
            },
            Case {
                name: "near u64 boundary",
                ranges: vec![(u64::MAX - 2, 2)],
                start: u64::MAX - 2,
                request_end: u64::MAX,
                expected: u64::MAX,
            },
        ];
        for case in cases {
            assert_eq!(
                validated_archive_prefix(case.ranges, case.start, case.request_end).unwrap(),
                case.expected,
                "{}",
                case.name
            );
        }
        for ranges in [vec![(u64::MAX, 1)], vec![(u64::MAX - 1, 2)]] {
            assert!(validated_archive_prefix(ranges, 0, u64::MAX).is_err());
        }
        assert_eq!(request_end(u64::MAX - 1, 1, u64::MAX).unwrap(), u64::MAX);
        assert!(request_end(u64::MAX, 1, u64::MAX).is_err());
        assert_eq!(page_length(u64::MAX, None), 0);
    }

    #[test]
    fn legacy_archive_wrapper_rejects_excessive_or_overflowing_ranges() {
        let excessive = vec![
            icp_ledger::ArchivedBlocksRange {
                start: 0,
                length: 0,
            };
            config::LEDGER_PAGE_SIZE as usize + 1
        ];
        assert_eq!(
            legacy_archived(&excessive, 0, 10).unwrap_err(),
            "excessive archive ranges"
        );
        assert!(legacy_archived(
            &[icp_ledger::ArchivedBlocksRange {
                start: u64::MAX,
                length: 1,
            }],
            0,
            10,
        )
        .is_err());
    }

    fn icrc_result(groups: Vec<Vec<GetBlocksRequest>>) -> GetBlocksResult {
        GetBlocksResult {
            log_length: Nat::from(0u8),
            blocks: vec![],
            archived_blocks: groups
                .into_iter()
                .map(|args| ArchivedBlocks {
                    args,
                    callback: QueryArchiveFn::new(Principal::anonymous(), "unused"),
                })
                .collect(),
        }
    }

    #[test]
    fn icrc_archive_wrapper_rejects_counts_unrepresentable_values_and_overflow() {
        let request = |start: Nat, length: Nat| GetBlocksRequest { start, length };
        let excessive_groups =
            icrc_result((0..=config::LEDGER_PAGE_SIZE).map(|_| Vec::new()).collect());
        assert_eq!(
            icrc_archived(&excessive_groups, 0, 10).unwrap_err(),
            "excessive archive groups"
        );
        let excessive_ranges = icrc_result(vec![(0..=config::LEDGER_PAGE_SIZE)
            .map(|_| request(0u8.into(), 0u8.into()))
            .collect()]);
        assert_eq!(
            icrc_archived(&excessive_ranges, 0, 10).unwrap_err(),
            "excessive archive ranges"
        );
        let above_u64 = Nat::from_str("18446744073709551616").unwrap();
        for malformed in [
            icrc_result(vec![vec![request(above_u64.clone(), 1u8.into())]]),
            icrc_result(vec![vec![request(1u8.into(), above_u64)]]),
            icrc_result(vec![vec![request(u64::MAX.into(), 1u8.into())]]),
        ] {
            assert!(icrc_archived(&malformed, 0, 10).is_err());
        }
    }

    fn block(id: u64, value: u64) -> BlockWithId {
        BlockWithId {
            id: id.into(),
            block: ICRC3Value::Nat(value.into()),
        }
    }

    #[test]
    fn icrc_live_blocks_sort_and_deduplicate_without_selecting_conflicts() {
        assert_eq!(
            icrc_live_blocks(vec![block(11, 2), block(10, 1), block(10, 1)], 10, 12).unwrap(),
            vec![
                (10, ICRC3Value::Nat(1u8.into())),
                (11, ICRC3Value::Nat(2u8.into()))
            ]
        );
        assert_eq!(
            icrc_live_blocks(vec![block(10, 1), block(10, 2)], 10, 11).unwrap_err(),
            "conflicting duplicate block id=10"
        );
    }
}
