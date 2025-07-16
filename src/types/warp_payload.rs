use scrypto::prelude::*;

use crate::types::Bytes32;

pub type RawWarpPayload = Vec<u8>;

/// A full Hyperlane message between chains
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WarpPayload {
    /// 32-byte Address in destination convention
    pub recipient: Bytes32,
    /// 32-byte Amount
    pub amount: Decimal,
}

impl WarpPayload {
    pub fn new(recipient: Bytes32, amount: Decimal) -> Self {
        Self { recipient, amount }
    }

    pub fn component_address(&self) -> ComponentAddress {
        // Extract Component address first 32 bytes.
        // Although radix only uses 30 bytes for the address.
        // The first two bytes are zero.
        let mut arr = [0u8; 30];
        arr.copy_from_slice(&self.recipient.as_ref()[2..32]);
        ComponentAddress::new_or_panic(arr)
    }
}

impl From<RawWarpPayload> for WarpPayload {
    fn from(m: RawWarpPayload) -> Self {
        WarpPayload::from(&m)
    }
}

impl From<&RawWarpPayload> for WarpPayload {
    fn from(m: &RawWarpPayload) -> Self {
        let recipient: Bytes32 = m[0..32].try_into().unwrap();

        // Next 32 bytes encode the amount
        // In the future it might be possible that the warp payload carries additional metadata
        let mut b = m[32..64].to_vec();
        b.reverse();
        let amount = U256::from_le_bytes(b.as_ref());

        let amount = I192::try_from(amount).expect("Invalid payload");
        let amount = Decimal::from_attos(amount);

        Self { recipient, amount }
    }
}

impl From<&WarpPayload> for RawWarpPayload {
    fn from(w: &WarpPayload) -> Self {
        let mut amount = U256::try_from(w.amount.attos())
            .unwrap()
            .to_le_bytes()
            .to_vec();
        amount.reverse();

        let mut message_vec: Vec<u8> = vec![];
        message_vec.extend_from_slice(w.recipient.as_ref());
        message_vec.extend_from_slice(&*amount);
        message_vec
    }
}

impl Into<Vec<u8>> for WarpPayload {
    fn into(self) -> Vec<u8> {
        RawWarpPayload::from(&self)
    }
}
