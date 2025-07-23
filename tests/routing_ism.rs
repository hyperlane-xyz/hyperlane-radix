use crate::common::Suite;
use hyperlane_radix::types::HyperlaneMessage;
use scrypto_test::prelude::*;

mod common;

fn create_routing_ism(
    suite: &mut Suite,
    domains: Vec<u32>,
    addresses: Vec<ComponentAddress>,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "RoutingIsm",
            "instantiate",
            manifest_args!(domains, addresses),
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

fn create_noop_ism(suite: &mut Suite) -> ComponentAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "NoopIsm",
            "instantiate",
            manifest_args!(),
        )
        .deposit_entire_worktop(suite.account.address)
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_commit_success();

    receipt.expect_commit_success().new_component_addresses()[0]
}

fn call_dummy_verify(
    suite: &mut Suite,
    component_address: ComponentAddress,
    origin: u32,
) -> TransactionReceipt {
    let raw_metadata: Vec<u8> = vec![];
    let message = HyperlaneMessage {
        version: 3,
        nonce: 0,
        origin,
        sender: Default::default(),
        destination: 0,
        recipient: Default::default(),
        body: vec![],
    };

    let raw_message: Vec<u8> = message.into();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            component_address,
            "verify",
            manifest_args!(raw_metadata, raw_message),
        )
        .build();

    suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    )
}

#[test]
fn test_create_empty_routing_ism() {
    let mut suite = common::setup();
    let receipt = create_routing_ism(&mut suite, vec![], vec![]);
    receipt.expect_commit_success();
}

#[test]
fn test_create_invalid_routing_ism() {
    let mut suite = common::setup();
    let receipt = create_routing_ism(&mut suite, vec![1], vec![]);
    assert!(format!("{:?}", receipt.expect_commit_failure())
        .contains("domains and ism array must have the same length"));
}

#[test]
fn test_domain_does_not_exist() {
    // Arrange
    let mut suite = common::setup();
    let receipt = create_routing_ism(&mut suite, vec![], vec![]);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];
    let _owner_badge = receipt.expect_commit_success().new_resource_addresses()[0];

    let receipt = call_dummy_verify(&mut suite, component_address, 17);

    // Assert error message
    assert!(format!("{:?}", receipt.expect_commit_failure()).contains("No ISM for route 17"));
}

#[test]
fn test_non_owner_can_not_update_route() {
    // Arrange
    let mut suite = common::setup();
    let receipt = create_routing_ism(&mut suite, vec![], vec![]);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];
    let _owner_badge = receipt.expect_commit_success().new_resource_addresses()[0];

    let domain: u32 = 1;
    let non_authorized_account = suite.dummy_accounts.get(0).unwrap().address;

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            component_address,
            "set_route",
            manifest_args!(domain, non_authorized_account),
        )
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    // Assert error message
    assert!(format!("{:?}", receipt.expect_commit_failure())
        .contains("SystemModuleError(AuthError(Unauthorized(Unauthorized"));
}

#[test]
fn test_add_new_route() {
    // Arrange
    let mut suite = common::setup();
    let receipt = create_routing_ism(&mut suite, vec![], vec![]);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];
    let owner_badge = receipt.expect_commit_success().new_resource_addresses()[0];

    let domain: u32 = 1;
    let ism_component = create_noop_ism(&mut suite);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(suite.account.address, owner_badge, dec!(1))
        .call_method(
            component_address,
            "set_route",
            manifest_args!(domain, ism_component),
        )
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    // Assert error message
    receipt.expect_commit_success();
    let success_receipt = call_dummy_verify(&mut suite, component_address, 1);
    success_receipt.expect_commit_success();
}

#[test]
fn test_remove_route() {
    // Arrange
    let mut suite = common::setup();
    let domain: u32 = 1;
    let ism_component = create_noop_ism(&mut suite);
    let receipt = create_routing_ism(&mut suite, vec![domain], vec![ism_component]);
    receipt.expect_commit_success();

    let component_address = receipt.expect_commit_success().new_component_addresses()[0];
    let owner_badge = receipt.expect_commit_success().new_resource_addresses()[0];

    let success_receipt = call_dummy_verify(&mut suite, component_address, 1);
    success_receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(suite.account.address, owner_badge, dec!(1))
        .call_method(component_address, "remove_route", manifest_args!(domain))
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    // Assert error message
    receipt.expect_commit_success();
    let failure_receipt = call_dummy_verify(&mut suite, component_address, 1);
    assert!(format!("{:?}", failure_receipt.expect_commit_failure()).contains("No ISM for route 1"));
}
