use crate::{
    clients::{historian, icrc3, subscriber},
    config, logging,
    memo::{parse_subscription_memo, SubscriptionDeclaration},
    pricing, state,
    subscription::Subscription,
};
use candid::{Nat, Principal};
use icrc_ledger_types::{icrc1::account::Account, icrc3::blocks::GetBlocksResult};
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
}
fn pos(m: &state::Metadata, s: Stream) -> (bool, u64) {
    match s {
        Stream::Admission => (m.admission_bootstrapped, m.admission_next_block),
        Stream::Observed => (m.observed_bootstrapped, m.observed_next_block),
    }
}
fn set(s: Stream, ready: bool, next: u64) {
    state::modify_metadata(|m| match s {
        Stream::Admission => {
            m.admission_bootstrapped = ready;
            m.admission_next_block = next
        }
        Stream::Observed => {
            m.observed_bootstrapped = ready;
            m.observed_next_block = next
        }
    })
}
fn u64nat(n: &Nat) -> Result<u64, String> {
    u64::try_from(n.0.clone()).map_err(|_| "Nat exceeds u64".into())
}
fn archived_prefix(r: &GetBlocksResult, start: u64, boundary: u64) -> Result<u64, String> {
    let mut ranges = Vec::new();
    for a in &r.archived_blocks {
        if a.args.len() > 256 {
            return Err("excessive archive ranges".into());
        }
        for q in &a.args {
            let (s, l) = q.as_start_and_length()?;
            ranges.push((s, s.saturating_add(l).min(boundary)));
        }
    }
    let mut end = start;
    loop {
        let n = ranges
            .iter()
            .filter(|(s, e)| *s <= end && end < *e)
            .fold(end, |x, (_, e)| x.max(*e));
        if n == end {
            return Ok(end);
        }
        end = n
    }
}
async fn admit(from: &Account, to: &Account, memo: Option<&[u8]>, decimals: u8) {
    let r = config::runtime();
    if *from != Account::from(r.faucet_canister)
        || *to != Account::from(ic_cdk::api::canister_self())
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
        Err(e) if e == "reserve_protection" => {}
        Err(e) => logging::historian_failure(&e),
    }
}
fn observe(e: &icrc3::LedgerEvent, out: &mut BTreeMap<Principal, MatchState>) {
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
async fn page(
    r: GetBlocksResult,
    s: Stream,
    boundary: u64,
    admission: bool,
    observed: bool,
    decimals: u8,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<u64, String> {
    let original = pos(&state::read_metadata(), s).1;
    let mut at = original;
    let archived = archived_prefix(&r, at, boundary)?;
    if archived > at {
        logging::history_gap(
            match s {
                Stream::Admission => "admission",
                Stream::Observed => "observed",
            },
            at,
            archived,
        );
        at = archived;
        set(s, true, at)
    }
    let mut blocks = r.blocks;
    blocks.sort_by(|a, b| a.id.cmp(&b.id));
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
        if admission {
            if let icrc3::LedgerEvent::Transfer { from, to, memo, .. } = &e {
                admit(from, to, memo.as_deref(), decimals).await
            }
        }
        if observed {
            observe(&e, out)
        }
        at += 1;
        set(s, true, at)
    }
    if at == original && at < boundary {
        return Err("no live progress or archive evidence".into());
    }
    Ok(at)
}
async fn scan(
    ledger: Principal,
    s: Stream,
    admission: bool,
    observed: bool,
    decimals: u8,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<bool, String> {
    let (ready, mut at) = pos(&state::read_metadata(), s);
    if !ready {
        let tip = u64nat(&icrc3::get_blocks(ledger, 0, 0).await?.log_length)?;
        set(s, true, tip);
        return Ok(false);
    }
    let first = icrc3::get_blocks(ledger, at, config::LEDGER_PAGE_SIZE).await?;
    let boundary = u64nat(&first.log_length)?;
    let mut next = Some(first);
    let mut activity = false;
    while at < boundary {
        let r = match next.take() {
            Some(x) => x,
            None => {
                icrc3::get_blocks(ledger, at, config::LEDGER_PAGE_SIZE.min(boundary - at)).await?
            }
        };
        let n = page(r, s, boundary, admission, observed, decimals, out).await?;
        activity |= n > at;
        at = n
    }
    Ok(activity)
}
pub async fn run_poll() {
    let r = config::runtime();
    let profile = match crate::instance::ensure_observed_profile().await {
        Ok(x) => x,
        Err(e) => {
            logging::observed_ledger_failure(&e);
            return;
        }
    };
    let mut out = BTreeMap::new();
    let active;
    if r.observed_ledger == r.icp_ledger {
        let m = state::read_metadata();
        if !m.admission_bootstrapped || !m.observed_bootstrapped {
            match icrc3::get_blocks(r.icp_ledger, 0, 0)
                .await
                .and_then(|x| u64nat(&x.log_length))
            {
                Ok(t) => {
                    set(Stream::Admission, true, t);
                    set(Stream::Observed, true, t)
                }
                Err(e) => logging::icp_admission_ledger_failure(&e),
            }
            return;
        }
        let start = m.admission_next_block.min(m.observed_next_block);
        set(Stream::Admission, true, start);
        set(Stream::Observed, true, start);
        match scan(
            r.icp_ledger,
            Stream::Admission,
            true,
            true,
            profile.decimals,
            &mut out,
        )
        .await
        {
            Ok(a) => {
                active = a;
                let end = state::read_metadata().admission_next_block;
                set(Stream::Observed, true, end);
                logging::ledger_recovered_all()
            }
            Err(e) => {
                logging::icp_admission_ledger_failure(&e);
                logging::observed_ledger_failure(&e);
                return;
            }
        }
    } else {
        if let Err(e) = scan(
            r.icp_ledger,
            Stream::Admission,
            true,
            false,
            profile.decimals,
            &mut out,
        )
        .await
        {
            logging::icp_admission_ledger_failure(&e)
        } else {
            logging::icp_admission_ledger_recovered()
        }
        match scan(
            r.observed_ledger,
            Stream::Observed,
            false,
            true,
            profile.decimals,
            &mut out,
        )
        .await
        {
            Ok(a) => {
                active = a;
                logging::observed_ledger_recovered()
            }
            Err(e) => {
                logging::observed_ledger_failure(&e);
                return;
            }
        }
    }
    if active {
        for p in state::global_subscribers() {
            out.entry(p).or_default().global = true
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
