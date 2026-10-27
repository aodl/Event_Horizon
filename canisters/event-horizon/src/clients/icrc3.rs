use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::call::Call;
use icrc_ledger_types::{
    icrc::generic_value::ICRC3Value,
    icrc1::account::Account,
    icrc3::blocks::{GetBlocksRequest, GetBlocksResult, SupportedBlockType},
};

use crate::config::RESERVE_PROTECTION_CYCLES;
#[cfg(feature = "debug_api")]
thread_local! { static GET_BLOCKS_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct SupportedStandard {
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerEvent<'a> {
    Transfer {
        from: Account,
        to: Account,
        amount: &'a Nat,
        memo: Option<&'a [u8]>,
    },
    Other,
}

fn map(value: &ICRC3Value) -> Option<&std::collections::BTreeMap<String, ICRC3Value>> {
    match value {
        ICRC3Value::Map(map) => Some(map),
        _ => None,
    }
}

fn text(value: Option<&ICRC3Value>) -> Option<&str> {
    match value {
        Some(ICRC3Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn account(value: Option<&ICRC3Value>) -> Result<Account, String> {
    let values = match value {
        Some(ICRC3Value::Array(values)) => values,
        _ => return Err("account is not an array".into()),
    };
    if !(1..=2).contains(&values.len()) {
        return Err("account array length must be 1 or 2".into());
    }
    let owner = match &values[0] {
        ICRC3Value::Blob(bytes) => {
            Principal::try_from_slice(bytes).map_err(|_| "invalid account owner principal")?
        }
        _ => return Err("account owner is not a blob".into()),
    };
    let subaccount = if values.len() == 2 {
        match &values[1] {
            ICRC3Value::Blob(bytes) => Some(
                bytes
                    .as_ref()
                    .try_into()
                    .map_err(|_| "subaccount must be 32 bytes")?,
            ),
            _ => return Err("subaccount is not a blob".into()),
        }
    } else {
        None
    };
    Ok(Account { owner, subaccount })
}

/// Decodes only the shallow fields Event Horizon needs; unknown values are never traversed.
pub fn decode_block(block: &ICRC3Value) -> Result<LedgerEvent<'_>, String> {
    let block = map(block).ok_or("block is not a map")?;
    let btype = match block.get("btype") {
        None => None,
        Some(ICRC3Value::Text(value)) => Some(value.as_str()),
        Some(_) => return Err("block btype is not Text".into()),
    };
    let tx = match block.get("tx").and_then(map) {
        Some(tx) => tx,
        None if matches!(btype, Some("1xfer" | "2xfer")) => {
            return Err("transfer block has no tx map".into())
        }
        None => return Ok(LedgerEvent::Other),
    };
    let transfer = matches!(btype, Some("1xfer" | "2xfer"))
        || (btype.is_none() && text(tx.get("op")) == Some("xfer"));
    if !transfer {
        return Ok(LedgerEvent::Other);
    }
    let from = account(tx.get("from"))?;
    let to = account(tx.get("to"))?;
    let amount = match tx.get("amt") {
        Some(ICRC3Value::Nat(n)) => n,
        _ => return Err("transfer amt is not Nat".into()),
    };
    let memo = match tx.get("memo") {
        None => None,
        Some(ICRC3Value::Blob(bytes)) => Some(bytes.as_ref()),
        Some(_) => return Err("transfer memo is not Blob".into()),
    };
    Ok(LedgerEvent::Transfer {
        from,
        to,
        amount,
        memo,
    })
}

async fn checked_call(
    ledger: Principal,
    method: &str,
    arg: Option<Vec<u8>>,
) -> Result<ic_cdk::call::Response, String> {
    let mut call = Call::bounded_wait(ledger, method);
    if let Some(ref arg) = arg {
        call = call.with_raw_args(arg);
    }
    if ic_cdk::api::canister_liquid_cycle_balance()
        < RESERVE_PROTECTION_CYCLES.saturating_add(call.get_cost())
    {
        return Err(crate::config::RESERVE_PROTECTION_ERROR.into());
    }
    call.await.map_err(|e| format!("{method} transport: {e:?}"))
}

pub async fn get_blocks(
    ledger: Principal,
    start: u64,
    length: u64,
) -> Result<GetBlocksResult, String> {
    #[cfg(feature = "debug_api")]
    GET_BLOCKS_CALLS.with(|calls| calls.set(calls.get() + 1));
    let request = vec![GetBlocksRequest {
        start: start.into(),
        length: length.into(),
    }];
    checked_call(
        ledger,
        "icrc3_get_blocks",
        Some(candid::encode_one(request).map_err(|e| e.to_string())?),
    )
    .await?
    .candid()
    .map_err(|e| format!("icrc3_get_blocks decode: {e:?}"))
}
#[cfg(feature = "debug_api")]
pub fn debug_get_blocks_calls() -> u64 {
    GET_BLOCKS_CALLS.with(std::cell::Cell::get)
}

pub async fn supported_standards(ledger: Principal) -> Result<Vec<SupportedStandard>, String> {
    checked_call(ledger, "icrc1_supported_standards", None)
        .await?
        .candid()
        .map_err(|e| format!("supported standards decode: {e:?}"))
}
pub async fn symbol(ledger: Principal) -> Result<String, String> {
    checked_call(ledger, "icrc1_symbol", None)
        .await?
        .candid()
        .map_err(|e| format!("symbol decode: {e:?}"))
}
pub async fn decimals(ledger: Principal) -> Result<u8, String> {
    checked_call(ledger, "icrc1_decimals", None)
        .await?
        .candid()
        .map_err(|e| format!("decimals decode: {e:?}"))
}
pub async fn minting_account(ledger: Principal) -> Result<Option<Account>, String> {
    checked_call(ledger, "icrc1_minting_account", None)
        .await?
        .candid()
        .map_err(|e| format!("icrc1_minting_account decode: {e:?}"))
}
pub async fn supported_block_types(ledger: Principal) -> Result<Vec<SupportedBlockType>, String> {
    checked_call(ledger, "icrc3_supported_block_types", None)
        .await?
        .candid()
        .map_err(|e| format!("supported block types decode: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, str::FromStr};

    fn account_value(owner: Principal, subaccount: Option<[u8; 32]>) -> ICRC3Value {
        let mut values = vec![ICRC3Value::Blob(owner.as_slice().to_vec().into())];
        if let Some(subaccount) = subaccount {
            values.push(ICRC3Value::Blob(subaccount.to_vec().into()));
        }
        ICRC3Value::Array(values)
    }

    fn transfer(btype: Option<ICRC3Value>, op: Option<&str>, amount: Nat) -> ICRC3Value {
        let owner = Principal::from_slice(&[1, 2, 3]);
        let mut tx = BTreeMap::from([
            ("from".into(), account_value(owner, None)),
            ("to".into(), account_value(owner, Some([0; 32]))),
            ("amt".into(), ICRC3Value::Nat(amount)),
        ]);
        if let Some(op) = op {
            tx.insert("op".into(), ICRC3Value::Text(op.into()));
        }
        tx.insert("memo".into(), ICRC3Value::Blob(vec![9].into()));
        let mut block = BTreeMap::from([("tx".into(), ICRC3Value::Map(tx))]);
        if let Some(kind) = btype {
            block.insert("btype".into(), kind);
        }
        ICRC3Value::Map(block)
    }

    fn text_btype(value: &str) -> Option<ICRC3Value> {
        Some(ICRC3Value::Text(value.into()))
    }

    fn decoded_transfer(block: &ICRC3Value) -> (Account, Account, Nat, Option<Vec<u8>>) {
        let LedgerEvent::Transfer {
            from,
            to,
            amount,
            memo,
        } = decode_block(block).unwrap()
        else {
            panic!("expected transfer")
        };
        (from, to, amount.clone(), memo.map(<[u8]>::to_vec))
    }

    #[test]
    fn decoded_amount_and_large_memo_borrow_the_validated_page() {
        let huge = Nat(Nat::from(1u8).0 << 32768usize);
        let mut block = transfer(text_btype("1xfer"), None, huge);
        tx_mut(&mut block).insert("memo".into(), ICRC3Value::Blob(vec![3; 65536].into()));
        let LedgerEvent::Transfer {
            amount,
            memo: Some(memo),
            ..
        } = decode_block(&block).unwrap()
        else {
            panic!("transfer");
        };
        let tx = map(map(&block).unwrap().get("tx").unwrap()).unwrap();
        let ICRC3Value::Nat(stored) = &tx["amt"] else {
            panic!("Nat");
        };
        let ICRC3Value::Blob(bytes) = &tx["memo"] else {
            panic!("Blob");
        };
        assert!(std::ptr::eq(amount, stored));
        assert_eq!(memo.as_ptr(), bytes.as_ptr());
        assert_eq!(memo.len(), 65536);
    }

    fn tx_mut(block: &mut ICRC3Value) -> &mut BTreeMap<String, ICRC3Value> {
        let ICRC3Value::Map(outer) = block else {
            unreachable!()
        };
        let ICRC3Value::Map(tx) = outer.get_mut("tx").unwrap() else {
            unreachable!()
        };
        tx
    }

    #[test]
    fn transfer_discriminator_distinguishes_absent_unknown_and_malformed_btype() {
        for (name, kind, op) in [
            ("1xfer without op", text_btype("1xfer"), None),
            ("1xfer with op", text_btype("1xfer"), Some("xfer")),
            ("2xfer without op", text_btype("2xfer"), None),
            ("2xfer with op", text_btype("2xfer"), Some("xfer")),
            ("absent legacy fallback", None, Some("xfer")),
        ] {
            let (from, to, amount, memo) = decoded_transfer(&transfer(kind, op, Nat::from(7u8)));
            assert_eq!(from, to);
            assert_eq!(amount, Nat::from(7u8));
            assert_eq!(memo, Some(vec![9]));
            assert!(!name.is_empty());
        }

        assert_eq!(
            decode_block(&transfer(text_btype("99future"), Some("xfer"), 7u8.into())).unwrap(),
            LedgerEvent::Other,
            "a present unknown btype is authoritative"
        );
        assert_eq!(
            decode_block(&transfer(None, None, 7u8.into())).unwrap(),
            LedgerEvent::Other
        );
        assert_eq!(
            decode_block(&ICRC3Value::Map(BTreeMap::from([(
                "btype".into(),
                ICRC3Value::Nat(1u8.into())
            )])))
            .unwrap_err(),
            "block btype is not Text"
        );
    }

    #[test]
    fn recognized_transfer_requires_a_valid_tx_and_required_fields() {
        for tx in [None, Some(ICRC3Value::Text("wrong".into()))] {
            let mut fields = BTreeMap::new();
            fields.insert("btype".into(), ICRC3Value::Text("1xfer".into()));
            if let Some(tx) = tx {
                fields.insert("tx".into(), tx);
            }
            assert_eq!(
                decode_block(&ICRC3Value::Map(fields)).unwrap_err(),
                "transfer block has no tx map"
            );
        }

        for field in ["from", "to", "amt"] {
            for replacement in [None, Some(ICRC3Value::Text("wrong".into()))] {
                let mut block = transfer(text_btype("2xfer"), None, 7u8.into());
                match replacement {
                    Some(value) => {
                        tx_mut(&mut block).insert(field.into(), value);
                    }
                    None => {
                        tx_mut(&mut block).remove(field);
                    }
                }
                assert!(decode_block(&block).is_err(), "{field}");
            }
        }

        let mut without_memo = transfer(text_btype("1xfer"), None, 7u8.into());
        tx_mut(&mut without_memo).remove("memo");
        assert_eq!(decoded_transfer(&without_memo).3, None);
        let mut wrong_memo = transfer(text_btype("1xfer"), None, 7u8.into());
        tx_mut(&mut wrong_memo).insert("memo".into(), ICRC3Value::Text("wrong".into()));
        assert_eq!(
            decode_block(&wrong_memo).unwrap_err(),
            "transfer memo is not Blob"
        );
    }

    #[test]
    fn account_shapes_and_subaccount_lengths_are_checked_and_normalized() {
        let owner = Principal::from_slice(&[1, 2, 3]);
        let (from, to, _, _) = decoded_transfer(&transfer(None, Some("xfer"), 1u8.into()));
        assert_eq!(from.subaccount, None);
        assert_eq!(to.subaccount, Some([0; 32]));
        assert_eq!(
            from, to,
            "absent and explicit zero subaccounts normalize equally"
        );

        for values in [
            vec![],
            vec![
                ICRC3Value::Blob(owner.as_slice().to_vec().into()),
                ICRC3Value::Blob(vec![0; 32].into()),
                ICRC3Value::Blob(vec![0; 32].into()),
            ],
        ] {
            assert_eq!(
                account(Some(&ICRC3Value::Array(values))).unwrap_err(),
                "account array length must be 1 or 2"
            );
        }
        assert_eq!(
            account(Some(&ICRC3Value::Array(vec![ICRC3Value::Nat(1u8.into())]))).unwrap_err(),
            "account owner is not a blob"
        );
        assert_eq!(
            account(Some(&ICRC3Value::Array(vec![ICRC3Value::Blob(
                vec![0; 30].into()
            )])))
            .unwrap_err(),
            "invalid account owner principal"
        );

        for (length, valid) in [(31, false), (32, true), (33, false)] {
            let value = ICRC3Value::Array(vec![
                ICRC3Value::Blob(owner.as_slice().to_vec().into()),
                ICRC3Value::Blob(vec![0; length].into()),
            ]);
            assert_eq!(account(Some(&value)).is_ok(), valid, "length={length}");
        }
    }

    #[test]
    fn transfer_amounts_remain_arbitrary_precision_and_extra_fields_are_harmless() {
        let above_u64 = Nat::from_str("18446744073709551616").unwrap();
        for amount in [Nat::from(0u8), Nat::from(u64::MAX), above_u64] {
            let mut block = transfer(text_btype("1xfer"), None, amount.clone());
            tx_mut(&mut block).insert("created_at_time".into(), ICRC3Value::Nat(123u8.into()));
            let ICRC3Value::Map(outer) = &mut block else {
                unreachable!()
            };
            outer.insert("phash".into(), ICRC3Value::Blob(vec![4, 5].into()));
            assert_eq!(decoded_transfer(&block).2, amount);
        }
    }

    #[test]
    fn unknown_type_is_other_without_recursive_walk() {
        assert_eq!(
            decode_block(&ICRC3Value::Map(BTreeMap::from([(
                "btype".into(),
                ICRC3Value::Text("99future".into())
            )])))
            .unwrap(),
            LedgerEvent::Other
        );
    }
}
