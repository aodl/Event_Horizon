use crate::{
    account::account_identifier_bytes,
    clients::{historian, icp_ledger, icrc3, subscriber},
    config, logging,
    memo::{resolve_subscription_memo, SubscriptionDeclaration},
    pricing, state,
    subscription::{SubjectSubscription, Subscription, WatchTarget},
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct TargetMatch {
    first_block_index: u64,
    max_amount: Nat,
}

#[derive(Default, Clone)]
struct MatchState {
    global: bool,
    subs: BTreeMap<(Principal, WatchTarget), TargetMatch>,
}

fn dispatch_matches(
    matches: BTreeMap<Principal, MatchState>,
    mut poke: impl FnMut(Principal, subscriber::PokeNotification) -> bool,
) {
    let (globals, specifics): (Vec<_>, Vec<_>) = matches.into_iter().partition(|(_, s)| s.global);
    for (principal, state) in globals.into_iter().chain(specifics) {
        let notification = subscriber::PokeNotification {
            global_activity: state.global,
            matches: state
                .subs
                .into_iter()
                .map(|((subject, target), m)| subscriber::PokeMatch {
                    subject,
                    target,
                    first_block_index: m.first_block_index,
                    max_amount: m.max_amount,
                })
                .collect(),
        };
        if !notification.global_activity && notification.matches.is_empty() {
            continue;
        }
        let _ = poke(principal, notification);
    }
}
struct SharedGlobalOrder {
    existing: Option<BTreeSet<Principal>>,
    admitted_at: BTreeMap<Principal, u64>,
    last_live: Option<u64>,
}
enum PreparedAdmission {
    Global(Principal),
    Accounts(Vec<SubjectSubscription>),
}

/// Page-local cache generation. Removing an entry invalidates both its watcher
/// list and its amount summary; a new relation never inherits earlier activity.
struct CachedInterests {
    subscriptions: Vec<SubjectSubscription>,
    record_high: Option<Nat>,
    #[cfg(test)]
    walked: usize,
}
impl CachedInterests {
    fn new(subscriptions: Vec<SubjectSubscription>) -> Self {
        Self {
            subscriptions,
            record_high: None,
            #[cfg(test)]
            walked: 0,
        }
    }
    fn observe(&mut self, index: u64, amount: &Nat, out: &mut BTreeMap<Principal, MatchState>) {
        if self.subscriptions.is_empty() {
            return;
        }
        if self.record_high.as_ref().is_some_and(|old| amount <= old) {
            return;
        }
        #[cfg(test)]
        {
            self.walked += self.subscriptions.len();
        }
        observe_matches(index, amount, self.subscriptions.iter(), out);
        // A singleton gets no reusable high-water allocation: walking it is cheap.
        if self.subscriptions.len() > 1 {
            self.record_high = Some(amount.clone());
        }
    }
}

#[derive(Default)]
struct PendingAdmissions {
    globals: BTreeSet<Principal>,
    subscriptions: BTreeMap<
        (Principal, Principal, WatchTarget),
        (state::RelationKey, SubjectSubscription, bool),
    >,
    shared_overlay: BTreeMap<([u8; 32], Principal, Principal, WatchTarget), SubjectSubscription>,
    shared_cache: BTreeMap<[u8; 32], CachedInterests>,
}
impl PendingAdmissions {
    fn stage(&mut self, admission: PreparedAdmission, context: &mut AdmissionContext) {
        match admission {
            PreparedAdmission::Global(p) => {
                self.globals.insert(p);
            }
            PreparedAdmission::Accounts(subscriptions) => {
                for candidate in subscriptions {
                    let identity = candidate.identity();
                    let key = if let Some((key, old, _changed)) = self.subscriptions.get(&identity)
                    {
                        if old.subscription.minimum_units <= candidate.subscription.minimum_units {
                            continue;
                        }
                        key.clone()
                    } else {
                        let inputs = *context
                            .relations
                            .get_or_insert_with(|| state::relation_context(&context.runtime));
                        let key = state::relation_key(&candidate, inputs);
                        if let Some(old) = state::relation_at(&key) {
                            if old.identity() == identity
                                && old.subscription.minimum_units
                                    <= candidate.subscription.minimum_units
                            {
                                // Retain the known effective value for this page,
                                // without staging a write or invalidating watchers.
                                self.subscriptions.insert(identity, (key, old, false));
                                continue;
                            }
                        }
                        key
                    };
                    if context.runtime.observed_ledger == context.runtime.icp_ledger {
                        let account = key.icp_identifier().expect("shared account key");
                        self.shared_cache.remove(&account);
                        self.shared_overlay.insert(
                            (account, identity.0, identity.1, identity.2),
                            candidate.clone(),
                        );
                    }
                    self.subscriptions.insert(identity, (key, candidate, true));
                }
            }
        }
    }
    fn shared_subscriptions(&mut self, account: [u8; 32]) -> &mut CachedInterests {
        self.shared_cache.entry(account).or_insert_with(|| {
            let mut interested: BTreeMap<_, _> = state::interested_icp(account)
                .into_iter()
                .map(|s| (s.identity(), s))
                .collect();
            let lo = (
                account,
                Principal::management_canister(),
                Principal::management_canister(),
                WatchTarget::Subaccount(0),
            );
            for (_, s) in self
                .shared_overlay
                .range(lo..)
                .take_while(|(k, _)| k.0 == account)
            {
                interested.insert(s.identity(), s.clone());
            }
            CachedInterests::new(interested.into_values().collect())
        })
    }
    fn publish(self) {
        for (key, s, changed) in self.subscriptions.into_values() {
            if changed {
                state::put_relation_at(key, s)
            }
        }
        for p in self.globals {
            state::put_global_subscriber(p)
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
        DEBUG_ABORT_AFTER_STAGED_ADMISSION.with(std::cell::Cell::get)
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
    u64::try_from(&n.0).map_err(|_| "Nat exceeds u64".into())
}
fn account_id(b: &[u8]) -> Option<[u8; 32]> {
    b.try_into().ok()
}
#[cfg(test)]
fn merge(target: &mut BTreeMap<Principal, MatchState>, source: BTreeMap<Principal, MatchState>) {
    for (p, m) in source {
        let e = target.entry(p).or_default();
        e.global |= m.global;
        for (target_key, matched) in m.subs {
            record_match(
                e,
                target_key,
                matched.first_block_index,
                &matched.max_amount,
            );
        }
    }
}

fn record_match(
    state: &mut MatchState,
    target: (Principal, WatchTarget),
    first_block_index: u64,
    amount: &Nat,
) {
    if let Some(existing) = state.subs.get_mut(&target) {
        existing.first_block_index = existing.first_block_index.min(first_block_index);
        if amount > &existing.max_amount {
            existing.max_amount = amount.clone();
        }
    } else if state.subs.len() < MAX_SPECIFIC_TARGETS_PER_POKE {
        state.subs.insert(
            target,
            TargetMatch {
                first_block_index,
                max_amount: amount.clone(),
            },
        );
    }
}

struct AdmissionContext {
    runtime: config::RuntimeConfig,
    own_id: Principal,
    decimals: u8,
    accounts: Option<([u8; 32], [u8; 32])>,
    relations: Option<state::RelationContext>,
}
impl AdmissionContext {
    fn accounts(&mut self) -> &([u8; 32], [u8; 32]) {
        self.accounts.get_or_insert_with(|| {
            (
                account_identifier_bytes(self.runtime.faucet_canister, [0; 32]),
                account_identifier_bytes(self.own_id, [0; 32]),
            )
        })
    }
}

async fn prepare_admission(
    context: &mut AdmissionContext,
    from: &[u8],
    to: &[u8],
    memo: Option<&[u8]>,
) -> Option<PreparedAdmission> {
    let memo = memo?;
    let (faucet, own) = context.accounts();
    if account_id(from) != Some(*faucet) || account_id(to) != Some(*own) {
        return None;
    }
    let Ok(resolved) = resolve_subscription_memo(memo, state::alias) else {
        return None;
    };
    let subject = resolved.subject;
    let d = resolved.declaration;
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
        context.runtime.historian_canister,
        context.own_id,
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
                    Subscription::from_declaration(x, context.decimals)
                        .ok()
                        .map(|subscription| {
                            PreparedAdmission::Accounts(vec![SubjectSubscription {
                                subject,
                                subscription,
                            }])
                        })
                }
                SubscriptionDeclaration::Range {
                    subscriber,
                    neuron,
                    start,
                    end,
                    minimum,
                } => minimum
                    .map(|x| x.to_units(context.decimals))
                    .transpose()
                    .ok()
                    .map(|minimum| {
                        let minimum_units = minimum.unwrap_or_else(|| Nat::from(0u8));
                        PreparedAdmission::Accounts(
                            (start..=end)
                                .map(|n| SubjectSubscription {
                                    subject,
                                    subscription: Subscription {
                                        subscriber,
                                        target: if neuron {
                                            WatchTarget::NeuronNonce(n)
                                        } else {
                                            WatchTarget::Subaccount(n)
                                        },
                                        minimum_units: minimum_units.clone(),
                                    },
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
fn observe_matches(
    index: u64,
    amount: &Nat,
    interests: impl IntoIterator<Item = impl std::borrow::Borrow<SubjectSubscription>>,
    out: &mut BTreeMap<Principal, MatchState>,
) {
    for s in interests {
        let s = std::borrow::Borrow::<SubjectSubscription>::borrow(&s);
        if s.subscription.matches(amount) {
            record_match(
                out.entry(s.subscription.subscriber).or_default(),
                (s.subject, s.subscription.target),
                index,
                amount,
            )
        }
    }
}
fn observe_icrc3(
    first_block_index: u64,
    e: &icrc3::LedgerEvent,
    cache: &mut BTreeMap<(Principal, [u8; 32]), CachedInterests>,
    out: &mut BTreeMap<Principal, MatchState>,
) {
    if let icrc3::LedgerEvent::Transfer { to, amount, .. } = e {
        let interests = cache
            .entry((to.owner, *to.effective_subaccount()))
            .or_insert_with(|| CachedInterests::new(state::interested_icrc(*to)));
        interests.observe(first_block_index, amount, out)
    }
}
fn observe_icp(
    first_block_index: u64,
    op: &Option<icp_ledger::Operation>,
    pending: &mut PendingAdmissions,
    out: &mut BTreeMap<Principal, MatchState>,
) {
    if let Some(icp_ledger::Operation::Transfer { to, amount, .. }) = op {
        if let Some(id) = account_id(to) {
            pending
                .shared_subscriptions(id)
                .observe(first_block_index, &Nat::from(amount.e8s), out)
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
    let mut live = false;
    let mut cache = BTreeMap::new();
    for (id, block) in blocks {
        if id != at {
            return Err(format!("unexplained hole cursor={at} block={id}"));
        }
        let e = icrc3::decode_block(&block)?;
        observe_icrc3(id, &e, &mut cache, out);
        live = true;
        at = at.checked_add(1).ok_or("observed cursor overflow")?;
    }
    if at == original && at < boundary {
        return Err("no live progress or archive evidence".into());
    }
    if live {
        commit(Stream::Observed, true, at);
    }
    Ok((at, live))
}
/// Only successful scans yield their transient notifications. A failure drops the
/// entire scan-owned accumulator; page admission/cursor commits remain independent.
#[derive(Default)]
struct CompletedScan {
    matches: BTreeMap<Principal, MatchState>,
    activity: bool,
}

async fn scan_icrc(ledger: Principal) -> Result<CompletedScan, String> {
    let mut out = BTreeMap::new();
    let (ready, mut at) = position(&state::read_metadata(), Stream::Observed).unwrap();
    if !ready {
        let tip = u64nat(&icrc3::get_blocks(ledger, 0, 0).await?.log_length)?;
        commit(Stream::Observed, true, tip);
        return Ok(CompletedScan::default());
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
        let (n, live) = icrc_page(r, boundary, at, requested_length, &mut out).await?;
        at = n;
        activity |= live
    }
    Ok(CompletedScan {
        matches: out,
        activity,
    })
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
    context: &mut AdmissionContext,
    out: &mut BTreeMap<Principal, MatchState>,
    mut shared_globals: Option<&mut SharedGlobalOrder>,
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
        // The admitting Faucet transfer is still earlier than the new relation:
        // even watchers of this canister's default account start at later blocks.
        if matches!(s, Stream::Shared) {
            // No admission writer has run yet. Snapshot once at the first live
            // block, before observing or admitting that block; archive-only and
            // empty scans never enumerate globals.
            if let Some(order) = shared_globals.as_deref_mut() {
                order
                    .existing
                    .get_or_insert_with(|| state::global_subscribers().into_iter().collect());
            }
            observe_icp(id, &b.transaction.operation, &mut pending, out);
            page_last_live = Some(id);
        }
        if let Some(icp_ledger::Operation::Transfer { from, to, .. }) = &b.transaction.operation {
            if let Some(admission) =
                prepare_admission(context, from, to, b.transaction.icrc1_memo.as_deref()).await
            {
                if let PreparedAdmission::Global(subscriber) = &admission {
                    if let Some(order) = shared_globals.as_deref() {
                        if !order
                            .existing
                            .as_ref()
                            .expect("live snapshot")
                            .contains(subscriber)
                        {
                            page_global_admissions.entry(*subscriber).or_insert(id);
                        }
                    }
                }
                pending.stage(admission, context);
                if debug_abort_after_staged_admission() {
                    return Err("debug abort after staged legacy admission".into());
                }
            }
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
    }
    Ok((at, live))
}
async fn scan_legacy(s: Stream, decimals: u8) -> Result<CompletedScan, String> {
    let mut out = BTreeMap::new();
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
        return Ok(CompletedScan::default());
    }
    // Snapshot once per shared scan. Admissions take effect only for later blocks,
    // without walking the complete global registry for every transaction.
    let mut shared_globals = matches!(s, Stream::Shared).then(|| SharedGlobalOrder {
        existing: None,
        admitted_at: BTreeMap::new(),
        last_live: None,
    });
    let mut context = AdmissionContext {
        runtime: r.clone(),
        own_id: ic_cdk::api::canister_self(),
        decimals,
        accounts: None,
        relations: None,
    };
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
            &mut context,
            &mut out,
            shared_globals.as_mut(),
        )
        .await?;
        at = n;
        activity |= live
    }
    if let Some(order) = shared_globals {
        if let Some(last_live) = order.last_live {
            for subscriber in order.existing.unwrap_or_default() {
                out.entry(subscriber).or_default().global = true;
            }
            for (subscriber, admitted_at) in order.admitted_at {
                if last_live > admitted_at {
                    out.entry(subscriber).or_default().global = true;
                }
            }
        }
    }
    Ok(CompletedScan {
        matches: out,
        activity,
    })
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
        if let Err(e) = scan_legacy(stream, 0).await {
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
    let out = if shared {
        match scan_legacy(Stream::Shared, profile.decimals).await {
            Ok(scan) => {
                logging::ledger_recovered_all();
                scan.matches
            }
            Err(e) => {
                failure(Stream::Shared, &e);
                return;
            }
        }
    } else {
        let (mut out, activity) = match scan_icrc(r.observed_ledger).await {
            Ok(scan) => {
                logging::observed_ledger_recovered();
                (scan.matches, scan.activity)
            }
            Err(e) => {
                failure(Stream::Observed, &e);
                (BTreeMap::new(), false)
            }
        };
        // The guarded poll lane is the only production global-membership
        // writer. Other lanes update aliases/pricing/funding, never globals.
        // Snapshot after successful observation, before the admission stream.
        let globals = if activity {
            state::global_subscribers()
        } else {
            Vec::new()
        };
        match scan_legacy(Stream::Admission, profile.decimals).await {
            Ok(_) => logging::icp_admission_ledger_recovered(),
            Err(e) => failure(Stream::Admission, &e),
        }
        if activity {
            for p in globals {
                out.entry(p).or_default().global = true;
            }
        }
        out
    };
    let mut global_cache = None;
    dispatch_matches(out, |principal, notification| {
        subscriber::poke(principal, notification, &mut global_cache)
    });
}

#[cfg(test)]
mod target_match_tests {
    use super::*;
    use icrc_ledger_types::icrc3::{
        archive::QueryArchiveFn,
        blocks::{ArchivedBlocks, BlockWithId, GetBlocksRequest},
    };
    use std::str::FromStr;

    fn p(n: u8) -> Principal {
        Principal::from_slice(&[n, 1])
    }
    #[test]
    fn nat_cursor_conversion_preserves_full_width_and_overflow_errors() {
        for value in [0, (1u64 << 53) + 1, u64::MAX] {
            let amount = Nat::from(value);
            assert_eq!(u64nat(&amount), Ok(value));
            assert_eq!(amount, Nat::from(value));
        }
        for bits in [65usize, 32769] {
            let amount = Nat(Nat::from(1u8).0 << (bits - 1));
            assert_eq!(u64nat(&amount), Err("Nat exceeds u64".into()));
        }
    }
    #[test]
    fn immutable_admission_accounts_are_lazy_and_reused() {
        let own = p(7);
        let runtime = config::RuntimeConfig::production(p(8), None);
        let expected = (
            account_identifier_bytes(runtime.faucet_canister, [0; 32]),
            account_identifier_bytes(own, [0; 32]),
        );
        let mut context = AdmissionContext {
            runtime,
            own_id: own,
            decimals: 8,
            accounts: None,
            relations: None,
        };
        assert!(context.accounts.is_none());
        for _ in 0..256 {
            assert_eq!(*context.accounts(), expected);
        }
        assert_eq!(context.accounts, Some(expected));
    }

    #[test]
    fn independent_first_index_and_maximum_include_zero_and_full_width() {
        for (events, first, maximum) in [
            (vec![(0, 7)], 0, 7),
            (vec![(100, 4), (105, 15), (109, 8)], 100, 15),
            (vec![(200, 50), (205, 10)], 200, 50),
            (vec![(u64::MAX, 50), ((1 << 53) + 1, 10)], (1 << 53) + 1, 50),
        ] {
            let mut s = MatchState::default();
            let key = (p(1), WatchTarget::Subaccount(7));
            for (i, a) in events {
                record_match(&mut s, key, i, &Nat::from(a as u64))
            }
            assert_eq!(
                s.subs[&key],
                TargetMatch {
                    first_block_index: first,
                    max_amount: Nat::from(maximum as u64)
                }
            );
        }
    }
    #[test]
    fn page_merging_independently_combines_minimum_and_maximum_in_either_order() {
        for reverse in [false, true] {
            let key = (p(2), WatchTarget::Subaccount(7));
            let mut early = MatchState::default();
            record_match(&mut early, key, 100, &4u8.into());
            let mut late = MatchState {
                global: true,
                ..Default::default()
            };
            record_match(&mut late, key, 300, &15u8.into());
            let (a, b) = if reverse {
                (late, early)
            } else {
                (early, late)
            };
            let mut out = BTreeMap::from([(p(1), a)]);
            merge(&mut out, BTreeMap::from([(p(1), b)]));
            assert!(out[&p(1)].global);
            assert_eq!(
                out[&p(1)].subs[&key],
                TargetMatch {
                    first_block_index: 100,
                    max_amount: 15u8.into()
                }
            );
        }
    }
    #[test]
    fn later_page_overflow_still_updates_previously_retained_identity() {
        let mut first = MatchState::default();
        for n in 0..256 {
            record_match(
                &mut first,
                (p(2), WatchTarget::Subaccount(n)),
                100,
                &1u8.into(),
            );
        }
        let mut out = BTreeMap::from([(p(1), first)]);
        let mut local = out.clone();
        for n in 256..512 {
            record_match(
                local.get_mut(&p(1)).unwrap(),
                (p(2), WatchTarget::Subaccount(n)),
                200,
                &2u8.into(),
            );
        }
        record_match(
            local.get_mut(&p(1)).unwrap(),
            (p(2), WatchTarget::Subaccount(7)),
            300,
            &99u8.into(),
        );
        merge(&mut out, local);
        assert_eq!(out[&p(1)].subs.len(), 256);
        let retained = &out[&p(1)].subs[&(p(2), WatchTarget::Subaccount(7))];
        assert_eq!(retained.first_block_index, 100);
        assert_eq!(retained.max_amount, Nat::from(99u8));
    }

    #[test]
    fn below_threshold_observation_cannot_supply_first_index() {
        let relation = |subscriber, threshold| SubjectSubscription {
            subject: p(2),
            subscription: Subscription {
                subscriber,
                target: WatchTarget::Subaccount(7),
                minimum_units: Nat::from(threshold),
            },
        };
        let mut out = BTreeMap::new();
        for (i, a) in [(100, 2u64), (110, 15)] {
            observe_matches(
                i,
                &Nat::from(a),
                [relation(p(1), 1u64), relation(p(3), 10)],
                &mut out,
            )
        }
        let key = (p(2), WatchTarget::Subaccount(7));
        assert_eq!(out[&p(1)].subs[&key].first_block_index, 100);
        assert_eq!(out[&p(3)].subs[&key].first_block_index, 110);
        for s in out.values() {
            assert_eq!(s.subs[&key].max_amount, Nat::from(15u8))
        }
    }
    #[test]
    fn dispatches_global_matches_first_in_principal_order_with_unified_payloads() {
        let mut matches = BTreeMap::new();
        for n in [5, 3, 1, 4, 2] {
            let mut s = MatchState {
                global: n >= 3,
                ..Default::default()
            };
            if n != 4 {
                record_match(&mut s, (p(n), WatchTarget::Subaccount(7)), 0, &100u8.into());
                record_match(&mut s, (p(9), WatchTarget::Subaccount(7)), 1, &101u8.into());
            }
            matches.insert(p(n), s);
        }
        let mut seen = vec![];
        dispatch_matches(matches, |recipient, n| {
            seen.push(recipient);
            assert_eq!(
                n.global_activity,
                recipient == p(3) || recipient == p(4) || recipient == p(5)
            );
            if recipient == p(4) {
                assert!(n.matches.is_empty())
            } else {
                assert_eq!(n.matches[0].max_amount, Nat::from(100u8));
                assert_eq!(n.matches[1].subject, p(9));
            }
            true
        });
        assert_eq!(seen, vec![p(3), p(4), p(5), p(1), p(2)]);
    }
    #[test]
    fn no_notification_for_empty_non_global_state() {
        dispatch_matches(BTreeMap::from([(p(1), MatchState::default())]), |_, _| {
            panic!("empty notification must not be attempted")
        });
    }
    #[test]
    fn failed_global_attempts_do_not_block_specific_attempts() {
        let mut external = MatchState::default();
        record_match(
            &mut external,
            (p(3), WatchTarget::Subaccount(7)),
            0,
            &1u8.into(),
        );
        let mut seen = vec![];
        dispatch_matches(
            BTreeMap::from([
                (p(1), external),
                (
                    p(2),
                    MatchState {
                        global: true,
                        ..Default::default()
                    },
                ),
            ]),
            |p, n| {
                seen.push((p, n.global_activity));
                false
            },
        );
        assert_eq!(seen, vec![(p(2), true), (p(1), false)]);
    }
    #[test]
    fn dispatch_preserves_bounded_sorted_maxima_in_both_phases() {
        for global in [false, true] {
            let mut s = MatchState {
                global,
                ..Default::default()
            };
            let huge = Nat::from_str("18446744073709551616000000000000000000").unwrap();
            for n in (0..256).rev() {
                record_match(
                    &mut s,
                    (
                        if n % 2 == 0 { p(1) } else { p(2) },
                        WatchTarget::Subaccount(n),
                    ),
                    1000 - n,
                    &Nat::from(n),
                )
            }
            record_match(&mut s, (p(2), WatchTarget::Subaccount(7)), 3, &huge.clone());
            record_match(&mut s, (p(2), WatchTarget::Subaccount(7)), 1, &1u8.into());
            record_match(&mut s, (p(3), WatchTarget::NeuronNonce(0)), 0, &1u8.into());
            let mut attempts = 0;
            dispatch_matches(BTreeMap::from([(p(1), s)]), |_, n| {
                attempts += 1;
                assert_eq!(n.matches.len(), 256);
                assert!(n
                    .matches
                    .windows(2)
                    .all(|w| (w[0].subject, w[0].target) < (w[1].subject, w[1].target)));
                let m = n
                    .matches
                    .iter()
                    .find(|m| m.target == WatchTarget::Subaccount(7))
                    .unwrap();
                assert_eq!(m.first_block_index, 1);
                assert_eq!(m.max_amount, huge);
                true
            });
            assert_eq!(attempts, 1);
        }
    }
    #[test]
    fn maxima_order_and_arbitrary_precision_are_preserved() {
        let mut s = MatchState::default();
        let huge = Nat::from(u128::MAX) + Nat::from(1u8);
        for subject in [p(3), p(2)] {
            record_match(
                &mut s,
                (subject, WatchTarget::NeuronNonce(1)),
                u64::MAX,
                &huge.clone(),
            );
            record_match(
                &mut s,
                (subject, WatchTarget::Subaccount(7)),
                0,
                &1u8.into(),
            )
        }
        assert_eq!(s.subs.len(), 4);
        assert_eq!(
            s.subs[&(p(2), WatchTarget::NeuronNonce(1))].max_amount,
            huge
        );
        assert_eq!(
            s.subs.keys().next(),
            Some(&(p(2), WatchTarget::Subaccount(7)))
        );
    }
    #[test]
    fn specific_target_bound_keeps_updates_for_retained_targets() {
        let mut s = MatchState::default();
        for n in 0..300 {
            record_match(
                &mut s,
                (p(1), WatchTarget::Subaccount(n)),
                n + 100,
                &Nat::from(n),
            )
        }
        let key = (p(1), WatchTarget::Subaccount(7));
        record_match(&mut s, key, 0, &999u16.into());
        assert_eq!(s.subs.len(), 256);
        assert_eq!(s.subs[&key].first_block_index, 0);
        assert_eq!(s.subs[&key].max_amount, Nat::from(999u16));
        assert!(!s.subs.contains_key(&(p(1), WatchTarget::Subaccount(256))));
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

#[cfg(test)]
#[path = "polling_differential.rs"]
mod differential;
