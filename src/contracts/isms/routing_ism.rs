use crate::contracts::isms::types::Types;
use crate::types::HyperlaneMessage;
use scrypto::prelude::*;
// TODO: make this implement the ism trait

#[blueprint]
mod routing_ism {

    enable_method_auth! {
        methods {
            // Public
            module_type => PUBLIC;
            verify => PUBLIC;

            // Private
            set_route => restrict_to: [OWNER];
            remove_route => restrict_to: [OWNER];
        }
    }

    struct RoutingIsm {
        routes: KeyValueStore<u32, ComponentAddress>,
    }

    impl RoutingIsm {
        pub fn instantiate(
            domains: Vec<u32>,
            isms: Vec<ComponentAddress>,
        ) -> (Global<RoutingIsm>, FungibleBucket) {
            assert_eq!(
                domains.len(),
                isms.len(),
                "domains and ism array must have the same length"
            );

            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(RoutingIsm::blueprint_id());

            // create new owner badge
            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => format!(
                        "Hyperlane Routing Ism Owner Badge {}",
                        Runtime::bech32_encode_address(component_address)
                    ), locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let routes: KeyValueStore<u32, ComponentAddress> = KeyValueStore::new();
            for (domain, ism) in domains.iter().zip(isms.iter()) {
                routes.insert(*domain, *ism);
            }

            let component = Self { routes }
                .instantiate()
                .prepare_to_globalize(OwnerRole::Updatable(rule!(require(
                    owner_badge.resource_address()
                ))))
                .with_address(address_reservation)
                .globalize();

            (component, owner_badge)
        }

        pub fn module_type(&self) -> Types {
            Types::ROUTING
        }

        pub fn verify(&mut self, raw_metadata: Vec<u8>, raw_message: Vec<u8>) -> bool {
            let message: HyperlaneMessage = raw_message.clone().into();

            let ism = self
                .routes
                .get(&message.origin)
                .expect(format!("No ISM for route {}", message.origin).as_str());

            let result = ScryptoVmV1Api::object_call(
                ism.as_node_id(),
                "verify",
                scrypto_args!(raw_metadata, raw_message),
            );

            let result: bool =
                scrypto_decode(&result).expect("Failed to decode ISM verification result");
            if !result {
                panic!("Mailbox: ISM verification failed");
            }

            true
        }

        pub fn set_route(&mut self, domain: u32, ism_address: ComponentAddress) {
            self.routes.insert(domain, ism_address);
        }

        pub fn remove_route(&mut self, domain: u32) {
            self.routes.remove(&domain);
        }
    }
}
