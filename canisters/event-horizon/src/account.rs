use candid::Principal;
use sha2::{Digest, Sha224};

/// Event Horizon's deliberately narrow numbered-subaccount convention.
/// Subaccount 0 is the all-zero/default subaccount; 1..=255 set only the last byte.
pub fn numbered_subaccount(number: u8) -> [u8; 32] {
    let mut subaccount = [0u8; 32];
    subaccount[31] = number;
    subaccount
}

/// CMC payment subaccount convention: principal byte length followed by principal bytes.
pub fn principal_to_subaccount(principal: Principal) -> [u8; 32] {
    let bytes = principal.as_slice();
    let mut out = [0u8; 32];
    out[0] = bytes.len() as u8;
    let len = bytes.len().min(31);
    out[1..1 + len].copy_from_slice(&bytes[..len]);
    out
}

/// Legacy ICP account identifier used by the ICP Ledger transfer operation.
pub fn account_identifier_bytes(owner: Principal, subaccount: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha224::new();
    hasher.update(b"\x0Aaccount-id");
    hasher.update(owner.as_slice());
    hasher.update(subaccount);
    let hash = hasher.finalize();
    let checksum = crc32fast::hash(&hash).to_be_bytes();
    let mut bytes = [0u8; 32];
    bytes[..4].copy_from_slice(&checksum);
    bytes[4..].copy_from_slice(&hash);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_subaccount_only_uses_last_byte() {
        assert_eq!(numbered_subaccount(0), [0u8; 32]);
        let mut expected = [0u8; 32];
        expected[31] = 7;
        assert_eq!(numbered_subaccount(7), expected);
    }

    #[test]
    fn account_identifier_matches_jupiter_reference_vector() {
        let owner = Principal::from_text("qaa6y-5yaaa-aaaaa-aaafa-cai").unwrap();
        assert_eq!(
            hex::encode(account_identifier_bytes(owner, numbered_subaccount(1))),
            "439a264f2ce4d3aeeb10b8ad65dc3610512ef3c6c4bc8c2985a15ce8cc2ce3c0"
        );
    }

    #[test]
    fn principal_subaccount_encodes_length_and_bytes() {
        let p = Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap();
        let out = principal_to_subaccount(p);
        assert_eq!(out[0] as usize, p.as_slice().len());
        assert_eq!(&out[1..1 + p.as_slice().len()], p.as_slice());
    }
}
