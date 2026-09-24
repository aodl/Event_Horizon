use candid::{CandidType, Deserialize};
use serde::Serialize;

use crate::config;

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct SurplusPolicyState {
    pub initialized: bool,
    pub epoch_started_at_seconds: u64,
    /// Lowest liquid-cycles balance observed by the hourly funding lane in this epoch.
    pub epoch_min_liquid_cycles: u128,
    pub diversion_level: u8,
}

impl SurplusPolicyState {
    pub fn diversion_percent(&self) -> u8 {
        self.diversion_level
            .saturating_mul(config::SURPLUS_STEP_PERCENT)
            .min(config::SURPLUS_MAX_PERCENT)
    }
}

/// Records one hourly observation and performs at most one seven-day transition.
pub fn observe(
    mut state: SurplusPolicyState,
    enabled: bool,
    now_seconds: u64,
    liquid_cycles: u128,
) -> SurplusPolicyState {
    if !enabled {
        return SurplusPolicyState::default();
    }
    if state.diversion_level > config::SURPLUS_MAX_LEVEL
        || (state.initialized && now_seconds < state.epoch_started_at_seconds)
    {
        state = SurplusPolicyState::default();
    }
    if !state.initialized {
        return SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: now_seconds,
            epoch_min_liquid_cycles: liquid_cycles,
            diversion_level: 0,
        };
    }

    state.epoch_min_liquid_cycles = state.epoch_min_liquid_cycles.min(liquid_cycles);
    if now_seconds.saturating_sub(state.epoch_started_at_seconds) < config::SURPLUS_EPOCH_SECONDS {
        return state;
    }

    state.diversion_level =
        if state.epoch_min_liquid_cycles >= config::SURPLUS_HEALTH_THRESHOLD_CYCLES {
            state
                .diversion_level
                .saturating_add(1)
                .min(config::SURPLUS_MAX_LEVEL)
        } else if state.epoch_min_liquid_cycles >= config::SURPLUS_EMERGENCY_THRESHOLD_CYCLES {
            state.diversion_level.saturating_sub(1)
        } else {
            0
        };
    // Missing time never manufactures epochs: restart from this observation.
    state.epoch_started_at_seconds = now_seconds;
    state.epoch_min_liquid_cycles = liquid_cycles;
    state
}

pub fn effective_percent(state: &SurplusPolicyState, enabled: bool, liquid_cycles: u128) -> u8 {
    if enabled && liquid_cycles >= config::SURPLUS_HEALTH_THRESHOLD_CYCLES {
        state.diversion_percent()
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEALTHY: u128 = config::SURPLUS_HEALTH_THRESHOLD_CYCLES;

    fn advance(state: SurplusPolicyState, minimum: u128) -> SurplusPolicyState {
        observe(state, true, config::SURPLUS_EPOCH_SECONDS, minimum)
    }

    #[test]
    fn sustained_health_reaches_95_percent_only_after_19_epochs() {
        let mut state = observe(SurplusPolicyState::default(), true, 0, HEALTHY);
        for week in 1..=19 {
            state = observe(state, true, week * config::SURPLUS_EPOCH_SECONDS, HEALTHY);
            assert_eq!(state.diversion_percent(), (week as u8) * 5);
        }
        state = observe(state, true, 20 * config::SURPLUS_EPOCH_SECONDS, HEALTHY);
        assert_eq!(state.diversion_percent(), 95);
    }

    #[test]
    fn weak_epoch_steps_down_once_and_emergency_resets() {
        let state = SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: 0,
            epoch_min_liquid_cycles: 125 * config::TRILLION,
            diversion_level: 13,
        };
        assert_eq!(
            advance(state, 125 * config::TRILLION).diversion_percent(),
            60
        );

        let emergency = SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: 0,
            epoch_min_liquid_cycles: 99 * config::TRILLION,
            diversion_level: 19,
        };
        assert_eq!(
            advance(emergency, 99 * config::TRILLION).diversion_percent(),
            0
        );
    }

    #[test]
    fn immediate_gate_and_disabled_state_are_conservative() {
        let state = SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: 0,
            epoch_min_liquid_cycles: HEALTHY,
            diversion_level: 19,
        };
        assert_eq!(effective_percent(&state, true, 149 * config::TRILLION), 0);
        assert_eq!(effective_percent(&state, true, HEALTHY), 95);
        assert_eq!(
            observe(state, false, 999, HEALTHY),
            SurplusPolicyState::default()
        );
    }

    #[test]
    fn long_downtime_applies_only_one_transition() {
        let state = observe(SurplusPolicyState::default(), true, 0, HEALTHY);
        let state = observe(state, true, 100 * config::SURPLUS_EPOCH_SECONDS, HEALTHY);
        assert_eq!(state.diversion_level, 1);
        assert_eq!(
            state.epoch_started_at_seconds,
            100 * config::SURPLUS_EPOCH_SECONDS
        );
    }

    #[test]
    fn invalid_level_fails_closed() {
        let state = SurplusPolicyState {
            initialized: true,
            epoch_started_at_seconds: 0,
            epoch_min_liquid_cycles: HEALTHY,
            diversion_level: 20,
        };
        let state = observe(state, true, 1, HEALTHY);
        assert_eq!(state.diversion_level, 0);
        assert!(state.initialized);
    }
}
