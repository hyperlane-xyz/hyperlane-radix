use crate::common::Suite;
use hyperlane_radix::types::{Bytes32, HyperlaneMessage};
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

pub fn dispatch_message(
    suite: &mut Suite,
    component_address: ComponentAddress,
    destination: u32,
    recipient_address: Bytes32,
    message_body: Vec<u8>,
    hook: Option<ComponentAddress>,
    claimed_account_address: ComponentAddress,
    gas_limit: Decimal,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(20))
        .take_from_worktop(XRD, dec!(1), "payment")
        .call_method_with_name_lookup(component_address, "dispatch", |lookup| {
            manifest_args!(
                destination,
                recipient_address,
                message_body,
                hook,
                Some((gas_limit, None::<Vec<u8>>)),
                vec![lookup.bucket("payment")],
                ManifestValue::enum_variant(
                    1u8,
                    vec![ManifestValue::Custom {
                        value: ManifestCustomValue::Address(ManifestAddress::Static(
                            *claimed_account_address.as_node_id()
                        )),
                    }]
                )
            )
        })
        .deposit_batch(suite.account.address, ManifestExpression::EntireWorktop)
        .build();

    suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    )
}

#[test]
fn test_create_mailbox() {
    let mut suite = common::setup();

    //Act
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 100);

    //Assert
    receipt.expect_commit_success();

    let mailbox_address = mailbox_address.unwrap();
    let domain: u32 = suite.call_method_success(mailbox_address, "local_domain", manifest_args!());
    assert_eq!(domain, 100);

    let nonce: u32 = suite.call_method_success(mailbox_address, "nonce", manifest_args!());
    assert_eq!(nonce, 0);

    let processed: u32 = suite.call_method_success(mailbox_address, "processed", manifest_args!());
    assert_eq!(processed, 0);

    let default_ism: Option<ComponentAddress> =
        suite.call_method_success(mailbox_address, "default_ism", manifest_args!());
    assert!(default_ism.is_none());

    let default_hook: Option<ComponentAddress> =
        suite.call_method_success(mailbox_address, "default_hook", manifest_args!());
    assert!(default_hook.is_none());

    let required_hook: Option<ComponentAddress> =
        suite.call_method_success(mailbox_address, "required_hook", manifest_args!());
    assert!(required_hook.is_none());
}

#[test]
fn test_process_message_invalid_domain() {
    let mut suite = common::setup();
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 1337);
    receipt.expect_commit_success();

    let metadata: Vec<u8> = vec![];
    let message: Vec<u8> = HyperlaneMessage {
        version: 3,
        nonce: 0,
        origin: 1,
        sender: Bytes32::zero(),
        destination: 1, // does not match mailbox domain
        recipient: Bytes32::zero(),
        body: vec![],
    }
    .into();

    let visible_components: Vec<ComponentAddress> = vec![];
    let receipt = suite.call_method(
        mailbox_address.unwrap(),
        "process",
        manifest_args!(metadata, message, visible_components),
    );

    assert!(format!("{:?}", receipt.expect_failure())
        .contains("Mailbox: message destination domain does not match local domain"))
}

#[test]
fn test_process_message_invalid_version() {
    let mut suite = common::setup();
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 1337);
    receipt.expect_commit_success();

    let metadata: Vec<u8> = vec![];
    let message: Vec<u8> = HyperlaneMessage {
        version: 4, // does not match mailbox version
        nonce: 0,
        origin: 1,
        sender: Bytes32::zero(),
        destination: 1337,
        recipient: Bytes32::zero(),
        body: vec![],
    }
    .into();

    let visible_components: Vec<ComponentAddress> = vec![];
    let receipt = suite.call_method(
        mailbox_address.unwrap(),
        "process",
        manifest_args!(metadata, message, visible_components),
    );

    assert!(
        format!("{:?}", receipt.expect_failure()).contains("Mailbox: unsupported message version")
    )
}

#[test]
fn test_process_message_recipient_no_app() {
    let mut suite = common::setup();
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 1337);
    receipt.expect_commit_success();

    let metadata: Vec<u8> = vec![];
    let message: Vec<u8> = HyperlaneMessage {
        version: 3,
        nonce: 0,
        origin: 1,
        sender: Bytes32::zero(),
        destination: 1337,
        recipient: suite.dummy_accounts[0].address.into(),
        body: vec![],
    }
    .into();

    let visible_components: Vec<ComponentAddress> = vec![suite.dummy_accounts[0].address];
    let receipt = suite.call_method(
        mailbox_address.unwrap(),
        "process",
        manifest_args!(metadata, message, visible_components),
    );

    assert!(format!("{:?}", receipt.expect_failure()).contains("SystemModuleError"))
}

#[test]
fn test_dispatch_message() {
    let mut suite = common::setup();
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 100);
    receipt.expect_commit_success();

    let address = suite.account.address;
    let r = dispatch_message(
        &mut suite,
        mailbox_address.unwrap(),
        1337u32,
        Bytes32::zero(),
        vec![],
        None,
        address,
        dec!(200000),
    );

    r.expect_commit_success();
}

#[test]
fn test_dispatch_message_invalid_claimed_sender() {
    let mut suite = common::setup();
    let (receipt, mailbox_address, _) = create_mailbox(&mut suite, 100);
    receipt.expect_commit_success();

    // Choose an invalid dummy account as sender here
    let address = suite.dummy_accounts[0].address;
    let r = dispatch_message(
        &mut suite,
        mailbox_address.unwrap(),
        1337u32,
        Bytes32::zero(),
        vec![],
        None,
        address,
        dec!(200000),
    );

    let outcome = &r.expect_commit_failure().outcome;
    assert_eq!(
        format!("{outcome:?}"),
        "Failure(SystemError(AssertAccessRuleFailed))"
    );
}
