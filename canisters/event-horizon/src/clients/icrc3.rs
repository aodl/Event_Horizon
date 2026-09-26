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
pub enum LedgerEvent {
    Transfer {
        from: Account,
        to: Account,
        amount: Nat,
        memo: Option<Vec<u8>>,
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
pub fn decode_block(block: &ICRC3Value) -> Result<LedgerEvent, String> {
    let block = map(block).ok_or("block is not a map")?;
    let tx = match block.get("tx").and_then(map) {
        Some(tx) => tx,
        None if matches!(text(block.get("btype")), Some("1xfer" | "2xfer")) => {
            return Err("transfer block has no tx map".into())
        }
        None => return Ok(LedgerEvent::Other),
    };
    let btype = text(block.get("btype"));
    let transfer = matches!(btype, Some("1xfer" | "2xfer"))
        || (btype.is_none() && text(tx.get("op")) == Some("xfer"));
    if !transfer {
        return Ok(LedgerEvent::Other);
    }
    let from = account(tx.get("from"))?;
    let to = account(tx.get("to"))?;
    let amount = match tx.get("amt") {
        Some(ICRC3Value::Nat(n)) => n.clone(),
        _ => return Err("transfer amt is not Nat".into()),
    };
    let memo = match tx.get("memo") {
        None => None,
        Some(ICRC3Value::Blob(bytes)) => Some(bytes.to_vec()),
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
pub async fn supported_block_types(ledger: Principal) -> Result<Vec<SupportedBlockType>, String> {
    checked_call(ledger, "icrc3_supported_block_types", None)
        .await?
        .candid()
        .map_err(|e| format!("supported block types decode: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn account_value(owner: Principal, subaccount: Option<[u8; 32]>) -> ICRC3Value {
        let mut values = vec![ICRC3Value::Blob(owner.as_slice().to_vec().into())];
        if let Some(subaccount) = subaccount {
            values.push(ICRC3Value::Blob(subaccount.to_vec().into()));
        }
        ICRC3Value::Array(values)
    }
    fn transfer(btype: Option<&str>) -> ICRC3Value {
        let owner = Principal::from_slice(&[1, 2, 3]);
        let mut tx = BTreeMap::from([
            ("op".into(), ICRC3Value::Text("xfer".into())),
            ("from".into(), account_value(owner, None)),
            ("to".into(), account_value(owner, Some([0; 32]))),
            ("amt".into(), ICRC3Value::Nat(7u8.into())),
        ]);
        tx.insert("memo".into(), ICRC3Value::Blob(vec![9].into()));
        let mut block = BTreeMap::from([("tx".into(), ICRC3Value::Map(tx))]);
        if let Some(kind) = btype {
            block.insert("btype".into(), ICRC3Value::Text(kind.into()));
        }
        ICRC3Value::Map(block)
    }
    #[test]
    fn decodes_transfer_variants_and_normalizes_default_account() {
        for kind in [Some("1xfer"), Some("2xfer"), None] {
            let LedgerEvent::Transfer {
                from,
                to,
                amount,
                memo,
            } = decode_block(&transfer(kind)).unwrap()
            else {
                panic!()
            };
            assert_eq!(from, to);
            assert_eq!(amount, Nat::from(7u8));
            assert_eq!(memo, Some(vec![9]));
        }
    }
    #[test]
    fn malformed_identified_transfer_fails() {
        let mut block = transfer(Some("1xfer"));
        let ICRC3Value::Map(ref mut outer) = block else {
            unreachable!()
        };
        let ICRC3Value::Map(ref mut tx) = outer.get_mut("tx").unwrap() else {
            unreachable!()
        };
        tx.remove("from");
        assert!(decode_block(&block).is_err());
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
