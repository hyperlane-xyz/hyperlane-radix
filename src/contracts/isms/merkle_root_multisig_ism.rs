use crate::contracts::isms::{multisig_ism::verify_multisig, types::Types};
use crate::types::merkle::merkle_root_from_branch;
use crate::types::metadata::MultisigIsmMerkleRootMetadata;
use crate::types::EthAddress;
use crate::types::HyperlaneMessage;
use scrypto::prelude::*;
// TODO: make this implement the ism trait

#[blueprint]
mod merkle_root_multisig_ism {

    struct MerkleRootMultisigIsm {
        validators: Vec<EthAddress>,
        threshold: usize,
    }

    impl MerkleRootMultisigIsm {
        pub fn instantiate(
            validators: Vec<EthAddress>,
            threshold: usize,
        ) -> Global<MerkleRootMultisigIsm> {
            // TODO: assert correct threshold & uniqueness
            Self {
                validators,
                threshold,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize()
        }

        pub fn module_type(&self) -> Types {
            Types::MerkleRootMultisig
        }

        pub fn validators_and_threshold(&self, _message: Vec<u8>) -> (Vec<EthAddress>, usize) {
            (self.validators.clone(), self.threshold)
        }

        pub fn verify(&mut self, metadata: Vec<u8>, message: Vec<u8>) -> bool {
            let metadata: MultisigIsmMerkleRootMetadata = metadata.into();

            let message: HyperlaneMessage = message.into();
            let signed_root = merkle_root_from_branch(
                message.id().into(),
                &metadata.merkle_proof,
                metadata.message_index,
            );

            let digest = message.digest(
                metadata.origin_merkle_tree_hook,
                signed_root.into(),
                metadata.message_index,
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
