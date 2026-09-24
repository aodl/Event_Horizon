use crate::config::MIN_TRIGGER_E8S;
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
    Range {
        subscriber: Principal,
        start_subaccount: u8,
        end_subaccount: u8,
        minimum_e8s: u64,
    },
}

impl SubscriptionDeclaration {
    pub fn subscriber(&self) -> Principal {
        match self {
            Self::Global { subscriber }
            | Self::Account { subscriber, .. }
            | Self::Range { subscriber, .. } => *subscriber,
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
    #[error("range must have exactly two numbered subaccount endpoints")]
    InvalidRange,
    #[error("range end must be greater than range start")]
    ReversedRange,
    #[error("use the single-account form when both range endpoints are equal")]
    DegenerateRange,
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

fn parse_numbered_subaccount(text: &str) -> Result<u8, MemoParseError> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(MemoParseError::InvalidSubaccount);
    }
    let value: u16 = text
        .parse()
        .map_err(|_| MemoParseError::InvalidSubaccount)?;
    u8::try_from(value).map_err(|_| MemoParseError::InvalidSubaccount)
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
    let subscriber = parse_principal(principal_text)?;
    if subaccount_text.contains('-') {
        let mut endpoints = subaccount_text.split('-');
        let start = endpoints.next().ok_or(MemoParseError::InvalidRange)?;
        let end = endpoints.next().ok_or(MemoParseError::InvalidRange)?;
        if endpoints.next().is_some() || start.is_empty() || end.is_empty() {
            return Err(MemoParseError::InvalidRange);
        }
        let start_subaccount = parse_numbered_subaccount(start)?;
        let end_subaccount = parse_numbered_subaccount(end)?;
        if start_subaccount > end_subaccount {
            return Err(MemoParseError::ReversedRange);
        }
        if start_subaccount == end_subaccount {
            return Err(MemoParseError::DegenerateRange);
        }
        Ok(SubscriptionDeclaration::Range {
            subscriber,
            start_subaccount,
            end_subaccount,
            minimum_e8s,
        })
    } else {
        Ok(SubscriptionDeclaration::Account {
            subscriber,
            numbered_subaccount: parse_numbered_subaccount(subaccount_text)?,
            minimum_e8s,
        })
    }
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
    fn parses_inclusive_ranges_with_the_existing_threshold_grammar() {
        let subscriber = Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap();
        for (memo, start, end, minimum_e8s) in [
            ("r5m5ydiaaaaaaaaqanaacai.7-18", 7, 18, 0),
            ("r5m5ydiaaaaaaaaqanaacai.7-18:0.01", 7, 18, 1_000_000),
            ("r5m5ydiaaaaaaaaqanaacai.0-255", 0, 255, 0),
            ("r5m5ydiaaaaaaaaqanaacai.0-255:1", 0, 255, 100_000_000),
        ] {
            assert_eq!(
                parse_subscription_memo(memo.as_bytes()).unwrap(),
                SubscriptionDeclaration::Range {
                    subscriber,
                    start_subaccount: start,
                    end_subaccount: end,
                    minimum_e8s,
                },
                "{memo}"
            );
        }
    }

    #[test]
    fn rejects_invalid_ranges_without_normalizing_them() {
        let principal = "r5m5ydiaaaaaaaaqanaacai";
        for suffix in [
            "10-9",
            "-1-5",
            "1-256",
            "256-257",
            "7-",
            "-18",
            "7--18",
            "7-18-20",
            "7-18:",
            "7-18:0",
            "7-18:0.001",
            "007-18",
            "7-018",
        ] {
            let memo = format!("{principal}.{suffix}");
            assert!(parse_subscription_memo(memo.as_bytes()).is_err(), "{memo}");
        }
        assert_eq!(
            parse_subscription_memo(format!("{principal}.10-9").as_bytes()).unwrap_err(),
            MemoParseError::ReversedRange
        );
        assert_eq!(
            parse_subscription_memo(format!("{principal}.7-7").as_bytes()).unwrap_err(),
            MemoParseError::DegenerateRange
        );
    }

    #[test]
    fn principal_hyphens_and_decimal_points_are_unambiguous() {
        let parsed = parse_subscription_memo(b"r5m5y-diaaa-aaaaa-qanaa-cai.7-18:10.25").unwrap();
        assert!(matches!(
            parsed,
            SubscriptionDeclaration::Range {
                start_subaccount: 7,
                end_subaccount: 18,
                minimum_e8s: 1_025_000_000,
                ..
            }
        ));
    }

    #[test]
    fn integer_subaccounts_are_canonical_decimal() {
        for memo in [
            b"r5m5ydiaaaaaaaaqanaacai.007".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai.+7".as_slice(),
        ] {
            assert_eq!(
                parse_subscription_memo(memo).unwrap_err(),
                MemoParseError::InvalidSubaccount
            );
        }
    }

    #[test]
    fn rejects_subaccount_above_255() {
        assert_eq!(
            parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.256:1").unwrap_err(),
            MemoParseError::InvalidSubaccount
        );
    }
}
