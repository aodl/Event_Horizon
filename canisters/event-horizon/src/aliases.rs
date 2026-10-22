//! Complete, additive Historian snapshots. Persistent data lives in state IDs9/10.
use crate::{
    clients::historian::{CanisterAlias, ListCanisterAliasesArgs, ListCanisterAliasesResponse},
    config, state,
};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk::call::Call;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    time::Duration,
};

pub const MAX_ENTRIES: usize = 4096;
pub const PAGE_SIZE: u32 = 128;
const MAX_RESPONSE_BYTES: usize = 65_536;
#[derive(Default, Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct AliasMetadata {
    pub revision: Option<Vec<u8>>,
    pub last_success: Option<u64>,
    pub last_attempt: Option<u64>,
}
pub fn valid_alias(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 8
        && b[0].is_ascii_uppercase()
        && b.iter()
            .all(|v| v.is_ascii_uppercase() || v.is_ascii_digit())
        && s != "X"
}
pub fn valid_canister(p: Principal) -> bool {
    p.as_slice().last() == Some(&1)
}
pub fn digest(items: &[CanisterAlias]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(b"JUPITER_CANISTER_ALIASES_V1\0");
    h.update((items.len() as u32).to_be_bytes());
    for item in items {
        h.update([item.alias.len() as u8]);
        h.update(item.alias.as_bytes());
        h.update([item.canister_id.as_slice().len() as u8]);
        h.update(item.canister_id.as_slice());
    }
    h.finalize().to_vec()
}
#[derive(Default)]
pub struct Snapshot {
    revision: Option<Vec<u8>>,
    items: Vec<CanisterAlias>,
    cursor: Option<String>,
    complete: bool,
    pages: usize,
}
impl Snapshot {
    pub fn cursor(&self) -> Option<String> {
        self.cursor.clone()
    }
    pub fn push(&mut self, page: ListCanisterAliasesResponse) -> Result<bool, String> {
        self.pages += 1;
        if (page.items.is_empty() && self.cursor.is_some())
            || (self.pages == 32 && page.next_start_after.is_some())
        {
            return Err("incomplete alias pagination".into());
        }
        if self.complete
            || self.pages > 32
            || page.items.len() > PAGE_SIZE as usize
            || self.items.len() + page.items.len() > MAX_ENTRIES
            || page.revision.len() != 32
        {
            return Err("alias snapshot bounds".into());
        }
        if self.revision.as_ref().is_some_and(|r| r != &page.revision) {
            return Err("alias revision changed".into());
        }
        self.revision = Some(page.revision);
        for item in page.items {
            if !valid_alias(&item.alias)
                || !valid_canister(item.canister_id)
                || self.items.last().is_some_and(|v| v.alias >= item.alias)
            {
                return Err("invalid or unsorted alias".into());
            }
            self.items.push(item);
        }
        if let Some(next) = page.next_start_after {
            if self.items.last().map(|v| &v.alias) != Some(&next)
                || self.cursor.as_ref().is_some_and(|p| p >= &next)
            {
                return Err("alias cursor did not progress".into());
            }
            self.cursor = Some(next);
            Ok(false)
        } else {
            self.complete = true;
            Ok(true)
        }
    }
    pub fn finish(
        self,
        old: &BTreeMap<String, Principal>,
    ) -> Result<(Vec<CanisterAlias>, Vec<u8>), String> {
        if !self.complete || self.revision.as_ref() != Some(&digest(&self.items)) {
            return Err("incomplete alias snapshot or digest mismatch".into());
        }
        let table: BTreeMap<_, _> = self
            .items
            .iter()
            .map(|i| (i.alias.clone(), i.canister_id))
            .collect();
        if old.iter().any(|(a, p)| table.get(a) != Some(p)) {
            return Err("alias deletion or reassignment".into());
        }
        Ok((self.items, self.revision.expect("validated revision")))
    }
}
pub fn due(now: u64, meta: &AliasMetadata) -> bool {
    meta.last_attempt.is_none_or(|t| t / 86400 < now / 86400)
}
fn now() -> u64 {
    ic_cdk::api::time() / 1_000_000_000
}
async fn fetch() -> Result<(), String> {
    let mut snapshot = Snapshot::default();
    loop {
        let args = ListCanisterAliasesArgs {
            start_after: snapshot.cursor(),
            limit: Some(PAGE_SIZE),
        };
        let call = Call::bounded_wait(
            config::runtime().historian_canister,
            "list_canister_aliases",
        )
        .with_arg(&args);
        if ic_cdk::api::canister_liquid_cycle_balance()
            < config::RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
        {
            return Err(config::RESERVE_PROTECTION_ERROR.into());
        }
        let response = call.await.map_err(|e| format!("alias transport: {e:?}"))?;
        if response.as_ref().len() > MAX_RESPONSE_BYTES {
            return Err("alias response too large".into());
        }
        let page = response
            .candid::<ListCanisterAliasesResponse>()
            .map_err(|e| format!("alias decode: {e:?}"))?;
        if snapshot.push(page)? {
            break;
        }
    }
    let (items, revision) = snapshot.finish(&state::alias_table())?;
    state::publish_aliases(items, revision, now());
    Ok(())
}
thread_local! {
    static RUNNING: Cell<bool> = const{Cell::new(false)};
    static TIMER: RefCell<Option<ic_cdk_timers::TimerId>> = const{RefCell::new(None)};
}
fn schedule() {
    let n = now();
    let m = state::alias_metadata();
    let delay = if due(n, &m) { 0 } else { 86400 - n % 86400 };
    let id = ic_cdk_timers::set_timer(Duration::from_secs(delay), async {
        TIMER.with_borrow_mut(|v| v.take());
        refresh().await;
    });
    TIMER.with_borrow_mut(|v| {
        if let Some(old) = v.replace(id) {
            ic_cdk_timers::clear_timer(old)
        }
    });
}
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        RUNNING.with(|v| v.set(false));
        schedule();
    }
}
pub async fn refresh() {
    if RUNNING.with(|v| v.replace(true)) {
        return;
    }
    let _guard = Guard;
    let n = now();
    let mut m = state::alias_metadata();
    if !due(n, &m) {
        return;
    }
    m.last_attempt = Some(n);
    state::write_alias_metadata(m);
    if let Err(e) = fetch().await {
        if !config::is_reserve_protection(&e) {
            ic_cdk::println!("ALIASES_UNAVAILABLE {e}")
        }
    }
}
#[cfg(not(feature = "debug_api"))]
pub fn start() {
    schedule()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(a: &str, n: u8) -> CanisterAlias {
        CanisterAlias {
            alias: a.into(),
            canister_id: Principal::from_slice(&[n, 1]),
        }
    }
    fn page(
        items: Vec<CanisterAlias>,
        revision: Vec<u8>,
        next: Option<&str>,
    ) -> ListCanisterAliasesResponse {
        ListCanisterAliasesResponse {
            items,
            revision,
            next_start_after: next.map(str::to_string),
        }
    }
    #[test]
    fn golden_digest_and_daily_schedule() {
        let e = CanisterAlias {
            alias: "E".into(),
            canister_id: Principal::from_text("eo6ei-gaaaa-aaaar-qchra-cai").unwrap(),
        };
        assert_eq!(
            hex::encode(digest(&[e])),
            "89d9f25601962fb394ad9ebf34aaceccf46b833b9cfd3ed2dc7eb942e9d63625"
        );
        assert!(due(1, &AliasMetadata::default()));
        let m = AliasMetadata {
            last_attempt: Some(1),
            ..Default::default()
        };
        assert!(!due(86399, &m));
        assert!(due(86400, &m));
    }
    #[test]
    fn complete_pages_and_additive_cache_validation() {
        let items: Vec<_> = (0..260)
            .map(|n| item(&format!("A{n:04}"), (n % 200) as u8))
            .collect();
        let revision = digest(&items);
        let mut s = Snapshot::default();
        assert!(!s
            .push(page(items[..128].to_vec(), revision.clone(), Some("A0127")))
            .unwrap());
        assert!(!s
            .push(page(
                items[128..256].to_vec(),
                revision.clone(),
                Some("A0255")
            ))
            .unwrap());
        assert!(s.push(page(items[256..].to_vec(), revision, None)).unwrap());
        assert_eq!(
            s.finish(&BTreeMap::from([("A0000".into(), items[0].canister_id)]))
                .unwrap()
                .0,
            items
        );
    }
    #[test]
    fn invalid_snapshots_never_publish() {
        let a = item("A", 1);
        let b = item("B", 2);
        let good = vec![a.clone(), b.clone()];
        let d = digest(&good);
        for bad in [
            vec![b.clone(), a.clone()],
            vec![a.clone(), a.clone()],
            vec![item("lower", 1)],
            vec![item("A12345678", 1)],
            vec![item("X", 1)],
            vec![CanisterAlias {
                alias: "A".into(),
                canister_id: Principal::anonymous(),
            }],
        ] {
            let mut s = Snapshot::default();
            assert!(s.push(page(bad, d.clone(), None)).is_err())
        }
        let mut s = Snapshot::default();
        assert!(!s.push(page(vec![a.clone()], d.clone(), Some("A"))).unwrap());
        assert!(s.push(page(vec![b.clone()], vec![0; 32], None)).is_err());
        for old in [
            BTreeMap::from([("A".into(), b.canister_id)]),
            BTreeMap::from([("C".into(), b.canister_id)]),
        ] {
            let mut s = Snapshot::default();
            s.push(page(good.clone(), d.clone(), None)).unwrap();
            assert!(s.finish(&old).is_err())
        }
        let mut s = Snapshot::default();
        s.push(page(vec![a], d, None)).unwrap();
        assert!(s.finish(&BTreeMap::new()).is_err());
    }
}
