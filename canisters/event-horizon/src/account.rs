use candid::Principal;
use sha2::{Digest, Sha224, Sha256};

/// Event Horizon's numeric-subaccount convention: 24 zero bytes followed by the
/// unsigned 64-bit number in big-endian order.
pub fn numbered_subaccount(number: u64) -> [u8; 32] {
    let mut subaccount = [0u8; 32];
    subaccount[24..].copy_from_slice(&number.to_be_bytes());
    subaccount
}

/// Standard NNS/SNS neuron staking-subaccount derivation.
///
/// Source: dfinity/ic, rs/nervous_system/common/src/ledger.rs,
/// `compute_neuron_staking_subaccount_bytes`.
pub fn neuron_staking_subaccount(controller: Principal, nonce: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x0c]);
    hasher.update(b"neuron-stake");
    hasher.update(controller.as_slice());
    hasher.update(nonce.to_be_bytes());
    hasher.finalize().into()
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
    fn numbered_subaccount_is_big_endian_u64() {
        for number in [0, 1, 7, 255, 256, 65_535, 1_000_000, u64::MAX] {
            let actual = numbered_subaccount(number);
            assert_eq!(&actual[..24], &[0; 24]);
            assert_eq!(&actual[24..], &number.to_be_bytes());
        }
        assert_eq!(&numbered_subaccount(256)[..30], &[0; 30]);
        assert_eq!(&numbered_subaccount(256)[30..], &[0x01, 0x00]);
    }

    #[test]
    fn neuron_staking_subaccount_matches_dfinity_vectors() {
        let controller = Principal::from_text("r5m5y-diaaa-aaaaa-qanaa-cai").unwrap();
        for (nonce, expected) in [
            (
                0,
                "536840bb85686ffd063f0d09da7accec98a3b15f6cd683707ad81294fcc2efd1",
            ),
            (
                42,
                "c7fe59656f136313d31b7b5d0cbd57a248f741b2e2831771d90a0d28a79e7ccf",
            ),
            (
                1_000_000,
                "cea791a58db37863afde77b275ce6b74661bc5ffb3384a2c4e11a9292e02184b",
            ),
        ] {
            assert_eq!(
                hex::encode(neuron_staking_subaccount(controller, nonce)),
                expected
            );
        }
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
