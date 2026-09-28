use candid::{Nat, Principal};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecimalAmount(String);

impl DecimalAmount {
    pub fn to_units(&self, decimals: u8) -> Result<Nat, MemoParseError> {
        let (whole, fraction) = self.0.split_once('.').map_or((self.0.as_str(), ""), |v| v);
        if fraction.len() > decimals as usize {
            return Err(MemoParseError::TooManyFractionalDigits);
        }
        let mut digits = String::with_capacity(whole.len() + decimals as usize);
        digits.push_str(whole);
        digits.push_str(fraction);
        digits.extend(std::iter::repeat_n('0', decimals as usize - fraction.len()));
        let value = Nat::from_str(&digits).map_err(|_| MemoParseError::InvalidAmount)?;
        if value == 0u8 {
            return Err(MemoParseError::ZeroAmount);
        }
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionDeclaration {
    Global {
        subscriber: Principal,
    },
    Account {
        subscriber: Principal,
        target: crate::subscription::WatchTarget,
        minimum: Option<DecimalAmount>,
    },
    Range {
        subscriber: Principal,
        neuron: bool,
        start: u64,
        end: u64,
        minimum: Option<DecimalAmount>,
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
    #[error("target must be a canonical unsigned 64-bit decimal")]
    InvalidSubaccount,
    #[error("range must have exactly two numbered subaccount endpoints")]
    InvalidRange,
    #[error("range end must be greater than range start")]
    ReversedRange,
    #[error("use the single-account form when both range endpoints are equal")]
    DegenerateRange,
    #[error("a range may contain at most 256 targets")]
    RangeTooWide,
    #[error("amount must be a canonical unsigned ASCII decimal")]
    InvalidAmount,
    #[error("amount has more fractional digits than the observed ledger supports")]
    TooManyFractionalDigits,
    #[error("explicit amount must be greater than zero raw units")]
    ZeroAmount,
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

fn parse_decimal(text: &str) -> Result<DecimalAmount, MemoParseError> {
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
    if matches!(frac, Some("")) || frac.is_some_and(|f| !f.bytes().all(|b| b.is_ascii_digit())) {
        return Err(MemoParseError::InvalidAmount);
    }
    Ok(DecimalAmount(text.to_string()))
}

pub fn parse_canonical_u64(text: &str) -> Result<u64, MemoParseError> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(MemoParseError::InvalidSubaccount);
    }
    text.parse().map_err(|_| MemoParseError::InvalidSubaccount)
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

    let minimum = amount_text.map(parse_decimal).transpose()?;
    let subscriber = parse_principal(principal_text)?;
    let (neuron, target_text) = match subaccount_text.strip_prefix('n') {
        Some(value) => (true, value),
        None => (false, subaccount_text),
    };
    if target_text.contains('-') {
        let mut endpoints = target_text.split('-');
        let start = endpoints.next().ok_or(MemoParseError::InvalidRange)?;
        let end = endpoints.next().ok_or(MemoParseError::InvalidRange)?;
        if endpoints.next().is_some() || start.is_empty() || end.is_empty() {
            return Err(MemoParseError::InvalidRange);
        }
        let start = parse_canonical_u64(start)?;
        let end = parse_canonical_u64(end)?;
        if start > end {
            return Err(MemoParseError::ReversedRange);
        }
        if start == end {
            return Err(MemoParseError::DegenerateRange);
        }
        if end - start > 255 {
            return Err(MemoParseError::RangeTooWide);
        }
        Ok(SubscriptionDeclaration::Range {
            subscriber,
            neuron,
            start,
            end,
            minimum,
        })
    } else {
        Ok(SubscriptionDeclaration::Account {
            subscriber,
            target: if neuron {
                crate::subscription::WatchTarget::NeuronNonce(parse_canonical_u64(target_text)?)
            } else {
                crate::subscription::WatchTarget::Subaccount(parse_canonical_u64(target_text)?)
            },
            minimum,
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
                target: crate::subscription::WatchTarget::Subaccount(7),
                minimum: Some(parse_decimal("0.01").unwrap()),
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
                target: crate::subscription::WatchTarget::Subaccount(7),
                minimum: None,
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
        for (s, units) in [
            ("0.01", 1_000_000u64),
            ("0.1", 10_000_000),
            ("0.10", 10_000_000),
            ("1", 100_000_000),
            ("1.00", 100_000_000),
            ("10.25", 1_025_000_000),
        ] {
            assert_eq!(
                parse_decimal(s).unwrap().to_units(8).unwrap(),
                Nat::from(units),
                "{s}"
            );
        }
    }

    #[test]
    fn converts_generic_precisions_without_u64_limits() {
        for (text, decimals, units) in [
            ("1", 0, "1"),
            ("1", 2, "100"),
            ("0.01", 2, "1"),
            ("0.00000001", 8, "1"),
            ("10.12345678", 8, "1012345678"),
            ("0.000000000000000001", 18, "1"),
        ] {
            assert_eq!(
                parse_decimal(text).unwrap().to_units(decimals).unwrap(),
                Nat::from_str(units).unwrap()
            );
        }
        assert_eq!(
            parse_decimal("0.001").unwrap().to_units(2),
            Err(MemoParseError::TooManyFractionalDigits)
        );
        assert_eq!(
            parse_decimal("0").unwrap().to_units(0),
            Err(MemoParseError::ZeroAmount)
        );
    }

    #[test]
    fn rejects_confusing_or_over_precise_amounts() {
        for s in [
            "", ".1", "0.001", "+1", "-1", "1e2", "1.", "01", "01.0", "01..0",
        ] {
            assert!(
                parse_decimal(s).is_err() || parse_decimal(s).unwrap().to_units(2).is_err(),
                "{s}"
            );
        }
    }

    #[test]
    fn explicit_threshold_still_rejects_below_natural_floor() {
        let value = parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.7:0.00").unwrap();
        let SubscriptionDeclaration::Account {
            minimum: Some(value),
            ..
        } = value
        else {
            panic!()
        };
        assert_eq!(value.to_units(2).unwrap_err(), MemoParseError::ZeroAmount);
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
        for (memo, start, end, minimum) in [
            ("r5m5ydiaaaaaaaaqanaacai.7-18", 7, 18, None),
            (
                "r5m5ydiaaaaaaaaqanaacai.7-18:0.01",
                7,
                18,
                Some(parse_decimal("0.01").unwrap()),
            ),
            ("r5m5ydiaaaaaaaaqanaacai.0-255", 0, 255, None),
            (
                "r5m5ydiaaaaaaaaqanaacai.0-255:1",
                0,
                255,
                Some(parse_decimal("1").unwrap()),
            ),
        ] {
            assert_eq!(
                parse_subscription_memo(memo.as_bytes()).unwrap(),
                SubscriptionDeclaration::Range {
                    subscriber,
                    neuron: false,
                    start,
                    end,
                    minimum,
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
            "1000-1256",
            "7-",
            "-18",
            "7--18",
            "7-18-20",
            "7-18:",
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
                start: 7,
                end: 18,
                minimum: Some(_),
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
    fn accepts_full_u64_and_neuron_ranges() {
        assert!(parse_subscription_memo(b"r5m5ydiaaaaaaaaqanaacai.18446744073709551615").is_ok());
        for memo in [
            b"r5m5ydiaaaaaaaaqanaacai.1000-1255".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai.n0-255",
            b"r5m5ydiaaaaaaaaqanaacai.n1000000-1000255",
        ] {
            assert!(parse_subscription_memo(memo).is_ok(), "{memo:?}");
        }
        for memo in [
            b"r5m5ydiaaaaaaaaqanaacai.1000-1256".as_slice(),
            b"r5m5ydiaaaaaaaaqanaacai.n1000000-1000256",
            b"r5m5ydiaaaaaaaaqanaacai.N1",
            b"r5m5ydiaaaaaaaaqanaacai.n01",
        ] {
            assert!(parse_subscription_memo(memo).is_err(), "{memo:?}");
        }
    }
}
