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
    icrc3::{
        archive::QueryArchiveFn,
        blocks::{ArchivedBlocks, BlockWithId, GetBlocksRequest, GetBlocksResult},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

const MAX_SPECIFIC_TARGETS_PER_POKE: usize = 256;
const MAX_ICRC_ARCHIVE_CALLS: usize = 16;

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
    last_processed: Option<u64>,
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
fn covering_archive<'a, T: 'a>(
    ranges: impl IntoIterator<Item = Result<(u64, u64, &'a T), String>>,
    requested: &Range<u64>,
) -> Result<Option<(u64, &'a T)>, String> {
    let mut selected = None;
    for range in ranges {
        let (start, length, callback) = range?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| format!("archive interval overflow start={start} length={length}"))?;
        let end = end.min(requested.end);
        // Keep the first covering range, while validating all advertised ranges.
        if selected.is_none() && start <= requested.start && requested.start < end {
            selected = Some((end, callback));
        }
    }
    Ok(selected)
}

fn archive_failure(error: String, canister: Principal, method: &str, range: &Range<u64>) -> String {
    if config::is_reserve_protection(&error) {
        return error;
    }
    format!(
        "archive {canister}.{method} {}..{}: {error}",
        range.start, range.end
    )
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

type IcrcArchiveCallback = QueryArchiveFn<Vec<GetBlocksRequest>, GetBlocksResult>;

fn icrc_archived<'a>(
    groups: &'a [ArchivedBlocks],
    requested: &Range<u64>,
) -> Result<Option<(u64, &'a IcrcArchiveCallback)>, String> {
    if groups.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive groups".into());
    }
    if groups.iter().map(|a| a.args.len()).sum::<usize>() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive ranges".into());
    }
    covering_archive(
        groups.iter().flat_map(|a| {
            a.args.iter().map(|q| {
                let (start, length) = q.as_start_and_length()?;
                Ok((start, length, &a.callback))
            })
        }),
        requested,
    )
}

