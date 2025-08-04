use crate::common::Suite;
use hyperlane_radix::types::{Bytes32, HyperlaneMessage};
use scrypto_test::prelude::*;

mod common;

fn create_merkle_tree_hook(suite: &mut Suite, caller: ComponentAddress) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(
            suite.package_address,
            "MerkleTreeHook",
            "instantiate",
            manifest_args!(caller),
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

fn merkle_root(suite: &mut Suite, component_address: ComponentAddress) -> Hash {
    suite.call_method_success(component_address, "root", manifest_args!())
}

fn latest_checkpoint(suite: &mut Suite, component_address: ComponentAddress) -> (Hash, u32) {
    suite.call_method_success(component_address, "latest_checkpoint", manifest_args!())
}

pub fn merkle_tree_post_dispatch(
    suite: &mut Suite,
    component_address: ComponentAddress,
    message: HyperlaneMessage,
) -> TransactionReceipt {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(suite.account.address, XRD, dec!(20))
        .take_from_worktop(XRD, dec!(1), "payment")
        .call_method_with_name_lookup(component_address, "post_dispatch", |lookup| {
            manifest_args!(
                None::<(Decimal, Option<Vec<u8>>)>,
                (
                    message.version,
                    message.nonce,
                    message.origin,
                    message.sender,
                    message.destination,
                    message.recipient,
                    message.body
                ),
                vec![lookup.bucket("payment")],
            )
        })
        .deposit_batch(suite.account.address, ManifestExpression::EntireWorktop)
        .build();

    let no_auth_config =
        ExecutionConfig::for_auth_disabled_system_transaction(NetworkDefinition::simulator());

    let receipt = suite.ledger.execute_manifest_with_execution_config(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(
            &suite.account.public_key,
        )],
        no_auth_config,
    );

    receipt
}

fn hex_str_to_bytes32(hex_string: &str) -> Bytes32 {
    hex::decode(hex_string).unwrap().as_slice().into()
}

#[test]
fn test_create_merkle_tree_hook() {
    let mut suite = common::setup();
    let caller = suite.account.address.clone();
    let receipt = create_merkle_tree_hook(&mut suite, caller);
    receipt.expect_commit_success();
}

#[test]
fn test_empty_root() {
    let mut suite = common::setup();
    let caller = suite.account.address.clone();
    let receipt = create_merkle_tree_hook(&mut suite, caller);
    receipt.expect_commit_success();
    let component_address = receipt.expect_commit_success().new_component_addresses()[0];

    let res = merkle_root(&mut suite, component_address);
    // Root of an empty merkle tree
    assert_eq!(
        res.to_string(),
        "27ae5ba08d7291c96c8cbddcc148bf48a6d68c7974b94356f53754ef6171d757"
    );
}

#[test]
fn test_example() {
    let mut suite = common::setup();
    let caller = suite.account.address.clone();
    let receipt = create_merkle_tree_hook(&mut suite, caller);
    receipt.expect_commit_success();
    let component_address = receipt.expect_commit_success().new_component_addresses()[0];

    let expected_hashes = vec![
        "10df2f89cb24ed6078fc3949b4870e94a7e32e40e8d8c6b7bd74ccc2c933d760",
        "080ef1c2cd394de78363ecb0a466c934b57de4abb5604a0684e571990eb7b073",
        "bf78ad252da524f1e733aa6b83514dd83225676b5828f888f01487108f8f7cc7",
    ];

    for i in 0..3 {
        let mut body: [u8; 32] = [0; 32];
        body[31] = i;

        // Craft dummy message
        let recipient: Bytes32 =
            hex_str_to_bytes32("00000000000000000000000000000000000000000000000000000000deadbeef");
        let sender: Bytes32 =
            hex_str_to_bytes32("0000000000000000000000007fa9385be102ac3eac297483dd6233d62b3e1496");
        let message = HyperlaneMessage::new(0, 11, sender, 22, recipient, Vec::from(body));

        let receipt = merkle_tree_post_dispatch(&mut suite, component_address, message.clone());
        receipt.expect_commit_success();

        let result_hash = merkle_root(&mut suite, component_address);
        assert_eq!(format!("{:?}", result_hash), expected_hashes[i as usize]);

        let (checkpoint_root, checkpoint) = latest_checkpoint(&mut suite, component_address);
        assert_eq!(
            format!("{:?}", checkpoint_root),
            expected_hashes[i as usize]
        );
        assert_eq!(checkpoint, i as u32);
    }
}
