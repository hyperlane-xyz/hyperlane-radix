use crate::types::metadata::StandardHookMetadata;
use crate::types::Bytes32;
use crate::types::{HyperlaneMessage, MESSAGE_VERSION};
use scrypto::blueprint;
use scrypto::prelude::*;

/// TODO: populate this field with block height and tx sender address
#[derive(ScryptoSbor, ScryptoEvent)]
struct Delivery {}

#[blueprint]
#[events(
    InstantiationEvent,
    DispatchEvent,
    DispatchIdEvent,
    ProcessIdEvent,
    ProcessEvent
)]
mod mailbox {

    // better mod names
    enable_method_auth! {
        // decide which methods are public and which are restricted to the component's owner
        methods {
            // Public Lookup
            local_domain => PUBLIC;
            delivered => PUBLIC;

            // ISM
            default_ism => PUBLIC;
            set_default_ism => restrict_to: [OWNER];

            // Default Hook
            default_hook => PUBLIC;
            set_default_hook => restrict_to: [OWNER];

            // Required Hook
            required_hook => PUBLIC;
            set_required_hook => restrict_to: [OWNER];

            is_latest_dispatched => PUBLIC;
            dispatch => PUBLIC;
            quote_dispatch => PUBLIC;
            process => PUBLIC;
            recipient_ism => PUBLIC;
        }
    }

    struct Mailbox {
        local_domain: u32,
        nonce: u32,

        default_ism: Option<ComponentAddress>,
        default_hook: Option<ComponentAddress>,
        required_hook: Option<ComponentAddress>,

        processed_messages: KeyValueStore<Bytes32, ()>,

        // latests dispatched message, used for auth in hooks
        latest_dispatched_message: Bytes32,
    }

    impl Mailbox {
        /// Instantiates a new Mailbox component with the given local domain.
        pub fn mailbox_instantiate(local_domain: u32) -> (Global<Mailbox>, FungibleBucket) {

            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Mailbox::blueprint_id());

            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => format!("Hyperlane Mailbox Owner Badge {}", Runtime::bech32_encode_address(component_address)), locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let component = Self {
                local_domain,
                nonce: 0,
                default_ism: None,
                default_hook: None,
                required_hook: None,
                processed_messages: KeyValueStore::new(),
                latest_dispatched_message: Bytes32::zero(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(
                owner_badge.resource_address()
            ))))
            .with_address(address_reservation)
            .globalize();

