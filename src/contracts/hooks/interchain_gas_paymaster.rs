use scrypto::prelude::*;

use crate::{
    contracts::hooks::types::Types,
    types::{metadata::StandardHookMetadata, Bytes32, HyperlaneMessage},
};
#[derive(ScryptoSbor)]
pub struct DestinationGasConfig {
    pub gas_oracle: GasOracle,
    pub gas_overhead: u128,
}

#[derive(ScryptoSbor)]
pub struct GasOracle {
    pub token_exchange_rate: u128,
    pub gas_price: u128,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct GasPayment {
    pub message_id: Bytes32,
    pub destination_domain: u32,
    pub gas_amount: Decimal,
    pub payment: Decimal,
    pub resource_address: ResourceAddress,
    pub sequence: u32,
}

#[blueprint]
#[events(GasPayment)]
mod interchain_gas_paymaster {

    // TODO: maybe model this with decimals
    const EXCHANGE_RATE_SCALE: u64 = 10_000_000_000u64; // 1e10
    const DEFAULT_GAS: usize = 1usize;

    enable_method_auth! {
        // decide which methods are public and which are restricted to the component's owner
        methods {
            // Public
            hook_type => PUBLIC;
            destination_gas_limit => PUBLIC;
            quote_gas_payment => PUBLIC;
            pay_for_gas => PUBLIC;
            post_dispatch => PUBLIC;
            quote_dispatch => PUBLIC;

            // Owner only
            set_destination_gas_configs => restrict_to: [OWNER];
            claim => restrict_to: [OWNER];
        }
    }

    // TODO: configure public / owner methods like on the mailbox
    struct InterchainGasPaymaster {
        // map from domain -> gas config
        destination_gas_configs: KeyValueStore<u32, DestinationGasConfig>,

        // resource address that the user pays their gas in
        resource_address: ResourceAddress,

        // the vault holds the resources until they are claimed
        vault: FungibleVault,

        // current event sequence. used for better indexing
        sequence: u32,
    }

    impl InterchainGasPaymaster {
        pub fn instantiate(
            resource: ResourceAddress,
        ) -> (Global<InterchainGasPaymaster>, FungibleBucket) {
            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(InterchainGasPaymaster::blueprint_id());

            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => format!("Hyperlane InterchainGasPaymaster Owner Badge {}", Runtime::bech32_encode_address(component_address)), locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let component = Self {
                destination_gas_configs: KeyValueStore::new(),
                resource_address: resource,
                vault: FungibleVault::new(resource),
                sequence: 0u32,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(
                owner_badge.resource_address()
            ))))
            .with_address(address_reservation)
            .globalize();

            (component, owner_badge)
        }

        pub fn hook_type(&self) -> Types {
            Types::INTERCHAINGASPAYMASTER
        }

        fn get_config(&self, destination: u32) -> KeyValueEntryRef<DestinationGasConfig> {
            self.destination_gas_configs
                .get(&destination)
                .expect("IGP: no config for destination")
        }

        pub fn set_destination_gas_configs(&mut self, configs: Vec<(u32, DestinationGasConfig)>) {
            for (domain, config) in configs {
                self.destination_gas_configs.insert(domain, config)
            }
        }

        // TODO: I believe decimal is an incorrect use here, we should instead use I192
        pub fn destination_gas_limit(&self, destination: u32, gas_limit: Decimal) -> Decimal {
            let config = self.get_config(destination);
            gas_limit.saturating_add(config.gas_overhead.into())
        }

        pub fn quote_gas_payment(&self, destination: u32, gas_limit: Decimal) -> Decimal {
            let config = self
                .destination_gas_configs
                .get(&destination)
                .expect("IGP: no config for destination");

            // The total cost quoted in destination chain's igp token.
            gas_limit
                .checked_mul(config.gas_oracle.gas_price)
                .and_then(|gas_cost| gas_cost.checked_mul(config.gas_oracle.token_exchange_rate))
                .and_then(|gas_cost| gas_cost.checked_div(EXCHANGE_RATE_SCALE))
                .expect("IGP: decimal overflow when performing gas price calculation")
        }

        pub fn claim(&mut self) -> FungibleBucket {
            self.vault.take_all()
        }

        pub fn pay_for_gas(
            &mut self,
            message_id: Bytes32,
            destination: u32,
            gas_limit: Decimal,
            payment: FungibleBucket,
        ) -> FungibleBucket {
            let required_payment = self.quote_gas_payment(destination, gas_limit);
            if payment.amount() < required_payment {
                panic!(
                    "IGP: payment for gas does not match IGP quote. quote: {}",
                    required_payment
                )
            }
            let mut payment = payment;
            self.vault.put(payment.take(required_payment));

            Runtime::emit_event(GasPayment {
                destination_domain: destination,
                gas_amount: gas_limit,
                message_id,
                payment: required_payment,
                sequence: self.sequence,
                resource_address: self.resource_address,
            });

            self.sequence += 1;

            // return whats left of the payment
            payment
        }

        /// Post dispatch accepts a vec of buckets, that is the payment that the user is willing to pass
        /// We can't assume that payments will happen only in one resource type (one bucket is always only assisated with one resource)
        /// We return the left over Buckets that have not been consumed
        pub fn post_dispatch(
            &mut self,
            metadata: Option<StandardHookMetadata>,
            message: HyperlaneMessage,
            payment: Vec<FungibleBucket>,
        ) -> Vec<FungibleBucket> {
            let mut payment = payment;
            // Find the position of the matching payment in the vector
            let position = payment
                .iter()
                .position(|x| x.resource_address() == self.resource_address)
                .expect("IGP: no payment found for resource address");

            // Remove the payment from the vector to take ownership of it
            let resource_payment = payment.remove(position);
            let gas_limit = metadata
                .map(|x| x.gas_limit)
                .unwrap_or_else(|| DEFAULT_GAS.into());
            // apply gas overhead
            let gas_limit = self.destination_gas_limit(message.destination, gas_limit);
            let result = self.pay_for_gas(
                message.id(),
                message.destination,
                gas_limit,
                resource_payment,
            );

            // append left over payment back to the list of resuorces that have not been used
            payment.push(result);

            payment
        }

        /// Quote dispatch returns a map from resources and their amount that is required in decimals
        /// this ensure that we are not limited to a single payment resource and instead can model multiple resources
        /// that might be needed in order to perfrom a remote transfer
        pub fn quote_dispatch(
            &self,
            metadata: Option<StandardHookMetadata>,
            message: HyperlaneMessage,
        ) -> IndexMap<ResourceAddress, Decimal> {
            let gas_limit = metadata
                .map(|x| x.gas_limit)
                .unwrap_or_else(|| DEFAULT_GAS.into());
            // apply gas overhead for destination
            let gas_limit = self.destination_gas_limit(message.destination, gas_limit);
            let quote = self.quote_gas_payment(message.destination, gas_limit);

            IndexMap::from_iter([(self.resource_address, quote)])
        }
    }
}
