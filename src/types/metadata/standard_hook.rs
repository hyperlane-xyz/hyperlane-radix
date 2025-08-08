use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent, Clone)]
pub struct StandardHookMetadata {
    pub gas_limit: Decimal,
    pub custom_bytes: Option<Vec<u8>>,
}
