use crate::contracts::isms::types::Types;
use crate::format_error;
use crate::types::HyperlaneMessage;
use scrypto::prelude::*;

#[blueprint]
mod routing_ism {

    enable_method_auth! {
        methods {
            // Public
            module_type => PUBLIC;
            verify => PUBLIC;
            route => PUBLIC;

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
            initial_routes: Vec<(u32, ComponentAddress)>,
        ) -> (Global<RoutingIsm>, FungibleBucket) {
            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(RoutingIsm::blueprint_id());

            // create new owner badge
            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => "Routing Ism Owner Badge", locked;
                    "component" => component_address, locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let routes: KeyValueStore<u32, ComponentAddress> = KeyValueStore::new();
            for (domain, ism) in initial_routes {
                routes.insert(domain, ism);
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
            Types::Routing
        }

        /// Routes a message to a underlying ISM
        pub fn route(&self, raw_message: Vec<u8>) -> ComponentAddress {
            let message: HyperlaneMessage = raw_message.clone().into();

            let ism = self
                .routes
                .get(&message.origin)
                .expect(&format_error!("no ISM for route {}", message.origin));

            *ism
        }

        pub fn verify(&mut self, raw_metadata: Vec<u8>, raw_message: Vec<u8>) -> bool {
            let ism = self.route(raw_message.clone());

            let result = ScryptoVmV1Api::object_call(
                ism.as_node_id(),
                "verify",
                scrypto_args!(raw_metadata, raw_message),
            );

            scrypto_decode(&result)
                .expect(&format_error!("failed to decode ISM verification result"))
        }

        pub fn set_route(&mut self, domain: u32, ism_address: ComponentAddress) {
            self.routes.insert(domain, ism_address);
        }

        pub fn remove_route(&mut self, domain: u32) {
            self.routes.remove(&domain);
        }
    }
}
