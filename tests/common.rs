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
