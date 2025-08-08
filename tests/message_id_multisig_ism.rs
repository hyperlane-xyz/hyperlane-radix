use crate::common::Suite;
use hyperlane_radix::types::metadata::MultisigIsmMessageIdMetadata;
use hyperlane_radix::types::EthAddress;
use scrypto_test::prelude::*;

mod common;

fn create_message_id_multisig_ism(
    suite: &mut Suite,
    validators: Vec<EthAddress>,
    threshold: usize,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "MessageIdMultisigIsm",
            "instantiate",
            manifest_args!(validators, threshold),
        )
        .deposit_entire_worktop(suite.account.address)
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt
}

fn metadata_to_vec(msg: &MultisigIsmMessageIdMetadata) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(msg.origin_merkle_tree_hook.as_ref());
    bytes.extend_from_slice(msg.merkle_root.as_ref());
    bytes.extend_from_slice(msg.merkle_index.to_be_bytes().as_ref());

    msg.validator_signatures.iter().for_each(|signature| {
        bytes.extend_from_slice(signature.as_ref());
    });

    bytes
}

fn verify(
    suite: &mut Suite,
    component_address: ComponentAddress,
    metadata: Vec<u8>,
    message: Vec<u8>,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            component_address,
            "verify",
            manifest_args!(metadata, message),
        )
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt
}

#[test]
fn test_valid_relayer_message() {
    // Arrange
    let message = hex::decode("0300000000000000010000000000000000000000007ff2bf58c38a41ad7c9cbc14e780e8a7edbbd48d00002105000000000000000000000000811808dd29ba8b0fc6c0ec0b5537035e5974516248656c6c6f21").unwrap();

    let validator: [u8; 20] = hex::decode("03c842db86a6a3e524d4a6615390c1ea8e2b9541")
        .unwrap()
        .try_into()
        .unwrap();

    let metadata: MultisigIsmMessageIdMetadata = MultisigIsmMessageIdMetadata {
        origin_merkle_tree_hook: Hash::from_str("00000000000000000000000048e6c30b97748d1e2e03bf3e9fbe3890ca5f8cca").unwrap().into(),
        merkle_root: Hash::from_str("db278688f4f929bb03c76e57866ca41290dc63a1069752507fe6d20f307f1538").unwrap().into(),
        merkle_index: 0,
        validator_signatures: vec![Secp256k1Signature::from_str("3aeb79d0e542b8363144fe5286b1f8f6392d75d3220d9eca0ac20bb0cd41236d0e5eafcce7e6105cc282caa68ce73d095f80f111cde5a8f13e80bd8ddb0b91271b").unwrap()],
    };

    let mut suite = common::setup();
    let receipt = create_message_id_multisig_ism(&mut suite, vec![validator.into()], 1);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];

    // Act
    let receipt = verify(
        &mut suite,
        component_address,
        metadata_to_vec(&metadata),
        message,
    );

    // Assert
    let call_result = receipt.expect_commit_success().outcome.expect_success();
    call_result[1].expect_return_value(&true)
}

#[test]
fn test_invalid_relayer_message() {
    // Arrange
    let message = hex::decode("0300000000000000010000000000000000000000007ff2bf58c38a41ad7c9cbc14e780e8a7edbbd48d00002105000000000000000000000000811808dd29ba8b0fc6c0ec0b5537035e5974516248656c6c6f21").unwrap();

    let validator: [u8; 20] = hex::decode("03c842db86a6a3e524d4a6615390c1ea8e2b9541")
        .unwrap()
        .try_into()
        .unwrap();

    let metadata: MultisigIsmMessageIdMetadata = MultisigIsmMessageIdMetadata {
        origin_merkle_tree_hook: Hash::from_str("00000000000000000000000048e6c30b97748d1e2e03bf3e9fbe3890ca5f8cca").unwrap().into(),
        // Turned one the first character 'd' to an 'e' changing the message digest
        merkle_root: Hash::from_str("eb278688f4f929bb03c76e57866ca41290dc63a1069752507fe6d20f307f1538").unwrap().into(),
        merkle_index: 0,
        validator_signatures: vec![Secp256k1Signature::from_str("3aeb79d0e542b8363144fe5286b1f8f6392d75d3220d9eca0ac20bb0cd41236d0e5eafcce7e6105cc282caa68ce73d095f80f111cde5a8f13e80bd8ddb0b91271b").unwrap()],
    };

    let mut suite = common::setup();
    let receipt = create_message_id_multisig_ism(&mut suite, vec![validator.into()], 1);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];

    // Act
    let receipt = verify(
        &mut suite,
        component_address,
        metadata_to_vec(&metadata),
        message,
    );

    // Assert
    assert!(format!("{:?}", receipt.expect_commit_failure())
        .contains("Multisig: threshold not reached"));
}
