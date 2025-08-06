use crate::types::{merkle::MerkleTree, Bytes32};
use crate::{
    contracts::hooks::types::Types,
    format_error,
    types::{metadata::StandardHookMetadata, HyperlaneMessage},
};
use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct InsertedIntoTreeEvent {
    pub id: Bytes32,
    pub index: u32,
}

#[blueprint]
#[events(InsertedIntoTreeEvent)]
mod merkle_tree_hook {

    enable_method_auth! {
        roles {
            parent_component => updatable_by: [];
        },
        methods {
            // Public
            hook_type => PUBLIC;
            count => PUBLIC;
            root => PUBLIC;
            tree => PUBLIC;
            latest_checkpoint => PUBLIC;
            local_domain => PUBLIC;
            quote_dispatch => PUBLIC;
            // Mailbox Only
            post_dispatch => restrict_to: [parent_component];
        }
    }

    struct MerkleTreeHook {
        merkle_tree: MerkleTree,
        parent: ComponentAddress,
    }

    impl MerkleTreeHook {
        pub fn instantiate(parent: ComponentAddress) -> Global<MerkleTreeHook> {
            // Create mailbox component rule to ensure that the "post_dispatch()" function can only
            // be called by the mailbox itself.
            let mailbox_component_rule =
                rule!(require(NonFungibleGlobalId::global_caller_badge(parent)));

            Self {
                parent,
                merkle_tree: MerkleTree::new(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(roles! {
                parent_component => mailbox_component_rule;
            })
            .globalize()
        }

        pub fn hook_type(&self) -> Types {
            Types::MERKLETREE
        }

        pub fn count(&self) -> u32 {
            self.merkle_tree.count() as u32
        }

        pub fn tree(&self) -> MerkleTree {
            self.merkle_tree.clone()
        }

        pub fn root(&self) -> Hash {
            self.merkle_tree.root()
        }

        pub fn latest_checkpoint(&self) -> (Hash, u32) {
            (self.root(), self.count() - 1)
        }

        pub fn local_domain(&self) -> u32 {
            let result = ScryptoVmV1Api::object_call(
                self.parent.as_node_id(),
                "local_domain",
                scrypto_args!(),
            );

            scrypto_decode(&result).expect(&format_error!("failed to decode domain result"))
        }

        /// Post-dispatch accepts a vec of buckets; that is the payment that the user is willing to
        /// pass. We can't assume that payments will happen only in one resource type
        /// (one bucket is always only associated with one resource).
        /// We return the leftover buckets that have not been consumed.
        pub fn post_dispatch(
            &mut self,
            _metadata: Option<StandardHookMetadata>,
            message: HyperlaneMessage,
            payment: Vec<FungibleBucket>,
        ) -> Vec<FungibleBucket> {
            let id = message.id();

            let index = self.count();
            self.merkle_tree.insert(id.into());

            Runtime::emit_event(InsertedIntoTreeEvent { id, index });

            // Merkle tree hook does not consume any resources, return the entire payment unchanged.
            payment
        }

        /// Quote dispatch returns a map from resources and their amount that is required in
        /// decimals. This ensures that we are not limited to a single payment resource and instead
        /// can model multiple resources that might be needed to perform a remote transfer
        pub fn quote_dispatch(
            &self,
            _metadata: Option<StandardHookMetadata>,
            _message: HyperlaneMessage,
        ) -> IndexMap<ResourceAddress, Decimal> {
            IndexMap::new()
        }
    }
}
