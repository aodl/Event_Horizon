use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::config::*;

#[derive(CandidType, Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PollingMode {
    #[default]
    ReserveProtection,
    Economy,
    Standard,
    Fast,
    VeryFast,
    Continuous,
}

impl PollingMode {
    pub const fn delay_seconds(self) -> Option<u64> {
        match self {
            Self::ReserveProtection => None,
            Self::Economy => Some(3600),
            Self::Standard => Some(600),
            Self::Fast => Some(120),
            Self::VeryFast => Some(10),
            Self::Continuous => Some(0),
        }
    }

    const fn exit_threshold(self) -> u128 {
        match self {
            Self::ReserveProtection => 0,
            Self::Economy => ECONOMY_EXIT,
            Self::Standard => STANDARD_EXIT,
            Self::Fast => FAST_EXIT,
            Self::VeryFast => VERY_FAST_EXIT,
            Self::Continuous => CONTINUOUS_EXIT,
        }
    }
}

fn fastest_enterable(balance: u128) -> PollingMode {
    if balance >= CONTINUOUS_ENTER {
        PollingMode::Continuous
    } else if balance >= VERY_FAST_ENTER {
        PollingMode::VeryFast
    } else if balance >= FAST_ENTER {
        PollingMode::Fast
    } else if balance >= STANDARD_ENTER {
        PollingMode::Standard
    } else if balance >= ECONOMY_ENTER {
        PollingMode::Economy
    } else {
        PollingMode::ReserveProtection
    }
}

/// Applies the fixed threshold table with hysteresis.
///
/// Funding increases may accelerate immediately. Decreases retain the current mode until its
/// lower exit threshold is crossed, then fall back as far as necessary.
pub fn next_mode(current: PollingMode, balance: u128) -> PollingMode {
    if balance < RESERVE_PROTECTION_CYCLES {
        return PollingMode::ReserveProtection;
    }

    let fastest = fastest_enterable(balance);
    if rank(fastest) > rank(current) {
        return fastest;
    }
    if current == PollingMode::ReserveProtection {
        return fastest;
    }
    if balance >= current.exit_threshold() {
        return current;
    }

    // Falling back uses enterability of lower modes, except Economy may remain down to its exit.
    match current {
        PollingMode::Continuous => fallback_from(PollingMode::VeryFast, balance),
        PollingMode::VeryFast => fallback_from(PollingMode::Fast, balance),
        PollingMode::Fast => fallback_from(PollingMode::Standard, balance),
        PollingMode::Standard => fallback_from(PollingMode::Economy, balance),
        PollingMode::Economy | PollingMode::ReserveProtection => PollingMode::ReserveProtection,
    }
}

fn fallback_from(max_mode: PollingMode, balance: u128) -> PollingMode {
    let candidate = fastest_enterable(balance);
    if rank(candidate) <= rank(max_mode) && candidate != PollingMode::ReserveProtection {
        return candidate;
    }
    // Hysteresis gaps: retain the highest lower mode whose exit threshold is still covered.
    for mode in [PollingMode::VeryFast, PollingMode::Fast, PollingMode::Standard, PollingMode::Economy] {
        if rank(mode) <= rank(max_mode) && balance >= mode.exit_threshold() {
            return mode;
        }
    }
    PollingMode::ReserveProtection
}

const fn rank(mode: PollingMode) -> u8 {
    match mode {
        PollingMode::ReserveProtection => 0,
        PollingMode::Economy => 1,
        PollingMode::Standard => 2,
        PollingMode::Fast => 3,
        PollingMode::VeryFast => 4,
        PollingMode::Continuous => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerates_directly_to_fastest_enterable_mode() {
        assert_eq!(next_mode(PollingMode::Economy, 100 * TRILLION), PollingMode::Continuous);
    }

    #[test]
    fn very_fast_holds_inside_hysteresis_band() {
        assert_eq!(next_mode(PollingMode::VeryFast, 20 * TRILLION), PollingMode::VeryFast);
        assert_eq!(next_mode(PollingMode::VeryFast, 15 * TRILLION), PollingMode::VeryFast);
    }

    #[test]
    fn very_fast_falls_once_exit_is_crossed() {
        assert_eq!(next_mode(PollingMode::VeryFast, 14 * TRILLION), PollingMode::Fast);
    }

    #[test]
    fn economy_survives_until_one_trillion() {
        assert_eq!(next_mode(PollingMode::Economy, TRILLION), PollingMode::Economy);
        assert_eq!(next_mode(PollingMode::Economy, TRILLION - 1), PollingMode::ReserveProtection);
    }

    #[test]
    fn reserve_requires_economy_entry_to_resume() {
        assert_eq!(next_mode(PollingMode::ReserveProtection, TRILLION + 500_000_000_000), PollingMode::ReserveProtection);
        assert_eq!(next_mode(PollingMode::ReserveProtection, 2 * TRILLION), PollingMode::Economy);
    }
}