fn icrc_blocks(
    mut blocks: Vec<BlockWithId>,
    requested: &Range<u64>,
) -> Result<Vec<(u64, ICRC3Value)>, String> {
    if blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive blocks".into());
    }
    blocks.sort_by(|a, b| a.id.cmp(&b.id));
    let mut unique: Vec<(u64, ICRC3Value)> = Vec::new();
    for block in blocks {
        let id = u64nat(&block.id)?;
        if !requested.contains(&id) {
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

async fn icrc_prefix(
    mut response: GetBlocksResult,
    mut requested: Range<u64>,
) -> Result<Vec<(u64, ICRC3Value)>, String> {
    let mut source: Option<IcrcArchiveCallback> = None;
    let mut calls = 0;
    loop {
        let context = |error| match &source {
            Some(callback) => {
                archive_failure(error, callback.canister_id, &callback.method, &requested)
            }
            None => error,
        };
        let referral = icrc_archived(&response.archived_blocks, &requested).map_err(context)?;
        let mut blocks = icrc_blocks(response.blocks, &requested).map_err(context)?;
        if blocks.first().is_some_and(|(id, _)| *id == requested.start) {
            // A single reply can advertise a hole between locally available blocks.
            // Commit the validated prefix and let the next root request resolve it.
            if let Some((index, pair)) = blocks
                .windows(2)
                .enumerate()
                .find(|(_, pair)| pair[0].0 + 1 != pair[1].0)
            {
                let missing = pair[0].0 + 1; // Both IDs are below the exclusive end.
                if icrc_archived(&response.archived_blocks, &(missing..requested.end))
                    .map_err(context)?
                    .is_some()
                {
                    blocks.truncate(index + 1);
                }
            }
            return Ok(blocks);
        }
        let (end, callback) = referral.ok_or_else(|| {
            context(match blocks.first() {
                Some((id, _)) => format!("unexplained hole cursor={} block={id}", requested.start),
                None => "no blocks or covering archive callback".into(),
            })
        })?;
        if calls == MAX_ICRC_ARCHIVE_CALLS {
            return Err(context("archive callback limit reached".into()));
        }
        requested.end = end;
        let callback = callback.clone();
        response = icrc3::archive_blocks(&callback, requested.start, end - requested.start)
            .await
            .map_err(|error| {
                archive_failure(error, callback.canister_id, &callback.method, &requested)
            })?;
        // Archive log_length describes its local store. It cannot alter the root boundary.
        source = Some(callback);
        calls += 1;
    }
}

async fn icrc_page(
    r: GetBlocksResult,
    requested: Range<u64>,
    out: &mut BTreeMap<Principal, MatchState>,
) -> Result<u64, String> {
    let blocks = icrc_prefix(r, requested.clone()).await?;
    let mut at = position(&state::read_metadata(), Stream::Observed)
        .unwrap()
        .1;
    if at != requested.start {
        return Err(format!(
            "observed request cursor changed expected={} actual={at}",
            requested.start
        ));
    }
    let mut cache = BTreeMap::new();
    for (id, block) in blocks {
        if id != at {
            return Err(format!("unexplained hole cursor={at} block={id}"));
        }
        let event = icrc3::decode_block(&block)?;
        observe_icrc3(id, &event, &mut cache, out);
        at = id.checked_add(1).ok_or("observed cursor overflow")?;
    }
    commit(Stream::Observed, true, at);
    Ok(at)
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
        at = icrc_page(
            r,
            at..request_end(at, requested_length, boundary)?,
            &mut out,
        )
        .await?;
        activity = true;
    }
    Ok(CompletedScan {
        matches: out,
        activity,
    })
}

fn legacy_archived<'a>(
    ranges: &'a [icp_ledger::ArchivedBlocksRange],
    requested: &Range<u64>,
) -> Result<Option<(u64, &'a icp_ledger::ArchiveCallback)>, String> {
    if ranges.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive archive ranges".into());
    }
    covering_archive(
        ranges
            .iter()
            .map(|range| Ok((range.start, range.length, &range.callback))),
        requested,
    )
}

async fn legacy_prefix(
    r: icp_ledger::QueryBlocksResponse,
    requested: &Range<u64>,
) -> Result<Vec<icp_ledger::Block>, String> {
    if r.blocks.len() > config::LEDGER_PAGE_SIZE as usize {
        return Err("excessive blocks".into());
    }
    let referral = legacy_archived(&r.archived_blocks, requested)?;
    if !r.blocks.is_empty() {
        let end = r
            .first_block_index
            .checked_add(r.blocks.len() as u64)
            .ok_or("legacy block index overflow")?;
        if (r.first_block_index..end).contains(&requested.start) {
            let offset = (requested.start - r.first_block_index) as usize;
            let length = (end.min(requested.end) - requested.start) as usize;
            return Ok(r.blocks.into_iter().skip(offset).take(length).collect());
        }
    }
    let Some((end, callback)) = referral else {
        return Err(
            if !r.blocks.is_empty() && r.first_block_index > requested.start {
                format!(
                    "unexplained hole cursor={} first={}",
                    requested.start, r.first_block_index
                )
            } else {
                "no blocks or covering archive callback".into()
            },
        );
    };
    let interval = requested.start..end;
    let length = end - interval.start;
    let result = icp_ledger::archive_blocks(callback, interval.start, length)
        .await
        .and_then(|blocks| {
            if blocks.is_empty() || blocks.len() as u64 > length {
                return Err(format!(
                    "invalid archive block count={} requested={length}",
                    blocks.len()
                ));
            }
            Ok(blocks)
        });
    result.map_err(|error| {
        archive_failure(error, callback.0.principal, &callback.0.method, &interval)
    })
}

