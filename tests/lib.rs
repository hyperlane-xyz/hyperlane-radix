use scrypto_test::prelude::*;

use hyperlane_radix::{types::recover_eth_address, *};

use crate::types::HyperlaneMessage;

#[test]
fn test_recover_address() {
    let validator =
        Hash::from_str("00000000000000000000000003c842db86a6a3e524d4a6615390c1ea8e2b9541").unwrap();

    let signature = Secp256k1Signature::from_str("3aeb79d0e542b8363144fe5286b1f8f6392d75d3220d9eca0ac20bb0cd41236d0e5eafcce7e6105cc282caa68ce73d095f80f111cde5a8f13e80bd8ddb0b91271b").unwrap();
    let domain = 1u32;
    let merkle_tree_hook =
        Hash::from_str("00000000000000000000000048e6c30b97748d1e2e03bf3e9fbe3890ca5f8cca").unwrap();
    let root =
        Hash::from_str("db278688f4f929bb03c76e57866ca41290dc63a1069752507fe6d20f307f1538").unwrap();
    let index = 0u32;
    let message_id =
        Hash::from_str("f0a76f8d108fed3fd57858fc879017f61329dcea7fe170ac4ffa8e938bcdad30").unwrap();

    let digest = HyperlaneMessage::message_digest(
        domain,
        message_id.into(),
        merkle_tree_hook.into(),
        root.into(),
        index,
    );

    let result = recover_eth_address(digest.as_slice(), &signature);

    assert_eq!(result, validator.into());
}
