use crate::{account::numbered_subaccount, config::MIN_TRIGGER_E8S};
use candid::Principal;
use thiserror::Error;

/// `minimum_e8s == 0` is the internal sentinel for an unfiltered subscription:
/// every incoming transfer to the watched account is relevant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionDeclaration {
    Global {
        subscriber: Principal,
    },
    Account {
        subscriber: Principal,
        numbered_subaccount: u8,
        minimum_e8s: u64,
    },
}

impl SubscriptionDeclaration {
    pub fn subscriber(&self) -> Principal {
        match self {
            Self::Global { subscriber } | Self::Account { subscriber, .. } => *subscriber,
        }
    }

    pub fn subaccount(&self) -> Option<[u8; 32]> {
        match self {
            Self::Global { .. } => None,
            Self::Account {
                numbered_subaccount: number,
                ..
            } => Some(numbered_subaccount(*number)),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MemoParseError {
    #[error("memo must be non-empty ASCII")]
    NonAsciiOrEmpty,
    #[error("memo must have <principal>.<subaccount>[:<amount>] form")]
    InvalidShape,
    #[error("subscriber principal is invalid")]
    InvalidPrincipal,
    #[error("numbered subaccount must be 0..255")]
    InvalidSubaccount,
    #[error("amount must be a positive decimal ICP value with at most two fractional digits")]
    InvalidAmount,
    #[error("minimum explicit trigger amount is 0.01 ICP")]
    BelowMinimum,
}

fn with_group_separators(text: &str) -> String {
    if text.contains('-') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + text.len() / 5);
    for (i, c) in text.chars().enumerate() {
        if i > 0 && i % 5 == 0 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

fn parse_principal(text: &str) -> Result<Principal, MemoParseError> {
    if text.is_empty() || text.trim() != text {
        return Err(MemoParseError::InvalidPrincipal);
    }
    let normalized = with_group_separators(text);
    let principal =
        Principal::from_text(normalized).map_err(|_| MemoParseError::InvalidPrincipal)?;
    if principal == Principal::anonymous() || principal == Principal::management_canister() {
        return Err(MemoParseError::InvalidPrincipal);
    }
    Ok(principal)
}

/// Parses the deliberately human-readable ICP amount grammar into e8s.
/// Integers or 1-2 decimal places are accepted. No sign/exponent/.1 shorthand.
fn parse_amount_e8s(text: &str) -> Result<u64, MemoParseError> {
    if text.is_empty()
        || text.starts_with('+')
        || text.starts_with('-')
        || text.contains(['e', 'E'])
    {
        return Err(MemoParseError::InvalidAmount);
    }
    let mut parts = text.split('.');
    let whole = parts.next().ok_or(MemoParseError::InvalidAmount)?;
    let frac = parts.next();
    if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MemoParseError::InvalidAmount);
    }
    // For fractional values, require an explicit leading digit (therefore .1 is impossible).
    if whole.len() > 1 && whole.starts_with('0') {
        return Err(MemoParseError::InvalidAmount);
    }
    let whole: u64 = whole.parse().map_err(|_| MemoParseError::InvalidAmount)?;
    let frac_e8s = match frac {
        None => 0u64,
        Some(f) if (1..=2).contains(&f.len()) && f.bytes().all(|b| b.is_ascii_digit()) => {
            let n: u64 = f.parse().map_err(|_| MemoParseError::InvalidAmount)?;
            if f.len() == 1 {
                n * 10_000_000
            } else {
                n * 1_000_000
            }
        }
        Some(_) => return Err(MemoParseError::InvalidAmount),
    };
    whole
        .checked_mul(100_000_000)
        .and_then(|v| v.checked_add(frac_e8s))
        .ok_or(MemoParseError::InvalidAmount)
}

pub fn parse_subscription_memo(memo: &[u8]) -> Result<SubscriptionDeclaration, MemoParseError> {
    if memo.is_empty() || !memo.is_ascii() {
        return Err(MemoParseError::NonAsciiOrEmpty);
    }
    let text = std::str::from_utf8(memo).map_err(|_| MemoParseError::NonAsciiOrEmpty)?;
    if text.trim() != text {
        return Err(MemoParseError::InvalidShape);
    }

    // Principal text never contains '.', while an amount may (for example 0.01),
    // so split at the first separator rather than the last.
    let Some((principal_text, rest)) = text.split_once('.') else {
        return Ok(SubscriptionDeclaration::Global {
            subscriber: parse_principal(text)?,
        });
    };
    let (subaccount_text, amount_text) = match rest.split_once(':') {
        Some((subaccount, amount)) if !amount.contains(':') && !amount.is_empty() => {
            (subaccount, Some(amount))
        }
        Some(_) => return Err(MemoParseError::InvalidShape),
        None => (rest, None),
    };

    let subscriber = parse_principal(principal_text)?;
    if subaccount_text.is_empty() || !subaccount_text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MemoParseError::InvalidSubaccount);
    }
    let subaccount_u16: u16 = subaccount_text
        .parse()
        .map_err(|_| MemoParseError::InvalidSubaccount)?;
    let numbered_subaccount =
        u8::try_from(subaccount_u16).map_err(|_| MemoParseError::InvalidSubaccount)?;

