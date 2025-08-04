use crate::contracts::isms::types::Types;
use scrypto::prelude::*;

// TODO: make this implement the ism trait
#[blueprint]
mod noop_ism {
    struct NoopIsm {}

    impl NoopIsm {
        pub fn instantiate() -> Global<NoopIsm> {
            Self {}
                .instantiate()
                .prepare_to_globalize(OwnerRole::None)
                .globalize()
        }

        pub fn module_type(&self) -> Types {
            Types::Null
        }

        pub fn verify(&mut self, _metadata: Vec<u8>, _message: Vec<u8>) -> bool {
            true
        }
    }
}
