use crate::types::{announcement_digest, hash_concat, recover_eth_address, EthAddress};
use scrypto::prelude::*;
use std::ops::Deref;

#[blueprint]
mod validator_announce {

    struct ValidatorAnnounce {
        storage_locations: KeyValueStore<EthAddress, Vec<String>>,
        announcements: KeyValueStore<Hash, ()>,
        validators: Vec<EthAddress>, // TODO: remove
        mailbox: ComponentAddress,
        local_domain: u32,
    }

    impl ValidatorAnnounce {
        pub fn instantiate(mailbox: ComponentAddress) -> Global<ValidatorAnnounce> {
            // get the local domain from the mailbox
            let local_domain =
                ScryptoVmV1Api::object_call(mailbox.as_node_id(), "local_domain", scrypto_args!());
            let local_domain: u32 = scrypto_decode(&local_domain)
                .expect("ValidatorAnnounce: failed to decode local_domain from mailbox");

            Self {
                storage_locations: KeyValueStore::new(),
                announcements: KeyValueStore::new(),
                validators: Vec::new(),
                mailbox,
                local_domain,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize()
        }

        pub fn get_announced_storage_locations(
            &self,
            validators: Vec<EthAddress>,
        ) -> Vec<Vec<String>> {
            validators
                .iter()
                .map(|validator| {
                    self.storage_locations
                        .get(validator)
                        .cloned()
                        .unwrap_or_default() // return an empty list if the validator is not present
                })
                .collect()
        }

        pub fn get_announced_validators(&self) -> Vec<EthAddress> {
            self.validators.clone()
        }

        pub fn announce(
            &mut self,
            address: EthAddress,
            storage_location: String,
            signature: Vec<u8>,
        ) -> bool {
            let announcement_id = hash_concat(address, &storage_location);
            let replayed = self.announcements.get(&announcement_id);

            if replayed.is_some() {
                panic!("ValidatorAnnounce: cannot announce same storage locations twice")
            }

            self.announcements.insert(announcement_id, ());

            let announcement_digest =
                announcement_digest(&storage_location, self.local_domain, self.mailbox.into());
            info!("{:?}", announcement_digest);

            let signature = Secp256k1Signature::try_from(signature.as_slice())
                .expect("ValidatorAnnounce: failed to parse signature");

            let signer = recover_eth_address(&announcement_digest, &signature);
            info!("{:?}", signer);
            if signer != address {
                panic!("ValidatorAnnounce: signer does not match passed address")
            }

            // we could reference the already inserted locations if present, instead of cloning them
            // but we expect this not to be expensive either way
            let mut locations = self
                .storage_locations
                .get(&address)
                .map(|x| x.deref().clone())
                .unwrap_or_default();

            locations.push(storage_location);

            self.storage_locations.insert(address, locations);
            self.validators.push(address);

            true
        }
    }
}
