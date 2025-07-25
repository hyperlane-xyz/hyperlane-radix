use scrypto::blueprints::transaction_processor::InstructionOutput::CallReturn;
use scrypto_test::prelude::*;

pub struct SuiteAccount {
    pub public_key: Secp256k1PublicKey,
    pub _private_key: Secp256k1PrivateKey,
    pub address: ComponentAddress,
}

pub struct Suite {
    pub ledger: LedgerSimulator<NoExtension, InMemorySubstateDatabase>,
    pub account: SuiteAccount,
    pub dummy_accounts: Vec<SuiteAccount>,
    pub package_address: PackageAddress,
}

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

impl Suite {
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

        let receipt = self.ledger.execute_manifest(
            manifest,
            vec![NonFungibleGlobalId::from_public_key(
                &self.account.public_key,
            )],
        );

        receipt
    }

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
            CallReturn(data) => scrypto_decode(&data).expect("Failed to decode result."),
            _ => panic!("No CallData returned."),
        }
    }
}
