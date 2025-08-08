use hyperlane_radix::types::Bytes32;
use scrypto::blueprints::transaction_processor::InstructionOutput::CallReturn;
use scrypto_test::prelude::*;

pub struct SuiteAccount {
    pub public_key: Secp256k1PublicKey,
    pub _private_key: Secp256k1PrivateKey,
    pub address: ComponentAddress,
}

#[allow(dead_code)]
pub struct Suite {
    pub ledger: LedgerSimulator<NoExtension, InMemorySubstateDatabase>,
    pub account: SuiteAccount,
    pub dummy_accounts: Vec<SuiteAccount>,
    pub package_address: PackageAddress,
}

#[allow(dead_code)]
pub fn setup() -> Suite {
    let mut ledger = LedgerSimulatorBuilder::new().build();
    let (public_key, _private_key, account_address) = ledger.new_allocated_account();
    let package_address = ledger.compile_and_publish(this_package!());

    let dummy_accounts: Vec<SuiteAccount> = (0..10)
        .map(|_| {
            let (public_key, _private_key, account_address) = ledger.new_allocated_account();
            SuiteAccount {
                address: account_address,
                public_key,
                _private_key,
            }
        })
        .collect();

    Suite {
        ledger,
        account: SuiteAccount {
            public_key,
            _private_key,
            address: account_address,
        },
        package_address,
        dummy_accounts,
    }
}

#[allow(dead_code)]
pub fn hex_str_to_bytes32(hex_string: &str) -> Bytes32 {
    hex::decode(hex_string).unwrap().as_slice().into()
}

impl Suite {
    #[allow(dead_code)]
    pub fn call_method(
        &mut self,
        component_address: ComponentAddress,
        method_name: &str,
        arguments: impl ResolvableArguments,
    ) -> TransactionReceipt {
        let manifest = ManifestBuilder::new()
            .lock_fee_from_faucet()
            .call_method(component_address, method_name, arguments)
            .build();

        self.ledger.execute_manifest(
            manifest,
            vec![NonFungibleGlobalId::from_public_key(
                &self.account.public_key,
            )],
        )
    }

    #[allow(dead_code)]
    pub fn call_method_success<T: ScryptoDecode>(
        &mut self,
        component_address: ComponentAddress,
        method_name: &str,
        arguments: impl ResolvableArguments,
    ) -> T {
        let receipt = self.call_method(component_address, method_name, arguments);
        receipt.expect_commit_success();
        let outcome = receipt.expect_commit_success().outcome.expect_success();
        match outcome.get(1).unwrap() {
            CallReturn(data) => scrypto_decode(data).expect("Failed to decode result."),
            _ => panic!("No CallData returned."),
        }
    }

    #[allow(dead_code)]
    pub fn call_method_with_badge(
        &mut self,
        component_address: ComponentAddress,
        method_name: &str,
        owner_badge: ResourceAddress,
        arguments: impl ResolvableArguments,
    ) -> TransactionReceipt {
        let manifest = ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_proof_from_account_of_amount(self.account.address, owner_badge, dec!(1))
            .call_method(component_address, method_name, arguments)
            .build();

        self.ledger.execute_manifest(
            manifest,
            vec![NonFungibleGlobalId::from_public_key(
                &self.account.public_key,
            )],
        )
    }

    #[allow(dead_code)]
    pub fn call_method_success_with_badge<T: ScryptoDecode>(
        &mut self,
        component_address: ComponentAddress,
        method_name: &str,
        owner_badge: ResourceAddress,
        arguments: impl ResolvableArguments,
    ) -> T {
        let receipt =
            self.call_method_with_badge(component_address, method_name, owner_badge, arguments);
        receipt.expect_commit_success();
        let outcome = receipt.expect_commit_success().outcome.expect_success();
        match outcome.get(1).unwrap() {
            CallReturn(data) => scrypto_decode(data).expect("Failed to decode result."),
            _ => panic!("No CallData returned."),
        }
    }

    #[allow(dead_code)]
    pub fn instantiate_blueprint(
        &mut self,
        blueprint_name: &str,
        arguments: impl ResolvableArguments,
    ) -> (
        TransactionReceipt,
        Option<ComponentAddress>,
        Option<ResourceAddress>,
    ) {
        let manifest = ManifestBuilder::new()
            .lock_fee_from_faucet()
            .call_function(
                self.package_address,
                blueprint_name,
                "instantiate",
                arguments,
            )
            .deposit_entire_worktop(self.account.address)
            .build();

        let receipt = self.ledger.execute_manifest(
            manifest,
            vec![NonFungibleGlobalId::from_public_key(
                &self.account.public_key,
            )],
        );

        match receipt.result.clone() {
            TransactionResult::Commit(data) => (
                receipt,
                Some(data.new_component_addresses()[0]),
                data.new_resource_addresses().get_index(0).copied(),
            ),
            TransactionResult::Abort(_) => (receipt, None, None),
            TransactionResult::Reject(_) => (receipt, None, None),
        }
    }
}