async fn legacy_page(
    r: icp_ledger::QueryBlocksResponse,
    s: Stream,
    requested: Range<u64>,
    context: &mut AdmissionContext,
    out: &mut BTreeMap<Principal, MatchState>,
    mut shared_globals: Option<&mut SharedGlobalOrder>,
) -> Result<u64, String> {
    let blocks = legacy_prefix(r, &requested).await?;
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
    let mut pending = PendingAdmissions::default();
    let mut page_global_admissions = BTreeMap::new();
    let end = original
        .checked_add(blocks.len() as u64)
        .ok_or("legacy cursor overflow")?;
    for (id, b) in (original..end).zip(blocks) {
        // The admitting Faucet transfer is still earlier than the new relation:
        // even watchers of this canister's default account start at later blocks.
        if matches!(s, Stream::Shared) {
            // Snapshot global membership once before observing or admitting
            // the first fetched block.
            if let Some(order) = shared_globals.as_deref_mut() {
                order
                    .existing
                    .get_or_insert_with(|| state::global_subscribers().into_iter().collect());
            }
            observe_icp(id, &b.transaction.operation, &mut pending, out);
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
                            .expect("global snapshot")
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
    }
    // Publish the verified admissions and both shared cursors without an await.
    pending.publish();
    commit(s, true, end);
    if let Some(order) = shared_globals {
        for (subscriber, admitted_at) in page_global_admissions {
            order.admitted_at.entry(subscriber).or_insert(admitted_at);
        }
        order.last_processed = Some(end - 1);
    }
    Ok(end)
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
        last_processed: None,
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
        at = legacy_page(
            response,
            s,
            at..request_end(at, requested_length, boundary)?,
            &mut context,
            &mut out,
            shared_globals.as_mut(),
        )
        .await?;
        activity = true;
    }
    if let Some(order) = shared_globals {
        if let Some(last_processed) = order.last_processed {
            for subscriber in order.existing.unwrap_or_default() {
                out.entry(subscriber).or_default().global = true;
            }
            for (subscriber, admitted_at) in order.admitted_at {
                if last_processed > admitted_at {
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
    let notification_driven = !shared
        && state::read_instance_config()
            .icp_admission_notifier
            .is_some();
    // Capture the prospective tip independently of observed-ledger readiness.
    // With an uninitialized cursor this reads only the tip and admits nothing.
    if !state::read_metadata().admission_bootstrapped {
        let attempt = notification_driven
            .then(crate::scheduler::AdmissionAttempt::begin)
            .flatten();
        if !notification_driven || attempt.is_some() {
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
            } else if let Some(attempt) = attempt {
                attempt.finish();
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
        let attempt = notification_driven
            .then(crate::scheduler::AdmissionAttempt::begin)
            .flatten();
        if !notification_driven || attempt.is_some() {
            match scan_legacy(Stream::Admission, profile.decimals).await {
                Ok(_) => {
                    if let Some(attempt) = attempt {
                        attempt.finish();
                    }
                    logging::icp_admission_ledger_recovered();
                }
                Err(e) => failure(Stream::Admission, &e),
            }
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
    fn archive_selection_clips_one_covering_range_without_joining_adjacent_ranges() {
        for (ranges, requested, expected) in [
            (vec![(15, 5), (8, 5), (13, 4)], 10..20, Some((13, &1))),
            (vec![(10, 0), (11, 9)], 10..20, None),
            (vec![(8, 100), (10, 2)], 10..20, Some((20, &0))),
            (
                vec![(u64::MAX - 2, 2)],
                u64::MAX - 1..u64::MAX,
                Some((u64::MAX, &0)),
            ),
        ] {
            let callbacks = [0, 1, 2];
            assert_eq!(
                covering_archive(
                    ranges
                        .into_iter()
                        .zip(&callbacks)
                        .map(|((start, length), callback)| Ok((start, length, callback))),
                    &requested,
                )
                .unwrap(),
                expected,
            );
        }
        assert!(covering_archive([Ok((0, 10, &0)), Ok((u64::MAX, 1, &1))], &(0..10)).is_err());
        assert_eq!(request_end(u64::MAX - 1, 1, u64::MAX).unwrap(), u64::MAX);
        assert!(request_end(u64::MAX, 1, u64::MAX).is_err());
        assert_eq!(page_length(u64::MAX, None), 0);
    }

    #[test]
    fn legacy_archive_wrapper_rejects_excessive_or_overflowing_ranges() {
        let callback = icp_ledger::ArchiveCallback::new(Principal::anonymous(), "unused".into());
        let excessive = vec![
            icp_ledger::ArchivedBlocksRange {
                start: 0,
                length: 0,
                callback: callback.clone(),
            };
            config::LEDGER_PAGE_SIZE as usize + 1
        ];
        assert_eq!(
            legacy_archived(&excessive, &(0..10)).unwrap_err(),
            "excessive archive ranges"
        );
        assert!(legacy_archived(
            &[icp_ledger::ArchivedBlocksRange {
                start: u64::MAX,
                length: 1,
                callback,
            }],
            &(0..10),
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
            icrc_archived(&excessive_groups.archived_blocks, &(0..10)).unwrap_err(),
            "excessive archive groups"
        );
        let excessive_ranges = icrc_result(vec![(0..=config::LEDGER_PAGE_SIZE)
            .map(|_| request(0u8.into(), 0u8.into()))
            .collect()]);
        assert_eq!(
            icrc_archived(&excessive_ranges.archived_blocks, &(0..10)).unwrap_err(),
            "excessive archive ranges"
        );
        let above_u64 = Nat::from_str("18446744073709551616").unwrap();
        for malformed in [
            icrc_result(vec![vec![request(above_u64.clone(), 1u8.into())]]),
            icrc_result(vec![vec![request(1u8.into(), above_u64)]]),
            icrc_result(vec![vec![request(u64::MAX.into(), 1u8.into())]]),
        ] {
            assert!(icrc_archived(&malformed.archived_blocks, &(0..10)).is_err());
        }
    }

    fn block(id: u64, value: u64) -> BlockWithId {
        BlockWithId {
            id: id.into(),
            block: ICRC3Value::Nat(value.into()),
        }
    }

    #[test]
    fn icrc_blocks_sort_and_deduplicate_without_selecting_conflicts() {
        assert_eq!(
            icrc_blocks(
                vec![block(12, 3), block(11, 2), block(10, 1), block(10, 1)],
                &(10..12)
            )
            .unwrap(),
            vec![
                (10, ICRC3Value::Nat(1u8.into())),
                (11, ICRC3Value::Nat(2u8.into()))
            ]
        );
        assert_eq!(
            icrc_blocks(vec![block(10, 1), block(10, 2)], &(10..11)).unwrap_err(),
            "conflicting duplicate block id=10"
        );
    }
    #[test]
    fn icrc_normalization_validates_all_numeric_ids_and_bounds_blocks() {
        let range = u64::MAX - 2..u64::MAX;
        assert_eq!(
            icrc_blocks(
                vec![
                    block(u64::MAX, 1),
                    block(u64::MAX, 2),
                    block(u64::MAX - 2, 3)
                ],
                &range
            )
            .unwrap(),
            vec![(u64::MAX - 2, ICRC3Value::Nat(3u8.into()))]
        );
        let mut oversized = block(0, 1);
        oversized.id = Nat::from(u64::MAX) + Nat::from(1u8);
        assert_eq!(
            icrc_blocks(vec![oversized], &range).unwrap_err(),
            "Nat exceeds u64"
        );
        let extra = vec![block(0, 1); config::LEDGER_PAGE_SIZE as usize + 1];
        assert_eq!(icrc_blocks(extra, &range).unwrap_err(), "excessive blocks");
    }
}

#[cfg(test)]
#[path = "polling_differential.rs"]
mod differential;