            // Return the global component and the owner badge
            (component, owner_badge)
        }

        /// Returns the local domain of the Mailbox component
        pub fn local_domain(&self) -> u32 {
            self.local_domain
        }

        pub fn delivered(&self, message_id: Bytes32) -> bool {
            self.processed_messages.get(&message_id).is_some()
        }

        pub fn default_ism(&self) -> Option<ComponentAddress> {
            self.default_ism
        }

        pub fn set_default_ism(&mut self, address: ComponentAddress) {
            self.default_ism = Some(address);
        }

        pub fn default_hook(&self) -> Option<ComponentAddress> {
            self.default_hook
        }

        pub fn set_default_hook(&mut self, address: ComponentAddress) {
            self.default_hook = Some(address);
        }

        pub fn required_hook(&self) -> Option<ComponentAddress> {
            self.required_hook
        }

        pub fn set_required_hook(&mut self, address: ComponentAddress) {
            self.required_hook = Some(address);
        }

        pub fn is_latest_dispatched(&self, id: Bytes32) -> bool {
            self.latest_dispatched_message == id
        }

        pub fn dispatch(
            &mut self,
            destination_domain: u32,
            recipient_address: Bytes32,
            message_body: Vec<u8>,
            hook: Option<ComponentAddress>,
            hook_metadata: Option<StandardHookMetadata>,
            payment: Vec<FungibleBucket>,
        ) -> (Bytes32, Vec<FungibleBucket>) {

            let hyperlane_message = HyperlaneMessage::new(
                self.nonce,
                self.local_domain,
                Bytes32::zero(), // TODO: sender address, this seems to be more tricky than anticipated
                destination_domain,
                recipient_address,
                message_body,
            );

            let message_id = hyperlane_message.id();
            self.latest_dispatched_message = message_id;
            self.nonce += 1;

            let mut payment = payment;

            let default_hook = hook.or(self.default_hook);
            if let Some(default_hook) = default_hook {
                // TODO: maybe we can use Global<T> for this, not sure tho
                let result = ScryptoVmV1Api::object_call(
                    default_hook.as_node_id(),
                    "post_dispatch",
                    scrypto_args!(hook_metadata.clone(), hyperlane_message.clone(), payment),
                );

                payment = scrypto_decode(&result).expect("Failed to decode post_dispatch result");
            }
            if let Some(required_hook) = self.required_hook {
                // TODO: maybe we can use Global<T> for this, not sure tho
                let result = ScryptoVmV1Api::object_call(
                    required_hook.as_node_id(),
                    "post_dispatch",
                    scrypto_args!(hook_metadata, hyperlane_message.clone(), payment),
                );

                payment = scrypto_decode(&result).expect("Failed to decode post_dispatch result");
            }

            Runtime::emit_event(DispatchEvent {
                destination: destination_domain,
                recipient: recipient_address,
                message: hyperlane_message.into(),
            });

            (message_id, payment)
        }

        /// Quote dispatch returns a map from resources and their amount that is required in decimals
        /// this ensure that we are not limited to a single payment resource and instead can model multiple resources
        /// that might be needed in order to perfrom a remote transfer
        pub fn quote_dispatch(
            &self,
            destination_domain: u32,
            recipient_address: Bytes32,
            message_body: Vec<u8>,
            hook: Option<ComponentAddress>,
            hook_metadata: Option<StandardHookMetadata>,
        ) -> IndexMap<ResourceAddress, Decimal> {
            let hyperlane_message = HyperlaneMessage::new(
                self.nonce,
                self.local_domain,
                Bytes32::zero(), // TODO: sender address, this seems to be more tricky than anticipated
                destination_domain,
                recipient_address,
                message_body,
            );

            let mut quote = IndexMap::new();
            let default_hook = hook.or(self.default_hook);
            if let Some(default_hook) = default_hook {
                // TODO: maybe we can use Global::<T> for this, not sure tho
                let result = ScryptoVmV1Api::object_call(
                    default_hook.as_node_id(),
                    "quote_dispatch",
                    scrypto_args!(hook_metadata.clone(), hyperlane_message.clone()),
                );

                quote = scrypto_decode(&result).expect("Failed to decode post_dispatch result");
            }
            if let Some(required_hook) = self.required_hook {
                // TODO: maybe we can use Global<T> for this, not sure tho
                let result = ScryptoVmV1Api::object_call(
                    required_hook.as_node_id(),
                    "quote_dispatch",
                    scrypto_args!(hook_metadata, hyperlane_message.clone()),
                );

                let required_hook_quote: IndexMap<ResourceAddress, Decimal> =
                    scrypto_decode(&result).expect("Failed to decode post_dispatch result");

                for (key, value) in required_hook_quote.iter() {
                    quote
                        .entry(*key)
                        .and_modify(|existing| *existing += *value) // TODO: double check if this can result in overflow
                        .or_insert(*value);
                }
            }
            quote
        }

        pub fn process(
            &mut self, 
            metadata: Vec<u8>, 
            raw_message: Vec<u8>, 
            visible_components: Vec<ComponentAddress>
        ) -> () {
            let message: HyperlaneMessage = raw_message.clone().into();

            if self.local_domain != message.destination {
                panic!("Message destination domain does not match local domain");
            }

            if message.version != MESSAGE_VERSION {
                panic!("Unsupported message version");
            }

            let message_id = message.id();
            if self.delivered(message_id) {
                panic!("Message already processed");
            }
            self.processed_messages.insert(message_id, ());

            let recipient_component: ComponentAddress = message.recipient.into();

            // Call the ISM to verify the message
            let recipient_ism = self.recipient_ism(recipient_component).expect("Neither mailbox nor receiver have specified an ISM");
            let result = ScryptoVmV1Api::object_call(
                recipient_ism.as_node_id(),
                "verify",
                scrypto_args!(metadata, raw_message.clone()),
            );

            let result: bool =
                scrypto_decode(&result).expect("Failed to decode ISM verification result");
            if !result {
                panic!("Mailbox: ISM verification failed");
            }

            Runtime::emit_event(ProcessEvent {
                origin: message.origin,
                sender: message.sender,
                recipient: message.recipient,
            });
            Runtime::emit_event(ProcessIdEvent { message_id });

            ScryptoVmV1Api::object_call(
                recipient_component.as_node_id(),
                "handle",
                scrypto_args!(raw_message, visible_components),
            );

            ()
        }

        /// Returns the ISM (Interchain Security Module) for the given recipient address.
        pub fn recipient_ism(&self, recipient: ComponentAddress) -> Option<ComponentAddress> {
            // uses low-level calls to fetch the ISM for the given recipient address
            let result =
                ScryptoVmV1Api::object_call(recipient.as_node_id(), "ism", scrypto_args!());

            let return_value: Option<ComponentAddress> = scrypto_decode(&result).unwrap(); // TODO: error handling

            return_value.or(self.default_ism)
        }
    }
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct InstantiationEvent {
    pub local_domain: u32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct DispatchEvent {
    pub destination: u32,
    pub recipient: Bytes32, // TODO: maybe encode this as hex string. I know there are built in function for parsing Component / Resource Addresses from and to hex strings
    pub message: Vec<u8>,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct DispatchIdEvent {
    pub message_id: Bytes32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ProcessIdEvent {
    pub message_id: Bytes32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ProcessEvent {
    pub origin: u32,
    pub sender: Bytes32,
    pub recipient: Bytes32,
}
