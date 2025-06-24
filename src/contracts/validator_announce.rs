use crate::types::{announcement_digest, hash_concat, recover_eth_address, EthAddress};
use scrypto::prelude::*;

#[blueprint]
mod validator_announce {

    struct ValidatorAnnounce {
        storage_locations: HashMap<EthAddress, Vec<String>>,
        announcements: HashSet<Hash>,
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
                storage_locations: HashMap::new(),
                announcements: HashSet::new(),
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
                .filter_map(|validator| self.storage_locations.get(validator))
                .cloned()
                .collect()
        }

        pub fn get_announced_validators(&self) -> Vec<EthAddress> {
            self.storage_locations.keys().into_iter().cloned().collect()
        }

        pub fn announce(
            &mut self,
            address: EthAddress,
            storage_location: String,
            signature: Vec<u8>,
        ) -> bool {
            let announcement_id = hash_concat(address, &storage_location);
            let replayed = self.announcements.insert(announcement_id);

            if replayed {
                panic!("ValidatorAnnounce: cannot announce same storage locations twice")
            }

            let announcement_digest =
                announcement_digest(&storage_location, self.local_domain, self.mailbox.into());
            let signature = Secp256k1Signature::try_from(signature.as_slice())
                .expect("ValidatorAnnounce: failed to parse signature");

            let signer = recover_eth_address(announcement_digest.as_slice(), &signature);

            if signer != address {
                panic!("ValidatorAnnounce: signer does not match passed address")
            }

            self.storage_locations
                .entry(address)
                .or_insert_with(Vec::new)
                .push(storage_location);
            true
        }
    }
}
