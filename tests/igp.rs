use crate::common::Suite;
use hyperlane_radix::types::Bytes32;
use scrypto_test::prelude::*;

mod common;

fn create_igp(suite: &mut Suite) -> (ComponentAddress, ResourceAddress) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "InterchainGasPaymaster",
            "instantiate",
            manifest_args!(XRD),
        )
        .deposit_entire_worktop(suite.account.address)
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );
    let output = receipt.expect_commit_success();
    (
        output.new_component_addresses()[0],
        output.new_resource_addresses()[0],
    )
}

fn set_destination_gas(
    suite: &mut Suite,
    igp_address: ComponentAddress,
    owner_badge: ResourceAddress,
) {
    let configs = vec![(1337u32, ((10_000_000_000u128, 1u128), 10u128))];
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(suite.account.address, owner_badge, dec!(1))
        .call_method(
            igp_address,
            "set_destination_gas_configs",
            manifest_args!(configs),
        )
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_commit_success();
}

#[test]
fn test_create_igp() {
    let mut suite = common::setup();
    create_igp(&mut suite);
}

#[test]
fn test_set_destination_gas_configs() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    let destination_gas: Decimal = suite.call_method_success(
        igp_address,
        "destination_gas_limit",
        manifest_args!(1337u32, Decimal::from(1)),
    );

    assert_eq!(destination_gas, Decimal::from(11))
}

#[test]
fn test_quote_dispatch() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    let metadata = Some((Decimal::one(), Option::<Vec<u8>>::None));

    let destination_gas: IndexMap<ResourceAddress, Decimal> = suite.call_method_success(
        igp_address,
        "quote_dispatch",
        manifest_args!(
            metadata,
            (
                3u8,
                0u32,
                0u32,
                Bytes32::zero(),
                1337u32,
                Bytes32::zero(),
                Vec::<u8>::new()
            )
        ),
    );

    let expected: indexmap::IndexMap<ResourceAddress, Decimal> =
        IndexMap::from_iter(vec![(XRD, Decimal::from(11))]);

    assert_eq!(destination_gas, expected)
}

#[test]
fn test_igp_ownership() {
    let mut suite = common::setup();
    let (igp_address, _) = create_igp(&mut suite);

    let configs = vec![(1337u32, ((10_000_000_000u128, 1u128), 10u128))];
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(suite.dummy_accounts[0].address, XRD, dec!(1))
        .call_method(
            igp_address,
            "set_destination_gas_configs",
            manifest_args!(configs),
        )
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_commit_failure();
}

#[test]
fn test_post_dispatch() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    let metadata = Some((Decimal::one(), Option::<Vec<u8>>::None));
    let message = (
        3u8,
        0u32,
        0u32,
        Bytes32::zero(),
        1337u32,
        Bytes32::zero(),
        Vec::<u8>::new(),
    );

    // this should consume the entire bucket
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(11))
        .take_from_worktop(XRD, dec!(11), "payment")
        .call_method_with_name_lookup(igp_address, "post_dispatch", |lookup| {
            manifest_args!(metadata, message, vec![lookup.bucket("payment")])
        })
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_commit_success();
}

#[test]
fn test_post_dispatch_failure() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    let metadata = Some((Decimal::one(), Option::<Vec<u8>>::None));
    let message = (
        3u8,
        0u32,
        0u32,
        Bytes32::zero(),
        1337u32,
        Bytes32::zero(),
        Vec::<u8>::new(),
    );

    // this should not consume the entire bucket and instead drop a non-empty bucket - which results in a failure
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(11))
        .take_from_worktop(XRD, dec!(12), "payment")
        .call_method_with_name_lookup(igp_address, "post_dispatch", |lookup| {
            manifest_args!(metadata, message, vec![lookup.bucket("payment")])
        })
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_failure();
}

#[test]
fn test_pay_for_gas() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    // this should consume the entire bucket
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(1))
        .take_from_worktop(XRD, dec!(1), "payment")
        .call_method_with_name_lookup(igp_address, "pay_for_gas", |lookup| {
            manifest_args!(
                Bytes32::zero(),
                1337u32,
                Decimal::one(),
                lookup.bucket("payment")
            )
        })
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    receipt.expect_commit_success();
}

#[test]
fn test_pay_for_gas_failure() {
    let mut suite = common::setup();
    let (igp_address, owner_badge) = create_igp(&mut suite);
    set_destination_gas(&mut suite, igp_address, owner_badge);

    // this should consume the entire bucket
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(1))
        .take_from_worktop(XRD, dec!(1), "payment")
        .call_method_with_name_lookup(igp_address, "pay_for_gas", |lookup| {
            manifest_args!(
                Bytes32::zero(),
                1337u32,
                Decimal::from(100),
                lookup.bucket("payment")
            )
        })
        .build();

    let receipt = suite.ledger.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
    );

    assert!(format!("{:?}", receipt.expect_failure())
        .contains("InterchainGasPaymaster: payment for gas does not match IGP quote. quote: 100"))
}
