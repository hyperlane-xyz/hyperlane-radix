use crate::{
    types::{Bytes32},
    types::{metadata::StandardHookMetadata, HyperlaneMessage, warp_payload::WarpPayload},
};
use scrypto::prelude::*;

#[derive(ScryptoSbor)]
pub struct HypSyntheticTokenMetadata {
    name: String,
    symbol: String,
    description: String,
    divisibility: u8,
}

#[derive(ScryptoSbor)]
pub enum HypTokenType {
    COLLATERAL(ResourceAddress),
    SYNTHETIC(HypSyntheticTokenMetadata),
}

#[derive(ScryptoSbor)]
pub struct RemoteRouter {
    pub domain: u32,
    pub recipient: Bytes32,
    pub gas: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct SendRemoteTransferEvent {
    pub sender: Bytes32,
    pub destination_domain: u32,
    pub recipient: Bytes32,
    pub amount: Decimal,
}

#[blueprint]
#[events(SendRemoteTransferEvent)]
mod hyp_token {

    enable_method_auth! {
        methods {
            transfer_remote => PUBLIC;
            handle => PUBLIC;
            enroll_remote_router => restrict_to: [OWNER];
            unroll_remote_router => restrict_to: [OWNER];
        }
    }

    struct HypToken {
        token_type: HypTokenType,
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
        pub fn instantiate(token_type: HypTokenType, mailbox: ComponentAddress) -> (Global<HypToken>, FungibleBucket) {

            // create new owner badge
            let owner_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init {
                    "name" => "Hyperlane Collateral Token - Owner Badge", locked;
                }))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            // reserve an address for the component
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(HypToken::blueprint_id());

            let mut resource_manager: Option<FungibleResourceManager> = None;
            let vault: FungibleVault = match &token_type {
                HypTokenType::SYNTHETIC(metadata) => {
                    let bucket = ResourceBuilder::new_fungible(OwnerRole::None)
                        .metadata(metadata!(
                            init {
                                "name" => metadata.name.clone(), locked;
                                "symbol" => metadata.symbol.clone(), locked;
                                "description" => metadata.description.clone(), locked;
                            }
                        ))
                        .mint_roles(mint_roles! {
                        minter => rule!(require(global_caller(component_address)));
                        minter_updater => rule!(deny_all);
                    }).burn_roles(burn_roles! {
                        burner => rule!(require(global_caller(component_address)));
                        burner_updater => rule!(deny_all);
                    })
                        .divisibility(metadata.divisibility)
                        .mint_initial_supply(0);

                    resource_manager = Some(bucket.resource_manager());

                    FungibleVault::with_bucket(bucket)
                },
                HypTokenType::COLLATERAL(resource_address) => {
                    FungibleVault::new(*resource_address)
                }
            };

            // populate a GumballMachine struct and instantiate a new component
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
            self.enrolled_routers.insert(receiver_domain, RemoteRouter {
                domain: receiver_domain,
                recipient: receiver_address,
                gas
            })
        }

        /*
            Remove remote router for a given domain. The component can no longer send or receive
            tokens to the unenrolled destination.
        */
        pub fn unroll_remote_router(
            &mut self,
            receiver_domain: u32,
        ) {
            self.enrolled_routers.remove(&receiver_domain);
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
                HypTokenType::SYNTHETIC(_) => {
                    // Burn Synthetic token
                    self.resource_manager.unwrap().burn(amount);
                },
                HypTokenType::COLLATERAL(_) => {
                    // Transfer collateral from user into the vault
                    self.vault.put(amount);
                }
            };

            // Get remote-router to know destination address and expected gas
            let router = self.
                enrolled_routers.
                get(&mut destination.clone()).
                expect("No route enrolled for destination");

            // Payload for the Hyperlane message
            let payload = WarpPayload::new(recipient, token_amount);
            let payload: Vec<u8> = payload.into();

            Runtime::emit_event(SendRemoteTransferEvent {
                sender: Bytes32::zero(),
                destination_domain: destination,
                recipient,
                amount: token_amount,
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
                    standard_hook_metadata,
                    hyp_fee_payment
                ),
            );

            // Return change-money of the interchain fee, if the user provided too much.
            let (_, bucket): (Bytes32, Vec<FungibleBucket>) = scrypto_decode(&result)
                .expect("Failed to decode dispatch result");

            bucket
        }

        /*
            This method is called by the mailbox when a message is sent to this component.
            Due to resource management in radix, the caller must provide a list
            of all resources the component needs to interact with.
            Contract: If visible_components is empty, the method panics and returns a list
            of required component addresses.
        */
        pub fn handle(
            &mut self,
            raw_message: Vec<u8>,
            visible_components: Vec<ComponentAddress>
        ) {
            // TODO verify mailbox caller ownership

            let hyperlane_message: HyperlaneMessage = raw_message.into();

            let router = self.enrolled_routers.get(&hyperlane_message.origin)
                .expect(&format!("No enrolled router for domain {:?}", hyperlane_message.origin));

            assert_eq!(router.recipient, hyperlane_message.sender);

            let warp_payload: WarpPayload = hyperlane_message.body.clone().into();

            if visible_components.len() == 0 {
                panic!("RequiredAddresses: {}", Runtime::bech32_encode_address(warp_payload.component_address()))
            }

            let share: FungibleBucket = match self.token_type {
                HypTokenType::SYNTHETIC(_) => self.resource_manager.unwrap().mint(warp_payload.amount),
                HypTokenType::COLLATERAL(_) => self.vault.take(warp_payload.amount),
            };

            ScryptoVmV1Api::object_call(
                warp_payload.component_address().as_node_id(),
                "try_deposit_or_abort",
                scrypto_args!(share, None::<ResourceOrNonFungible>),
            );

            // TODO add receive event
        }
    }
}