    // Zero is reserved as an internal sentinel for "all incoming transfers" and can
    // only arise by omitting the threshold entirely. Explicit amounts retain the
    // natural 0.01 ICP floor.
    let minimum_e8s = match amount_text {
        None => 0,
        Some(amount) => {
            let parsed = parse_amount_e8s(amount)?;
            if parsed < MIN_TRIGGER_E8S {
                return Err(MemoParseError::BelowMinimum);
            }
            parsed
        }
    };

    Ok(SubscriptionDeclaration::Account {
        subscriber,
        numbered_subaccount,
        minimum_e8s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholded_example_parses_and_full_jupiter_memo_is_32_bytes() {
        let memo = b"r5m5ydiaaaaaaaaqanaacai.7:0.01";
        assert_eq!(memo.len(), 30);
        let parsed = parse_subscription_memo(memo).unwrap();
        assert_eq!(
            parsed,
            SubscriptionDeclaration::Account {
                subscriber: Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap(),
                numbered_subaccount: 7,
                minimum_e8s: 1_000_000,
            }
        );
        assert_eq!(
            format!("X.{}", std::str::from_utf8(memo).unwrap()).len(),
            32
        );
    }

    #[test]
    fn omitted_threshold_means_every_incoming_transfer() {
        let parsed = parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.7").unwrap();
        assert!(matches!(
            parsed,
            SubscriptionDeclaration::Account {
                numbered_subaccount: 7,
                minimum_e8s: 0,
                ..
            }
        ));
    }

    #[test]
    fn principal_only_is_global() {
        let parsed = parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai").unwrap();
        assert_eq!(
            parsed,
            SubscriptionDeclaration::Global {
                subscriber: Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap(),
            }
        );
        assert_eq!(format!("X.{}", "r5m5ydiaaaaaaaaqanaacai").len(), 25);
    }

    #[test]
    fn malformed_ambiguous_forms_reject() {
        for memo in [
            b"r5m5ydiaaaaaaaaqanaacai:".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai.".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai.7.extra".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai:0.01".as_slice(),
        ] {
            assert!(parse_subscription_memo(memo).is_err(), "{memo:?}");
        }
    }

    #[test]
    fn accepts_human_decimal_forms() {
        for (s, e8s) in [
            ("0.01", 1_000_000),
            ("0.1", 10_000_000),
            ("0.10", 10_000_000),
            ("1", 100_000_000),
            ("1.00", 100_000_000),
            ("10.25", 1_025_000_000),
        ] {
            assert_eq!(parse_amount_e8s(s).unwrap(), e8s, "{s}");
        }
    }

    #[test]
    fn rejects_confusing_or_over_precise_amounts() {
        for s in [
            "", ".1", "0.001", "+1", "-1", "1e2", "1.", "01", "01.0", "01..0",
        ] {
            assert!(parse_amount_e8s(s).is_err(), "{s}");
        }
    }

    #[test]
    fn explicit_threshold_still_rejects_below_natural_floor() {
        let err = parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.7:0.00").unwrap_err();
        assert_eq!(err, MemoParseError::BelowMinimum);
    }

    #[test]
    fn colon_without_amount_is_invalid_not_unfiltered() {
        assert_eq!(
            parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.7:").unwrap_err(),
            MemoParseError::InvalidShape
        );
    }

    #[test]
    fn accepts_hyphenated_principal_too() {
        let parsed = parse_subscription_memo(b"r5m5y-diaaa-aaaaa-qanaa-cai.0:1").unwrap();
        assert_eq!(parsed.subscriber().to_text(), "r5m5y-diaaa-aaaaa-qanaa-cai");
    }

    #[test]
    fn rejects_subaccount_above_255() {
        assert_eq!(
            parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.256:1").unwrap_err(),
            MemoParseError::InvalidSubaccount
        );
    }
}
