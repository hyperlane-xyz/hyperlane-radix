use crate::types::{merkle::MerkleTree, Bytes32};
use crate::{
    contracts::hooks::types::Types,
    types::{metadata::StandardHookMetadata, HyperlaneMessage},
};
use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct InsertedIntoTreeEvent {
    pub id: Bytes32,
    pub index: u32,
}

// TODO: create a hook trait that each hook has to implement
// it should implement a set of default methods
#[blueprint]
#[events(InsertedIntoTreeEvent)]
mod merkle_tree_hook {

    struct MerkleTreeHook {
        merkle_tree: MerkleTree,
        mailbox: ComponentAddress,
    }

    impl MerkleTreeHook {
        // TODO: discuss whether or not this component has to be global
        // it instead could just be owned by the mailbox/aggregation hook directly
        // this would also ensure, that the post_dispatch invokation is always correct
        pub fn instantiate(mailbox: ComponentAddress) -> Global<MerkleTreeHook> {
            Self {
                mailbox,
                merkle_tree: MerkleTree::new(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize()
        }

        pub fn hook_type() -> Types {
            Types::MERKLETREE
        }

        pub fn count(&self) -> u32 {
            self.merkle_tree.count() as u32 // TODO: enforce size limit
        }

        pub fn root(&self) -> Hash {
            self.merkle_tree.root()
        }

        pub fn latest_checkpoint(&self) -> (Hash, u32) {
            (self.root(), self.count() - 1)
        }

        pub fn is_latest_dispatched(&self, id: Bytes32) -> bool {
            let is_latest_dispatch = ScryptoVmV1Api::object_call(
                self.mailbox.as_node_id(),
                "is_latest_dispatched",
                scrypto_args!(id),
            );
            scrypto_decode(&is_latest_dispatch)
                .expect("MerkleTreeHook: failed to decode is_latest_dispatch from mailbox")
        }

        /// Post dispatch accepts a vec of buckets, that is the payment that the user is willing to pass
        /// We can't assume that payments will happen onyl in one resource type (one bucket is always only assisated with one resource)
        /// We return the left over Buckets that have not been consumed
        pub fn post_dispatch(
            &mut self,
            _metadata: Option<StandardHookMetadata>,
            message: HyperlaneMessage,
            payment: Vec<FungibleBucket>,
        ) -> Vec<FungibleBucket> {
            let id = message.id();
            // TODO: we can't perfrom any state queries on the mailbox because it has already been locked when the merkle tree hook is called from the mailbox
            // TODO: we have to pass a proof that only the mailbox can create in order to verify that this has been called from the mailbox
            // TODO: we can do this by verifiying a proof to a resource that only the mailbox holds
            // if !self.is_latest_dispatched(id) {
            //     panic!("MerkleTreeHooK: message not dispatching on mailbox")
            // }

            let index = self.count();
            self.merkle_tree.insert(id.into());

            Runtime::emit_event(InsertedIntoTreeEvent { id, index });

            // Merkle tree hook does not consume any resources, return the entire payment unchangec
            payment
        }

        /// Quote dispatch returns a map from resources and their amount that is required in decimals
        /// this ensure that we are not limited to a single payment resource and instead can model multiple resources
        /// that might be needed in order to perfrom a remote transfer
        pub fn quote_dispatch(
            &self,
            _metadata: Option<StandardHookMetadata>,
            _message: HyperlaneMessage,
        ) -> IndexMap<ResourceAddress, Decimal> {
            IndexMap::new()
        }
    }
}
