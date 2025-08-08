use crate::common::{hex_str_to_bytes32, Suite};
use hyperlane_radix::types::metadata::StandardHookMetadata;
use hyperlane_radix::types::Bytes32;
use scrypto_test::prelude::*;

mod common;

fn create_mailbox(suite: &mut Suite, local_domain: u32) -> (ComponentAddress, ResourceAddress) {
    let result = suite.instantiate_blueprint("Mailbox", manifest_args!(local_domain));
    result.0.expect_commit_success();
    (result.1.unwrap(), result.2.unwrap())
}

fn create_merkle_tree_hook(suite: &mut Suite, caller: ComponentAddress) -> ComponentAddress {
    let result = suite.instantiate_blueprint("MerkleTreeHook", manifest_args!(caller));
    result.0.expect_commit_success();
    result.1.unwrap()
}

fn create_noop_ism(suite: &mut Suite) -> ComponentAddress {
    let result = suite.instantiate_blueprint("NoopIsm", manifest_args!());
    result.0.expect_commit_success();
    result.1.unwrap()
}

fn create_collateral_token(
    suite: &mut Suite,
    resource_address: ResourceAddress,
    mailbox_component: ComponentAddress,
) -> (ComponentAddress, ResourceAddress) {
    let result = suite.instantiate_blueprint(
        "HypToken",
        manifest_args!(
            ManifestValue::enum_variant(
                0u8,
                vec![ManifestValue::Custom {
                    value: ManifestCustomValue::Address(ManifestAddress::Static(
                        *resource_address.as_node_id()
                    )),
                }]
            ),
            mailbox_component
        ),
    );

    result.0.expect_commit_success();

    (result.1.unwrap(), result.2.unwrap())
}

fn create_synthetic_token(
    suite: &mut Suite,
    mailbox_component: ComponentAddress,
) -> (ComponentAddress, ResourceAddress, ResourceAddress) {
    let result = suite.instantiate_blueprint(
        "HypToken",
        manifest_args!(
            ManifestValue::enum_variant(
                1u8,
                vec![
                    ManifestValue::String {
                        value: "Eth".to_string()
                    },
                    ManifestValue::String {
                        value: "Ether".to_string()
                    },
                    ManifestValue::String {
                        value: "Native ETH from Ethereum".to_string()
                    },
                    ManifestValue::U8 { value: 18 },
                ]
            ),
            mailbox_component
        ),
    );

    result.0.expect_commit_success();
    (
        result.1.unwrap(),
        result.2.unwrap(),
        *result
            .0
            .expect_commit_success()
            .new_resource_addresses()
            .get_index(1)
            .unwrap(),
    )
}

fn setup_mailbox(suite: &mut Suite) -> ComponentAddress {
    let (mailbox_component, mailbox_owner_badge) = create_mailbox(suite, 1000);
    let merkle_tree_hook = create_merkle_tree_hook(suite, mailbox_component);
    let noop_ism = create_noop_ism(suite);

    suite
        .call_method_with_badge(
            mailbox_component,
            "set_required_hook",
            mailbox_owner_badge,
            manifest_args!(merkle_tree_hook),
        )
        .expect_commit_success();

    suite
        .call_method_with_badge(
            mailbox_component,
            "set_default_ism",
            mailbox_owner_badge,
            manifest_args!(noop_ism),
        )
        .expect_commit_success();

    mailbox_component
}

pub fn transfer_remote(
    suite: &mut Suite,
    token_component_address: ComponentAddress,
    destination: u32,
    recipient_address: Bytes32,
    amount: Decimal,
    resource_address: ResourceAddress,
    xrd_fee: Decimal,
    custom_hook: Option<ComponentAddress>,
    standard_hook_metadata: Option<StandardHookMetadata>,
) -> TransactionReceipt {
    let standard_hook_manifest = match standard_hook_metadata {
        Some(metadata) => Some((metadata.gas_limit, metadata.custom_bytes)),
        None => None,
    };

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, xrd_fee.clone())
        .withdraw_from_account(suite.account.address, resource_address, amount.clone())
        .take_from_worktop(XRD, xrd_fee.clone(), "hyperlane_fee")
        .take_from_worktop(resource_address, amount.clone(), "amount")
        .call_method_with_name_lookup(token_component_address, "transfer_remote", |lookup| {
            manifest_args!(
                destination,
                recipient_address,
                lookup.bucket("amount"),
                vec![lookup.bucket("hyperlane_fee")],
                custom_hook,
                standard_hook_manifest,
            )
        })
        .deposit_batch(suite.account.address, ManifestExpression::EntireWorktop)
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
fn test_collateral_create_token() {
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);

    create_collateral_token(&mut suite, XRD, mailbox_component);
}

#[test]
fn test_collateral_non_enrolled_remote_router() {
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);

    let (collateral_token, _owner_badge) =
        create_collateral_token(&mut suite, XRD, mailbox_component);

    let receipt = transfer_remote(
        &mut suite,
        collateral_token,
        1337,
        Bytes32::default(),
        100.into(),
        XRD,
        0.into(),
        None,
        None,
    );

    assert!(format!("{:?}", receipt.expect_failure())
        .contains("no route enrolled for destination 1337"))
}

