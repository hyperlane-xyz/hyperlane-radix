use scrypto::prelude::*;

/// TODO: double check if we want to keep this internal type or instead should adapt the EVM one
#[derive(ScryptoSbor, ScryptoEvent, Clone)]
pub struct StandardHookMetadata {
    pub gas_limit: Decimal,
    pub custom_bytes: Option<Vec<u8>>,
}
