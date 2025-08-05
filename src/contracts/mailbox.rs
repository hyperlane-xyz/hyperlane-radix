use crate::types::metadata::StandardHookMetadata;
use crate::types::Bytes32;
use crate::types::{HyperlaneMessage, MessageSender, MESSAGE_VERSION};
use crate::{format_error, panic_error};
use scrypto::blueprint;
use scrypto::prelude::*;

#[blueprint]
#[events(
    InstantiationEvent,
    DispatchEvent,
    DispatchIdEvent,
    ProcessIdEvent,
    ProcessEvent
)]
mod mailbox {

    enable_method_auth! {
        // decide which methods are public and which are restricted to the component's owner
        methods {
            // Public Lookup
            local_domain => PUBLIC;
            delivered => PUBLIC;
            nonce => PUBLIC;
            processed => PUBLIC;

            // ISM
            default_ism => PUBLIC;
            set_default_ism => restrict_to: [OWNER];

            // Default Hook
            default_hook => PUBLIC;
            set_default_hook => restrict_to: [OWNER];

            // Required Hook
            required_hook => PUBLIC;
            set_required_hook => restrict_to: [OWNER];

            latest_dispatched_id => PUBLIC;
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

        // sequence for process, used for better indexing
        process_sequence: u32,
    }

    impl Mailbox {
        /// Instantiates a new Mailbox component with the given local domain.
        pub fn instantiate(local_domain: u32) -> (Global<Mailbox>, FungibleBucket) {
            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Mailbox::blueprint_id());
            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => "Mailbox Owner Badge", locked;
                    "component" => component_address, locked;
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
                process_sequence: 0,
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

        pub fn nonce(&self) -> u32 {
            self.nonce
        }

        pub fn processed(&self) -> u32 {
            self.process_sequence
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

        pub fn latest_dispatched_id(&self) -> Bytes32 {
            self.latest_dispatched_message
        }

        pub fn dispatch(
            &mut self,
            destination_domain: u32,
            recipient_address: Bytes32,
            message_body: Vec<u8>,
            hook: Option<ComponentAddress>,
            hook_metadata: Option<StandardHookMetadata>,
            payment: Vec<FungibleBucket>,
            claimed_account_address: MessageSender,
        ) -> (Bytes32, Vec<FungibleBucket>) {
            // Important!: Assert that the claimed caller address is indeed the caller of the dispatch function.
            let verified_sender = self.verify_message_sender(claimed_account_address);

            let hyperlane_message = HyperlaneMessage::new(
                self.nonce,
                self.local_domain,
                verified_sender.into(),
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
                let result = ScryptoVmV1Api::object_call(
                    default_hook.as_node_id(),
                    "post_dispatch",
                    scrypto_args!(hook_metadata.clone(), hyperlane_message.clone(), payment),
                );
                payment = scrypto_decode(&result)
                    .expect(&format_error!("failed to decode post_dispatch result"));
            }
            if let Some(required_hook) = self.required_hook {
                let result = ScryptoVmV1Api::object_call(
                    required_hook.as_node_id(),
                    "post_dispatch",
                    scrypto_args!(hook_metadata, hyperlane_message.clone(), payment),
                );
                payment = scrypto_decode(&result)
                    .expect(&format_error!("failed to decode post_dispatch result"));
            }

            // the dispatch sequence is equal to the nonce of the message
            Runtime::emit_event(DispatchIdEvent {
                message_id,
                sequence: hyperlane_message.nonce,
            });

            Runtime::emit_event(DispatchEvent {
                destination: destination_domain,
                recipient: recipient_address,
                sequence: hyperlane_message.nonce,
                message: hyperlane_message.into(),
            });

            (message_id, payment)
        }

        /// Quote dispatch returns a map from resources and their amount that is required in decimals
        /// this ensure that we are not limited to a single payment resource and instead can model multiple resources
        /// that might be needed in order to perform a remote transfer
        pub fn quote_dispatch(
            &self,
            destination_domain: u32,
            recipient_address: Bytes32,
            message_body: Vec<u8>,
            hook: Option<ComponentAddress>,
            hook_metadata: Option<StandardHookMetadata>,
            claimed_account_address: MessageSender,
        ) -> IndexMap<ResourceAddress, Decimal> {
            // Important!: Assert that the claimed caller address is indeed the caller of the dispatch function.
            let verified_sender = self.verify_message_sender(claimed_account_address);

            let hyperlane_message = HyperlaneMessage::new(
                self.nonce,
                self.local_domain,
                verified_sender.into(),
                destination_domain,
                recipient_address,
                message_body,
            );

            let mut quote = IndexMap::new();
            let default_hook = hook.or(self.default_hook);
            if let Some(default_hook) = default_hook {
                let result = ScryptoVmV1Api::object_call(
                    default_hook.as_node_id(),
                    "quote_dispatch",
                    scrypto_args!(hook_metadata.clone(), hyperlane_message.clone()),
                );

                quote = scrypto_decode(&result)
                    .expect(&format_error!("failed to decode post_dispatch result"));
            }
            if let Some(required_hook) = self.required_hook {
                let result = ScryptoVmV1Api::object_call(
                    required_hook.as_node_id(),
                    "quote_dispatch",
                    scrypto_args!(hook_metadata, hyperlane_message.clone()),
                );

                let required_hook_quote: IndexMap<ResourceAddress, Decimal> =
                    scrypto_decode(&result)
                        .expect(&format_error!("failed to decode post_dispatch result"));

                for (key, value) in required_hook_quote.iter() {
                    quote
                        .entry(*key)
                        .and_modify(|existing| *existing += *value) // TODO: double check if this can result in overflow
                        .or_insert(*value);
                }
            }
            quote
        }

        fn verify_message_sender(&self, claimed_sender: MessageSender) -> ComponentAddress {
            let verified_sender: ComponentAddress = match claimed_sender {
                MessageSender::Component(component) => {
                    // Important!: Assert that the claimed caller address is indeed the caller of the dispatch function.
                    Runtime::assert_access_rule(rule!(require(global_caller(component.address()))));
                    component.address()
                }
                MessageSender::Account(account) => {
                    // Important!: Assert that the claimed (account) caller address is indeed the caller of the dispatch function.
                    let OwnerRoleEntry { rule, .. } = account.get_owner_role();

                    Runtime::assert_access_rule(rule);

                    account.address()
                }
            };

            verified_sender
        }

        pub fn process(
            &mut self,
            metadata: Vec<u8>,
            raw_message: Vec<u8>,
            visible_components: Vec<ComponentAddress>,
        ) {
            let message: HyperlaneMessage = raw_message.clone().into();

            if self.local_domain != message.destination {
                panic_error!("message destination domain does not match local domain");
            }

            if message.version != MESSAGE_VERSION {
                panic_error!("unsupported message version");
            }

            let message_id = message.id();
            if self.delivered(message_id) {
                panic_error!("message already processed");
            }
            self.processed_messages.insert(message_id, ());

            let recipient_component: ComponentAddress = message.recipient.into();

            // Call the ISM to verify the message
            let recipient_ism = self
                .recipient_ism(recipient_component)
                .expect(&format_error!(
                    "neither mailbox nor receiver have specified an ISM"
                ));
            let result = ScryptoVmV1Api::object_call(
                recipient_ism.as_node_id(),
                "verify",
                scrypto_args!(metadata, raw_message.clone()),
            );

            let result: bool = scrypto_decode(&result)
                .expect(&format_error!("failed to decode ISM verification result"));
            if !result {
                panic_error!("ISM verification failed");
            }

            Runtime::emit_event(ProcessEvent {
                origin: message.origin,
                sender: message.sender,
                recipient: message.recipient,
                sequence: self.process_sequence,
            });
            Runtime::emit_event(ProcessIdEvent {
                message_id,
                sequence: self.process_sequence,
            });

            self.process_sequence += 1;

            ScryptoVmV1Api::object_call(
                recipient_component.as_node_id(),
                "handle",
                scrypto_args!(raw_message, visible_components),
            );
        }

        /// Returns the ISM (Interchain Security Module) for the given recipient address.
        pub fn recipient_ism(&self, recipient: ComponentAddress) -> Option<ComponentAddress> {
            // uses low-level calls to fetch the ISM for the given recipient address
            let result =
                ScryptoVmV1Api::object_call(recipient.as_node_id(), "ism", scrypto_args!());

            let return_value: Option<ComponentAddress> = scrypto_decode(&result)
                .expect(&format_error!("failed to get ISM from recipient component"));

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
    pub recipient: Bytes32,
    pub message: Vec<u8>,
    pub sequence: u32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct DispatchIdEvent {
    pub message_id: Bytes32,
    pub sequence: u32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ProcessIdEvent {
    pub message_id: Bytes32,
    pub sequence: u32,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ProcessEvent {
    pub origin: u32,
    pub sender: Bytes32,
    pub recipient: Bytes32,
    pub sequence: u32,
}
