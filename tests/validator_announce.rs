use crate::common::Suite;
use hyperlane_radix::types::EthAddress;
use scrypto_test::prelude::*;

mod common;

fn create_mailbox(
    suite: &mut Suite,
    local_domain: u32,
) -> (
    TransactionReceipt,
    Option<ComponentAddress>,
    Option<ResourceAddress>,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "Mailbox",
            "instantiate",
            manifest_args!(local_domain),
        )
        .deposit_entire_worktop(suite.account.address)
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    match receipt.result.clone() {
        TransactionResult::Commit(data) => (
            receipt,
            Some(data.new_component_addresses()[0]),
            Some(data.new_resource_addresses()[0]),
        ),
        TransactionResult::Abort(_) => (receipt, None, None),
        TransactionResult::Reject(_) => (receipt, None, None),
    }
}

fn announce(
    suite: &mut Suite,
    validator_announce: ComponentAddress,
    address: EthAddress,
    storage_location: String,
    signature: Vec<u8>,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            validator_announce,
            "announce",
            manifest_args!(address, storage_location, signature),
        )
        .build();

    suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    )
}

fn create_validator_announce(suite: &mut Suite, mailbox: ComponentAddress) -> ComponentAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "ValidatorAnnounce",
            "instantiate",
            manifest_args!(mailbox),
        )
        .deposit_entire_worktop(suite.account.address)
        .build();

    suite
        .ledger
        .execute_manifest(
            manifest,
            vec![NonFungibleGlobalId::from_public_key(
                &suite.account.public_key,
            )],
        )
        .expect_commit_success()
        .new_component_addresses()[0]
}

#[test]
fn test_announce() {
    // Arrange
    let validator: [u8; 20] = hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
        .unwrap()
        .try_into()
        .unwrap();
    let signature = hex::decode("74b2bc086b9b7aa07e4e2c678381930ce16d91008dcbfebb2459aab1609b5fe169ac4a85dbcd598dee36e152014c8963932a83c5c4fcf4285b704860c592eb521b").unwrap();
    let storage_location = "s3://test-storage-location".to_string();

    let mut suite = common::setup();
    let (_, mailbox, _) = create_mailbox(&mut suite, 1337);
    let mailbox = mailbox.unwrap();
    let validator_announce = create_validator_announce(&mut suite, mailbox);

    // Act
    let receipt = announce(
        &mut suite,
        validator_announce,
        validator.into(),
        storage_location,
        signature,
    );

    // Assert
    let call_result = receipt.expect_commit_success().outcome.expect_success();
    call_result[1].expect_return_value(&true)
}

#[test]
fn test_announced_storage_locations() {
    // Arrange
    let validator: [u8; 20] = hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
        .unwrap()
        .try_into()
        .unwrap();
    let signature = hex::decode("74b2bc086b9b7aa07e4e2c678381930ce16d91008dcbfebb2459aab1609b5fe169ac4a85dbcd598dee36e152014c8963932a83c5c4fcf4285b704860c592eb521b").unwrap();
    let expected_storage_location = "s3://test-storage-location".to_string();

    let mut suite = common::setup();
    let (_, mailbox, _) = create_mailbox(&mut suite, 1337);
    let mailbox = mailbox.unwrap();
    let validator_announce = create_validator_announce(&mut suite, mailbox);

    // Act
    let receipt = announce(
        &mut suite,
        validator_announce,
        validator.into(),
        expected_storage_location.clone(),
        signature,
    );

    // Assert
    let call_result = receipt.expect_commit_success().outcome.expect_success();
    call_result[1].expect_return_value(&true);

    // Assert state after announcement
    let storage_locations: Vec<Vec<String>> = suite.call_method_success(
        validator_announce,
        "get_announced_storage_locations",
        manifest_args!(vec![validator]),
    );

    assert_eq!(storage_locations[0][0], expected_storage_location)
}