#[test]
fn test_collateral_enroll_route_and_send_remote() {
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);

    let recipient_contract: Bytes32 =
        hex_str_to_bytes32("0000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496");

    let recipient_user: Bytes32 =
        hex_str_to_bytes32("0000000000000000000000003f429f1bebdf2aac3c8eccf5a19b78cae73a3c4e");

    let (collateral_token, owner_badge) =
        create_collateral_token(&mut suite, XRD, mailbox_component);

    let receipt = suite.call_method_with_badge(
        collateral_token,
        "enroll_remote_router",
        owner_badge,
        manifest_args!(1337u32, recipient_contract, dec!(12)),
    );
    receipt.expect_commit_success();

    let amount = dec!(100);
    let receipt = transfer_remote(
        &mut suite,
        collateral_token,
        1337u32,
        recipient_user,
        amount,
        XRD,
        0.into(),
        None,
        None,
    );

    let collateral_balance = suite.ledger.get_component_balance(collateral_token, XRD);

    receipt.expect_commit_success();
    assert_eq!(collateral_balance, amount);

    // Check dispatch event for a correct message
    let dispatch_event = receipt
        .expect_commit_success()
        .application_events
        .iter()
        .find(|event| event.0 .1 == "DispatchEvent")
        .unwrap();
    let dispatch_event: hyperlane_radix::contracts::mailbox::DispatchEvent =
        scrypto_decode(&dispatch_event.1).expect("Failed to decode event");

    assert_eq!(dispatch_event.destination, 1337u32);
    assert_eq!(dispatch_event.recipient, recipient_contract);
    let expected_message = hex::decode("0300000000000003e80000c07341fadfb99d506736cf979374b560851b181d9e83e225d5437ac270e8000005390000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e14960000000000000000000000003f429f1bebdf2aac3c8eccf5a19b78cae73a3c4e0000000000000000000000000000000000000000000000056bc75e2d63100000").unwrap();
    assert_eq!(dispatch_event.message, expected_message);
}

#[test]
fn test_collateral_receive_token() {
    //Arrange
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);
    let recipient_contract: Bytes32 =
        hex_str_to_bytes32("0000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496");
    let recipient_user: Bytes32 =
        hex_str_to_bytes32("0000000000000000000000003f429f1bebdf2aac3c8eccf5a19b78cae73a3c4e");

    let (collateral_token, owner_badge) =
        create_collateral_token(&mut suite, XRD, mailbox_component);

    suite
        .call_method_with_badge(
            collateral_token,
            "enroll_remote_router",
            owner_badge,
            manifest_args!(1337u32, recipient_contract, dec!(12)),
        )
        .expect_commit_success();

    let amount = dec!(200);
    transfer_remote(
        &mut suite,
        collateral_token,
        1337u32,
        recipient_user,
        amount,
        XRD,
        0.into(),
        None,
        None,
    )
    .expect_commit_success();

    // Act - send back 50 XRD
    let metadata: Vec<u8> = vec![];
    let payload: Vec<u8> = hex::decode("0300000000000005390000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496000003e80000c07341fadfb99d506736cf979374b560851b181d9e83e225d5437ac270e80000c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d42000000000000000000000000000000000000000000000002b5e3af16b1880000").unwrap();
    let visible_components = vec![suite.account.address, collateral_token];

    let receipt = suite.call_method(
        mailbox_component,
        "process",
        manifest_args!(metadata, payload, visible_components),
    );

    // Assert
    receipt.expect_commit_success();

    let collateral_balance = suite.ledger.get_component_balance(collateral_token, XRD);
    assert_eq!(collateral_balance, dec!(150));
}

#[test]
fn test_synthetic_create_token() {
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);

    create_synthetic_token(&mut suite, mailbox_component);
}

#[test]
fn test_synthetic_receive_token() {
    //Arrange
    let mut suite = common::setup();
    let mailbox_component = setup_mailbox(&mut suite);
    let recipient_contract: Bytes32 =
        hex_str_to_bytes32("0000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496");

    let (synthetic_token, owner_badge, synthetic_token_resource) =
        create_synthetic_token(&mut suite, mailbox_component);

    suite
        .call_method_with_badge(
            synthetic_token,
            "enroll_remote_router",
            owner_badge,
            manifest_args!(1337u32, recipient_contract, dec!(12)),
        )
        .expect_commit_success();

    // Act - send back 50 XRD
    let metadata: Vec<u8> = vec![];
    let payload: Vec<u8> = hex::decode("0300000000000005390000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496000003e80000c07341fadfb99d506736cf979374b560851b181d9e83e225d5437ac270e80000c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d42000000000000000000000000000000000000000000000002b5e3af16b1880000").unwrap();
    let visible_components = vec![suite.account.address, synthetic_token];

    let receipt = suite.call_method(
        mailbox_component,
        "process",
        manifest_args!(metadata, payload, visible_components),
    );

    // Assert
    receipt.expect_commit_success();

    let component_balance = suite.ledger.get_component_balance(synthetic_token, XRD);
    assert_eq!(component_balance, dec!(0));

    let component_balance = suite
        .ledger
        .get_component_balance(suite.account.address, synthetic_token_resource);
    assert_eq!(component_balance, dec!(50));
}
