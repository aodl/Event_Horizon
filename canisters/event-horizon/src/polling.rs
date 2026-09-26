use crate::{
    account::account_identifier_bytes,
    clients::{historian, icp_ledger, icrc3, subscriber},
    config, logging,
    memo::{parse_subscription_memo, SubscriptionDeclaration},
    pricing, state,
    subscription::Subscription,
};
use candid::{Nat, Principal};
use icrc_ledger_types::icrc3::blocks::GetBlocksResult;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct MatchState {
    global: bool,
    subs: BTreeSet<u8>,
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
        e.subs.extend(m.subs)
    }
}

async fn admit(from: &[u8], to: &[u8], memo: Option<&[u8]>, decimals: u8) {
    let r = config::runtime();
    if account_id(from) != Some(account_identifier_bytes(r.faucet_canister, [0; 32]))
        || account_id(to)
            != Some(account_identifier_bytes(
                ic_cdk::api::canister_self(),
                [0; 32],
            ))
    {
        return;
    }
    let Some(memo) = memo else { return };
    let Ok(d) = parse_subscription_memo(memo) else {
        return;
    };
    let class = match d {
        SubscriptionDeclaration::Global { .. } => pricing::PricingClass::Global,
        SubscriptionDeclaration::Account { .. } => pricing::PricingClass::Account,
        SubscriptionDeclaration::Range { .. } => pricing::PricingClass::Range,
    };
    let Some(required) = pricing::current_admission_e8s(class) else {
        return;
    };
    match historian::route_is_admitted(
        r.historian_canister,
        ic_cdk::api::canister_self(),
        memo.to_vec(),
        required,
    )
    .await
    {
        Ok(true) => {
            match d {
                SubscriptionDeclaration::Global { subscriber } => {
                    state::put_global_subscriber(subscriber)
                }
                x @ SubscriptionDeclaration::Account { .. } => {
                    if let Ok(s) = Subscription::from_declaration(x, decimals) {
                        state::put_subscription(s)
                    }
                }
                SubscriptionDeclaration::Range {
                    subscriber,
                    start_subaccount,
                    end_subaccount,
                    minimum,
                } => {
                    let Ok(v) = minimum.map(|x| x.to_units(decimals)).transpose() else {
                        return;
                    };
                    let v = v.unwrap_or_else(|| Nat::from(0u8));
                    for n in start_subaccount..=end_subaccount {
                        state::put_subscription(Subscription {
                            subscriber,
                            numbered_subaccount: n,
                            minimum_units: v.clone(),
                        })
                    }
                }
            }
            logging::historian_recovered()
        }
        Ok(false) => logging::historian_recovered(),
        Err(e) if config::is_reserve_protection(&e) => {}
        Err(e) => logging::historian_failure(&e),
    }
}
fn observe_icrc3(e: &icrc3::LedgerEvent, out: &mut BTreeMap<Principal, MatchState>) {
    if let icrc3::LedgerEvent::Transfer { to, amount, .. } = e {
        if let Some(s) = state::get_subscription(*to) {
            if s.matches(amount) {
                out.entry(s.subscriber)
                    .or_default()
                    .subs
                    .insert(s.numbered_subaccount);
            }
        }
    }
}
fn observe_icp(op: &Option<icp_ledger::Operation>, out: &mut BTreeMap<Principal, MatchState>) {
    if let Some(icp_ledger::Operation::Transfer { to, amount, .. }) = op {
        if let Some(id) = account_id(to) {
            if let Some(s) = state::get_icp_subscription(id) {
                if s.matches(&Nat::from(amount.e8s)) {
                    out.entry(s.subscriber)
                        .or_default()
                        .subs
                        .insert(s.numbered_subaccount);
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
fn icrc_archived(r: &GetBlocksResult, start: u64, boundary: u64) -> Result<u64, String> {
    let mut v = Vec::new();
    for a in &r.archived_blocks {
        if v.len().saturating_add(a.args.len()) > config::LEDGER_PAGE_SIZE as usize {
            return Err("excessive archive ranges".into());
        }
        for q in &a.args {
            let (s, l) = q.as_start_and_length()?;
            v.push((s, s.saturating_add(l).min(boundary)))
        }
    }
    Ok(coalesce(&v, start))
}

async fn icrc_page(
    r: GetBlocksResult,
    boundary: u64,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<(u64, bool), String> {
    let original = position(&state::read_metadata(), Stream::Observed)
        .unwrap()
        .1;
    let mut at = original;
    let archived = icrc_archived(&r, at, boundary)?;
    if archived > at {
        logging::history_gap("observed", at, archived);
        at = archived;
        commit(Stream::Observed, true, at)
    }
    if r.blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive live blocks".into());
    }
    let mut blocks = r.blocks;
    blocks.sort_by(|a, b| a.id.cmp(&b.id));
    let mut local = BTreeMap::new();
    let mut live = false;
    for b in blocks {
        let id = u64nat(&b.id)?;
        if id < at {
            continue;
        }
        if id >= boundary {
            break;
        }
        if id != at {
            return Err(format!("unexplained hole cursor={at} block={id}"));
        }
        let e = icrc3::decode_block(&b.block)?;
        observe_icrc3(&e, &mut local);
        live = true;
        at += 1
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
    let first = icrc3::get_blocks(ledger, at, config::LEDGER_PAGE_SIZE).await?;
    let boundary = u64nat(&first.log_length)?;
    let mut next = Some(first);
    let mut activity = false;
    while at < boundary {
        let r = match next.take() {
            Some(r) => r,
            None => {
                icrc3::get_blocks(ledger, at, config::LEDGER_PAGE_SIZE.min(boundary - at)).await?
            }
        };
        let (n, live) = icrc_page(r, boundary, out).await?;
        at = n;
        activity |= live
    }
    Ok(activity)
}

fn legacy_archived(
    r: &[icp_ledger::ArchivedBlocksRange],
    start: u64,
    boundary: u64,
) -> Result<u64, String> {
    if r.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive ranges".into());
    }
    Ok(coalesce(
        &r.iter()
            .map(|x| (x.start, x.start.saturating_add(x.length).min(boundary)))
            .collect::<Vec<_>>(),
        start,
    ))
}
async fn legacy_page(
    r: icp_ledger::QueryBlocksResponse,
    s: Stream,
    boundary: u64,
    decimals: u8,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<(u64, bool), String> {
    if r.blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive live blocks".into());
    }
    let original = position(&state::read_metadata(), s)
        .ok_or("shared cursor invariant violated")?
        .1;
    let mut at = original;
    let archived = legacy_archived(&r.archived_blocks, at, boundary)?;
    if archived > at {
        logging::history_gap(name(s), at, archived);
        at = archived;
        commit(s, true, at)
    }
    if r.first_block_index > at {
        return Err(format!(
            "unexplained hole cursor={at} first={}",
            r.first_block_index
        ));
    }
    let mut local = BTreeMap::new();
    let mut live = false;
    for (offset, b) in r.blocks.into_iter().enumerate() {
        let id = r.first_block_index.saturating_add(offset as u64);
        if id < at {
            continue;
        }
        if id >= boundary {
            break;
        }
        if id != at {
            return Err(format!("non-contiguous block expected={at} got={id}"));
        }
        if let Some(icp_ledger::Operation::Transfer { from, to, .. }) = &b.transaction.operation {
            admit(from, to, b.transaction.icrc1_memo.as_deref(), decimals).await
        }
        if matches!(s, Stream::Shared) {
            observe_icp(&b.transaction.operation, &mut local);
            for p in state::global_subscribers() {
                local.entry(p).or_default().global = true
            }
        }
        live = true;
        at += 1
    }
    if at == original && at < boundary {
        return Err("no live progress or archive evidence".into());
    }
    if live {
        commit(s, true, at);
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
    let first = icp_ledger::query_blocks(r.icp_ledger, at, config::LEDGER_PAGE_SIZE).await?;
    let boundary = first.chain_length;
    let mut next = Some(first);
    let mut activity = false;
    while at < boundary {
        let response = match next.take() {
            Some(x) => x,
            None => {
                icp_ledger::query_blocks(
                    r.icp_ledger,
                    at,
                    config::LEDGER_PAGE_SIZE.min(boundary - at),
                )
                .await?
            }
        };
        let (n, live) = legacy_page(response, s, boundary, decimals, out).await?;
        at = n;
        activity |= live
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
            if !config::is_reserve_protection(&e) {
                logging::observed_ledger_failure(&e)
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
            m.subs.into_iter().collect()
        };
        let _ = subscriber::poke(p, hint);
    }
}
