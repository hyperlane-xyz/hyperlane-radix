use crate::{
    format_error, panic_error,
    types::Bytes32,
    types::{
        metadata::StandardHookMetadata, warp_payload::WarpPayload, HyperlaneMessage, MessageSender,
    },
};
use scrypto::prelude::*;

#[derive(ScryptoSbor)]
pub enum HypTokenType {
    Collateral {
        collateral_address: ResourceAddress,
    },
    Synthetic {
        name: String,
        symbol: String,
        description: String,
        divisibility: u8,
    },
}

#[derive(ScryptoSbor)]
pub struct RemoteRouter {
    pub domain: u32,
    pub recipient: Bytes32,
    pub gas: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct SendRemoteTransferEvent {
    pub destination_domain: u32,
    pub application_recipient: Bytes32,
    pub user_recipient: Bytes32,
    pub amount: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReceiveRemoteTransferEvent {
    pub origin_domain: u32,
    pub application_sender: Bytes32,
    pub user_recipient: String,
    pub amount: Decimal,
}

#[blueprint]
#[events(SendRemoteTransferEvent, ReceiveRemoteTransferEvent)]
mod hyp_token {

    enable_method_auth! {
        roles {
            mailbox_component => updatable_by: [];
        },
        methods {
            // Public
            transfer_remote => PUBLIC;
            ism => PUBLIC;
            quote_remote_transfer => PUBLIC;
            // Mailbox Only
            handle => restrict_to: [mailbox_component];
            // Owner Only
            set_ism => restrict_to: [OWNER];
            enroll_remote_router => restrict_to: [OWNER];
            unroll_remote_router => restrict_to: [OWNER];
        }
    }

    struct HypToken {
        token_type: HypTokenType,
        // TODO consider making mailbox update-able
        mailbox: ComponentAddress,
        ism: Option<ComponentAddress>,
        enrolled_routers: KeyValueStore<u32, RemoteRouter>,

        vault: FungibleVault,
        resource_manager: Option<FungibleResourceManager>,
    }

    impl HypToken {
        /*
            Instantiate Hyperlane Token Component with one fungible owner badge.
            The owner can enroll, unenroll and update remote routers.
            The owner can set a custom ISM.
        */
        pub fn instantiate(
            token_type: HypTokenType,
            mailbox: ComponentAddress,
        ) -> (Global<HypToken>, FungibleBucket) {
            // Create a mailbox component rule to ensure that the "handle()" function can only
            // be called by the mailbox itself.
            let mailbox_component_rule =
                rule!(require(NonFungibleGlobalId::global_caller_badge(mailbox)));

            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(HypToken::blueprint_id());

            // create a new owner badge
            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => format!(
                        "{} Token Owner Badge",
                        (match token_type {
                            HypTokenType::Synthetic {..} => "Synthetic",
                            HypTokenType::Collateral {..} => "Collateral"
                        }),
                    ), locked;
                    "component" => component_address, locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let mut resource_manager: Option<FungibleResourceManager> = None;
            let vault: FungibleVault = match &token_type {
                HypTokenType::Synthetic {
                    name,
                    symbol,
                    description,
                    divisibility,
                } => {
                    let bucket = ResourceBuilder::new_fungible(OwnerRole::None)
                        .metadata(metadata!(
                            roles {
                                metadata_setter => rule!(require(owner_badge.resource_address()));
                                metadata_setter_updater => rule!(require(owner_badge.resource_address()));
                                metadata_locker => rule!(require(owner_badge.resource_address()));
                                metadata_locker_updater => rule!(require(owner_badge.resource_address()));
                            },
                            init {
                                "name" => name.clone(), updatable;
                                "symbol" => symbol.clone(), updatable;
                                "description" => description.clone(), updatable;
                            }
                        ))
                        .mint_roles(mint_roles! {
                            minter => rule!(require(global_caller(component_address)));
                            minter_updater => rule!(deny_all);
                        })
                        .burn_roles(burn_roles! {
                            burner => rule!(require(global_caller(component_address)));
                            burner_updater => rule!(deny_all);
                        })
                        .divisibility(*divisibility)
                        .mint_initial_supply(0);

                    resource_manager = Some(bucket.resource_manager());

                    FungibleVault::with_bucket(bucket)
                }
                HypTokenType::Collateral { collateral_address } => {
                    FungibleVault::new(*collateral_address)
                }
            };

            let component = Self {
                token_type,
                mailbox,
                vault,
                ism: None,
                enrolled_routers: KeyValueStore::new(),
                resource_manager,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Updatable(rule!(require(
                owner_badge.resource_address()
            ))))
            .roles(roles! {
                mailbox_component => mailbox_component_rule;
            })
            .with_address(address_reservation)
            .globalize();

            (component, owner_badge)
        }

        /*
            Enroll remote router for a given domain to specify the counterpart contract address
        */
        pub fn enroll_remote_router(
            &mut self,
            receiver_domain: u32,
            receiver_address: Bytes32,
            gas: Decimal,
        ) {
            self.enrolled_routers.insert(
                receiver_domain,
                RemoteRouter {
                    domain: receiver_domain,
                    recipient: receiver_address,
                    gas,
                },
            )
        }

        /*
            Remove remote router for a given domain. The component can no longer send or receive
            tokens to the unenrolled destination.
        */
        pub fn unroll_remote_router(&mut self, receiver_domain: u32) {
            self.enrolled_routers.remove(&receiver_domain);
        }

        /*
            Set a custom ISM which is used for verification instead of the default one
            provided by the mailbox.
        */
        pub fn set_ism(&mut self, ism: Option<ComponentAddress>) {
            self.ism = ism;
        }

        /*
            The mailbox calls this function to receive the custom ism address.
        */
        pub fn ism(&mut self) -> Option<ComponentAddress> {
            self.ism
        }

        /*
            Public function called by the end-user to initiate a Hyperlane token transfer
        */
        pub fn transfer_remote(
            &mut self,
            destination: u32,
            recipient: Bytes32,
            amount: FungibleBucket,
            hyp_fee_payment: Vec<FungibleBucket>,
            custom_hook: Option<ComponentAddress>,
            standard_hook_metadata: Option<StandardHookMetadata>,
        ) -> Vec<FungibleBucket> {
            // value stored for later event and payload
            let token_amount = amount.amount();

            match self.token_type {
                HypTokenType::Synthetic { .. } => {
                    // Burn Synthetic token
                    self.resource_manager.unwrap().burn(amount);
                }
                HypTokenType::Collateral { .. } => {
                    // Transfer collateral from user into the vault
                    self.vault.put(amount);
                }
            };

            // Get remote-router to know destination address and expected gas
            let router = self
                .enrolled_routers
                .get(&destination)
                .expect(&format_error!(
                    "no route enrolled for destination {}",
                    destination
                ));

            // Payload for the Hyperlane message
            let payload = WarpPayload::try_new_with_divisibility(
                recipient,
                token_amount,
                self.get_divisibility(),
            )
            .expect("failed to create payload");

            Runtime::emit_event(SendRemoteTransferEvent {
                destination_domain: destination,
                application_recipient: router.recipient,
                user_recipient: recipient,
                amount: token_amount,
            });

            let payload: Vec<u8> = payload.into();

            let standard_hook_metadata =
                standard_hook_metadata.unwrap_or_else(|| StandardHookMetadata {
                    gas_limit: router.gas,
                    custom_bytes: None,
                });

            // Dispatch payload to mailbox
            let result = ScryptoVmV1Api::object_call(
                self.mailbox.as_node_id(),
                "dispatch",
                scrypto_args!(
                    destination,
                    router.recipient,
                    payload,
                    // TODO test if custom hook with metadata is working
                    custom_hook,
                    Some(standard_hook_metadata),
                    hyp_fee_payment,
                    MessageSender::Component(Runtime::global_component())
                ),
            );

            // Return change-money of the interchain fee, if the user provided too much.
            let (_, bucket): (Bytes32, Vec<FungibleBucket>) =
                scrypto_decode(&result).expect(&format_error!("failed to decode dispatch result"));

            bucket
        }

        pub fn quote_remote_transfer(
            &self,
            destination_domain: u32,
            recipient: Bytes32,
            amount: Decimal,
        ) -> IndexMap<ResourceAddress, Decimal> {
            let remote_router = self
                .enrolled_routers
                .get(&destination_domain)
                .expect(&format_error!("no router enrolled for domain"));

            let payload: Vec<u8> =
                WarpPayload::try_new_with_divisibility(recipient, amount, self.get_divisibility())
                    .expect("failed to create warp payload")
                    .into();

            let standard_hook_metadata = StandardHookMetadata {
                gas_limit: remote_router.gas,
                custom_bytes: None,
            };

            let result = ScryptoVmV1Api::object_call(
                self.mailbox.as_node_id(),
                "quote_dispatch",
                scrypto_args!(
                    destination_domain,
                    remote_router.recipient,
                    payload,
                    None::<ComponentAddress>,
                    Some(standard_hook_metadata),
                    MessageSender::Component(Runtime::global_component())
                ),
            );

            scrypto_decode(&result).expect(&format_error!("failed to decode dispatch result"))
        }

        /*
            This method is called by the mailbox when a message is sent to this component.
            Due to resource management in radix, the caller must provide a list
            of all resources the component needs to interact with.
            Contract: If visible_components is empty, the method panics and returns a list
            of required component addresses.
        */
        pub fn handle(&mut self, raw_message: Vec<u8>, visible_components: Vec<ComponentAddress>) {
            let hyperlane_message: HyperlaneMessage = raw_message.into();

            let router =
                self.enrolled_routers
                    .get(&hyperlane_message.origin)
                    .expect(&format_error!(
                        "no enrolled router for domain {:?}",
                        hyperlane_message.origin
                    ));

            assert_eq!(router.recipient, hyperlane_message.sender);

            let warp_payload = WarpPayload::try_from(hyperlane_message.body)
                .expect("failed to parse warp payload");

            if visible_components.is_empty() {
                panic_error!(
                    "RequiredAddresses: {}",
                    Runtime::bech32_encode_address(warp_payload.component_address())
                )
            }

            let amount = warp_payload.get_amount(self.get_divisibility());

            let share: FungibleBucket = match self.token_type {
                HypTokenType::Synthetic { .. } => self.resource_manager.unwrap().mint(amount),
                HypTokenType::Collateral { .. } => self.vault.take(amount),
            };

            let mut account: Global<Account> = warp_payload.component_address().into();
            account.try_deposit_or_abort(share.into(), None);

            Runtime::emit_event(ReceiveRemoteTransferEvent {
                application_sender: hyperlane_message.sender,
                origin_domain: hyperlane_message.origin,
                user_recipient: Runtime::bech32_encode_address(warp_payload.component_address()),
                amount,
            });
        }

        fn get_divisibility(&self) -> u32 {
            self.vault
                .resource_manager()
                .resource_type()
                .divisibility()
                .unwrap() as u32
        }
    }
}
