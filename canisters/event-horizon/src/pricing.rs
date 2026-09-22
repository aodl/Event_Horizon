use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{clients::cmc, config, state};

pub const STANDARD_BASE_ICP: u64 = 10;
pub const GLOBAL_BASE_ICP: u64 = 100;
pub const SECONDS_PER_DAY: u64 = 86_400;
pub const PRICE_HISTORY_DAYS: u64 = 1_461;
pub const PRICE_FREEZE_LEAD_SECONDS: u64 = 7 * SECONDS_PER_DAY;
pub const STALE_RATE_LIMIT_SECONDS: u64 = 7 * SECONDS_PER_DAY;

#[derive(CandidType, Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Price {
    pub account_icp: u64,
    pub global_icp: u64,
}

impl Price {
    pub const BASE: Self = Self {
        account_icp: STANDARD_BASE_ICP,
        global_icp: GLOBAL_BASE_ICP,
    };
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct PricingState {
    pub initialized: bool,
    pub current: Price,
    pub current_effective_at: u64,
    pub next: Option<Price>,
    pub next_effective_at: u64,
    pub next_freeze_at: u64,
    pub next_carried_forward_due_to_stale_rate: bool,
    pub last_observation_attempt_day: Option<u64>,
    pub last_observation_call_cost: u128,
}

impl Default for PricingState {
    fn default() -> Self {
        Self {
            initialized: false,
            current: Price::BASE,
            current_effective_at: 0,
            next: None,
            next_effective_at: 0,
            next_freeze_at: 0,
            next_carried_forward_due_to_stale_rate: false,
            last_observation_attempt_day: None,
            last_observation_call_cost: 0,
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub utc_day: u64,
    pub recorded_at: u64,
    pub rate_observed_at: u64,
    pub xdr_permyriad_per_icp: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pricing {
    pub initialized: bool,
    pub current: Price,
    pub current_effective_at: u64,
    pub next: Option<Price>,
    pub next_effective_at: u64,
    pub next_freeze_at: u64,
    pub observed_floor_xdr_permyriad: u64,
    pub floor_observed_at: u64,
    pub latest_xdr_permyriad: u64,
    pub latest_observed_at: u64,
    pub next_carried_forward_due_to_stale_rate: bool,
}

pub fn utc_day(timestamp_seconds: u64) -> u64 {
    timestamp_seconds / SECONDS_PER_DAY
}

pub fn oldest_retained_day(now_seconds: u64) -> u64 {
    utc_day(now_seconds).saturating_sub(PRICE_HISTORY_DAYS - 1)
}

pub fn calculate_price(base_icp: u64, floor: u64, current: u64) -> Result<u64, String> {
    if floor == 0 || current == 0 {
        return Err("conversion rates must be nonzero".to_string());
    }
    let numerator = u128::from(base_icp)
        .checked_mul(u128::from(floor))
        .ok_or_else(|| "price numerator overflow".to_string())?;
    let value = numerator
        .checked_add(u128::from(current) - 1)
        .ok_or_else(|| "price rounding overflow".to_string())?
        / u128::from(current);
    u64::try_from(value).map_err(|_| "calculated price does not fit nat64".to_string())
}

pub fn calculate_prices(floor: u64, current: u64) -> Result<Price, String> {
    Ok(Price {
        account_icp: calculate_price(STANDARD_BASE_ICP, floor, current)?,
        global_icp: calculate_price(GLOBAL_BASE_ICP, floor, current)?,
    })
}

fn civil_from_days(days_since_epoch: u64) -> (i64, u32, u32) {
    let z = i128::from(days_since_epoch) + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year as i64, month as u32, day as u32)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> u64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let yoe = adjusted_year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let doy = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    u64::try_from(era * 146_097 + doe - 719_468).expect("post-epoch UTC date")
}

pub fn next_month_start(now_seconds: u64) -> u64 {
    let (year, month, _) = civil_from_days(utc_day(now_seconds));
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    days_from_civil(next_year, next_month, 1) * SECONDS_PER_DAY
}

pub fn initialize(state: &mut PricingState, now_seconds: u64) {
    if state.initialized {
        return;
    }
    state.initialized = true;
    state.current = Price::BASE;
    state.current_effective_at = now_seconds;
    state.next_effective_at = next_month_start(now_seconds);
    state.next_freeze_at = state
        .next_effective_at
        .saturating_sub(PRICE_FREEZE_LEAD_SECONDS);
}

fn reference_before(
    observations: &[Observation],
    cutoff_exclusive: u64,
) -> Option<(Observation, Observation)> {
    let mut floor: Option<Observation> = None;
    let mut latest: Option<Observation> = None;
    for observation in observations
        .iter()
        .copied()
        .filter(|o| o.recorded_at < cutoff_exclusive)
    {
        if observation.xdr_permyriad_per_icp == 0 {
            continue;
        }
        if floor
            .map(|value| observation.xdr_permyriad_per_icp < value.xdr_permyriad_per_icp)
            .unwrap_or(true)
        {
            floor = Some(observation);
        }
        if latest
            .map(|value| observation.recorded_at > value.recorded_at)
            .unwrap_or(true)
        {
            latest = Some(observation);
        }
    }
    floor.zip(latest)
}

fn freeze_next(state: &mut PricingState, observations: &[Observation]) {
    if state.next.is_some() {
        return;
    }
    let reference = reference_before(observations, state.next_freeze_at);
    let stale = reference
        .map(|(_, latest)| {
            state.next_freeze_at.saturating_sub(latest.rate_observed_at) > STALE_RATE_LIMIT_SECONDS
        })
        .unwrap_or(true);
    if stale {
        state.next = Some(state.current);
        state.next_carried_forward_due_to_stale_rate = true;
        return;
    }
    let (floor, latest) = reference.expect("fresh reference exists");
    match calculate_prices(floor.xdr_permyriad_per_icp, latest.xdr_permyriad_per_icp) {
        Ok(price) => {
            state.next = Some(price);
            state.next_carried_forward_due_to_stale_rate = false;
        }
        Err(_) => {
            state.next = Some(state.current);
            state.next_carried_forward_due_to_stale_rate = true;
        }
    }
}

pub fn advance_epochs(state: &mut PricingState, observations: &[Observation], now_seconds: u64) {
    if !state.initialized {
        return;
    }
    loop {
        if now_seconds >= state.next_freeze_at {
            freeze_next(state, observations);
        }
        if now_seconds < state.next_effective_at {
            break;
        }
        state.current = state.next.unwrap_or(state.current);
        state.current_effective_at = state.next_effective_at;
        state.next = None;
        state.next_carried_forward_due_to_stale_rate = false;
        state.next_effective_at = next_month_start(state.current_effective_at);
        state.next_freeze_at = state
            .next_effective_at
            .saturating_sub(PRICE_FREEZE_LEAD_SECONDS);
    }
}

pub fn public_pricing(state: &PricingState, observations: &[Observation]) -> Pricing {
    let reference = reference_before(observations, u64::MAX);
    Pricing {
        initialized: state.initialized,
        current: state.current,
        current_effective_at: state.current_effective_at,
        next: state.next,
        next_effective_at: state.next_effective_at,
        next_freeze_at: state.next_freeze_at,
        observed_floor_xdr_permyriad: reference.map(|v| v.0.xdr_permyriad_per_icp).unwrap_or(0),
        floor_observed_at: reference.map(|v| v.0.rate_observed_at).unwrap_or(0),
        latest_xdr_permyriad: reference.map(|v| v.1.xdr_permyriad_per_icp).unwrap_or(0),
        latest_observed_at: reference.map(|v| v.1.rate_observed_at).unwrap_or(0),
        next_carried_forward_due_to_stale_rate: state.next_carried_forward_due_to_stale_rate,
    }
}

pub fn current_admission_e8s(global: bool) -> Option<u64> {
    let state = state::read_pricing_state();
    if !state.initialized {
        return None;
    }
    let whole_icp = if global {
        state.current.global_icp
    } else {
        state.current.account_icp
    };
    whole_icp.checked_mul(100_000_000)
}

pub fn get_pricing() -> Pricing {
    public_pricing(&state::read_pricing_state(), &state::price_observations())
}

pub async fn run_maintenance() {
    let now = ic_cdk::api::time() / 1_000_000_000;
    state::prune_price_observations(oldest_retained_day(now));
    let mut pricing = state::read_pricing_state();
    let observations = state::price_observations();
    advance_epochs(&mut pricing, &observations, now);

    let today = utc_day(now);
    if pricing.last_observation_attempt_day == Some(today) {
        state::write_pricing_state(pricing);
        return;
    }
    pricing.last_observation_attempt_day = Some(today);
    state::write_pricing_state(pricing.clone());

    let runtime = config::runtime();
    let Ok((rate, call_cost)) = cmc::get_icp_xdr_conversion_rate(runtime.cmc_canister).await else {
        return;
    };
    if state::price_observation(today).is_some() {
        return;
    }
    state::put_price_observation(Observation {
        utc_day: today,
        recorded_at: now,
        rate_observed_at: rate.timestamp_seconds,
        xdr_permyriad_per_icp: rate.xdr_permyriad_per_icp,
    });
    let mut pricing = state::read_pricing_state();
    pricing.last_observation_call_cost = call_cost;
    initialize(&mut pricing, now);
    advance_epochs(&mut pricing, &state::price_observations(), now);
    state::write_pricing_state(pricing);
}

pub fn seconds_until_next_event(now_seconds: u64) -> u64 {
    let next_day = utc_day(now_seconds)
        .saturating_add(1)
        .saturating_mul(SECONDS_PER_DAY);
    let state = state::read_pricing_state();
    let mut next = next_day;
    if state.initialized {
        for event in [state.next_freeze_at, state.next_effective_at] {
            if event > now_seconds {
                next = next.min(event);
            }
        }
    }
    next.saturating_sub(now_seconds).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(day: u64, rate: u64) -> Observation {
        Observation {
            utc_day: day,
            recorded_at: day * SECONDS_PER_DAY + 1,
            rate_observed_at: day * SECONDS_PER_DAY + 1,
            xdr_permyriad_per_icp: rate,
        }
    }

    #[test]
    fn formula_examples_and_independent_rounding() {
        assert_eq!(calculate_prices(10, 10).unwrap(), Price::BASE);
        assert_eq!(
            calculate_prices(10, 20).unwrap(),
            Price {
                account_icp: 5,
                global_icp: 50
            }
        );
        assert_eq!(
            calculate_prices(10, 50).unwrap(),
            Price {
                account_icp: 2,
                global_icp: 20
            }
        );
        let rounded = calculate_prices(10, 16).unwrap();
        assert_eq!(rounded.account_icp, 7);
        assert_eq!(rounded.global_icp, 63);
        assert_ne!(rounded.global_icp, rounded.account_icp * 10);
    }

    #[test]
    fn higher_current_never_raises_price_and_zero_rejects() {
        let mut previous = u64::MAX;
        for current in 10..=100 {
            let price = calculate_price(STANDARD_BASE_ICP, 10, current).unwrap();
            assert!(price <= previous);
            previous = price;
        }
        assert!(calculate_price(10, 0, 1).is_err());
        assert!(calculate_price(10, 1, 0).is_err());
        assert!(calculate_price(u64::MAX, u64::MAX, 1).is_err());
    }

    #[test]
    fn utc_month_boundaries_and_freeze_are_exact() {
        let october_15_2026 = days_from_civil(2026, 10, 15) * SECONDS_PER_DAY + 123;
        let november = days_from_civil(2026, 11, 1) * SECONDS_PER_DAY;
        assert_eq!(next_month_start(october_15_2026), november);
        let mut state = PricingState::default();
        initialize(&mut state, october_15_2026);
        assert_eq!(state.next_effective_at, november);
        assert_eq!(state.next_freeze_at, november - 7 * SECONDS_PER_DAY);
    }

    #[test]
    fn freeze_is_immutable_and_final_week_moves_to_later_epoch() {
        let now = days_from_civil(2026, 10, 1) * SECONDS_PER_DAY;
        let mut state = PricingState::default();
        initialize(&mut state, now);
        let before = obs(utc_day(state.next_freeze_at) - 1, 20);
        let mut observations = vec![obs(utc_day(now), 10), before];
        let freeze = state.next_freeze_at;
        advance_epochs(&mut state, &observations, freeze);
        let frozen = state.next.unwrap();
        observations.push(obs(utc_day(state.next_freeze_at) + 1, 100));
        let before_effective = state.next_effective_at - 1;
        advance_epochs(&mut state, &observations, before_effective);
        assert_eq!(state.next, Some(frozen));
        let effective = state.next_effective_at;
        advance_epochs(&mut state, &observations, effective);
        assert_eq!(state.current, frozen);
        let following_freeze = state.next_freeze_at;
        advance_epochs(&mut state, &observations, following_freeze);
        assert_ne!(state.next, None);
    }

    #[test]
    fn stale_reference_carries_forward_but_fresh_calculates() {
        let now = days_from_civil(2026, 10, 1) * SECONDS_PER_DAY;
        let mut stale = PricingState::default();
        initialize(&mut stale, now);
        let old = Observation {
            utc_day: utc_day(now),
            recorded_at: now + 1,
            rate_observed_at: stale.next_freeze_at - STALE_RATE_LIMIT_SECONDS - 1,
            xdr_permyriad_per_icp: 10,
        };
        let stale_freeze = stale.next_freeze_at;
        advance_epochs(&mut stale, &[old], stale_freeze);
        assert_eq!(stale.next, Some(stale.current));
        assert!(stale.next_carried_forward_due_to_stale_rate);

        let mut fresh = PricingState::default();
        initialize(&mut fresh, now);
        let observations = [
            obs(utc_day(now), 10),
            Observation {
                utc_day: utc_day(fresh.next_freeze_at) - 1,
                recorded_at: fresh.next_freeze_at - 1,
                rate_observed_at: fresh.next_freeze_at - 1,
                xdr_permyriad_per_icp: 20,
            },
        ];
        let fresh_freeze = fresh.next_freeze_at;
        advance_epochs(&mut fresh, &observations, fresh_freeze);
        assert_eq!(fresh.next.unwrap().account_icp, 5);
        assert!(!fresh.next_carried_forward_due_to_stale_rate);
    }

    #[test]
    fn rolling_window_is_exactly_1461_utc_days() {
        let today = 2_000;
        let now = today * SECONDS_PER_DAY + 99;
        assert_eq!(oldest_retained_day(now), today - 1_460);
        assert!(today - 1_460 >= oldest_retained_day(now));
        assert!(today - 1_461 < oldest_retained_day(now));
    }

    #[test]
    fn floor_latest_expiry_and_missed_days_need_no_synthetic_records() {
        let today = 2_000;
        let mut observations = vec![obs(today - 1_461, 1), obs(today - 100, 8), obs(today, 12)];
        observations.retain(|value| value.utc_day >= oldest_retained_day(today * SECONDS_PER_DAY));
        assert_eq!(observations.len(), 2, "missing days create no records");
        let (floor, latest) = reference_before(&observations, u64::MAX).unwrap();
        assert_eq!(floor.xdr_permyriad_per_icp, 8, "expired minimum falls out");
        assert_eq!(latest.xdr_permyriad_per_icp, 12);
        observations.push(obs(today + 1, 7));
        let (floor, latest) = reference_before(&observations, u64::MAX).unwrap();
        assert_eq!(
            floor.xdr_permyriad_per_icp, 7,
            "new lower observation becomes floor"
        );
        assert_eq!(latest.xdr_permyriad_per_icp, 7);
    }
}
