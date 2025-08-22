use crate::contracts::isms::multisig_ism::verify_multisig;
use crate::contracts::isms::types::Types;
use crate::panic_error;
use crate::types::metadata::MultisigIsmMessageIdMetadata;
use crate::types::EthAddress;
use crate::types::HyperlaneMessage;
use scrypto::prelude::*;

#[blueprint]
mod message_id_multisig_ism {

    struct MessageIdMultisigIsm {
        validators: Vec<EthAddress>,
        threshold: usize,
    }

    impl MessageIdMultisigIsm {
        pub fn instantiate(
            validators: Vec<EthAddress>,
            threshold: usize,
        ) -> Global<MessageIdMultisigIsm> {
            if validators.len() < threshold {
                panic_error!("threshold must be less than or equal to the number of validators");
            }
            Self {
                validators,
                threshold,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize()
        }

        pub fn module_type(&self) -> Types {
            Types::MessageIdMultisig
        }

        pub fn validators_and_threshold(&self, _message: Vec<u8>) -> (Vec<EthAddress>, usize) {
            (self.validators.clone(), self.threshold)
        }

        pub fn verify(&mut self, metadata: Vec<u8>, message: Vec<u8>) -> bool {
            let metadata: MultisigIsmMessageIdMetadata = metadata.into();

            let message: HyperlaneMessage = message.into();
            let digest = message.digest(
                metadata.origin_merkle_tree_hook,
                metadata.merkle_root,
                metadata.merkle_index,
            );

            verify_multisig(
                digest,
                &metadata.validator_signatures,
                &self.validators,
                self.threshold,
            )
        }
    }
}
