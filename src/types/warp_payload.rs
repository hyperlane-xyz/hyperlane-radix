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
        let recipient: Bytes32 = m[0..32].into();

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
        message_vec.extend_from_slice(&amount);
        message_vec
    }
}

impl From<WarpPayload> for Vec<u8> {
    fn from(w: WarpPayload) -> Self {
        RawWarpPayload::from(&w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    #[test]
    pub fn warp_payload_new_zero() {
        // Arrange & Act
        let address: Bytes32 = Bytes32::zero();
        let amount = Decimal::zero();
        let payload = WarpPayload::new(address, amount);
        let bytes: Vec<u8> = payload.into();

        // Assert
        assert_eq!(bytes.len(), 64);
        assert!(bytes.iter().all(|&x| x == 0), "Not all bytes are zero");
    }

    #[test]
    pub fn warp_payload_amount_encoding() {
        // Arrange & Act
        let address: Bytes32 = [1; 32].into();
        let amount = Decimal::one();
        let payload = WarpPayload::new(address, amount);
        let bytes: Vec<u8> = payload.into();

        // Assert
        assert_eq!(bytes.len(), 64);
        assert!(
            bytes[..32].iter().all(|&x| x == 1),
            "Not all bytes are zero"
        );
        assert!(
            bytes[32..56].iter().all(|&x| x == 0),
            "First amount bytes have to be zero"
        );
        assert_eq!(
            bytes[56..64].to_vec(),
            vec![13, 224, 182, 179, 167, 100, 0, 0]
        );
    }

    #[test]
    pub fn warp_payload_component_address() {
        // Arrange
        let rb: [u8; 30] =
            hex::decode("c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d42")
                .unwrap()
                .try_into()
                .unwrap();
        let account: ComponentAddress = ComponentAddress::new_or_panic(rb);
        let address: Bytes32 = account.into();
        let amount = Decimal::zero();
        let payload = WarpPayload::new(address, amount);
        let bytes: Vec<u8> = payload.clone().into();

        // Act
        let component_address = payload.component_address();

        // Assert
        assert_eq!(account, component_address);
        assert_eq!(
            component_address.to_vec(),
            hex::decode("c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d42").unwrap()
        );
        assert_eq!(bytes.len(), 64);
        assert!(
            bytes[32..64].iter().all(|&x| x == 0),
            "Amount has zo be zero"
        );

        let hex_str = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        assert_eq!(hex_str, "0000c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d420000000000000000000000000000000000000000000000000000000000000000")
    }

    #[test]
    pub fn warp_payload_parse() {
        // Arrange
        // Account creation is deterministic and matches the address tested below
        let rb: [u8; 30] =
            hex::decode("c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d42")
                .unwrap()
                .try_into()
                .unwrap();
        let account: ComponentAddress = ComponentAddress::new_or_panic(rb);

        let raw_message = "0000c1f7abd48c518b8ebdc6a35abfbe78583725a97eabdc99224571e0d11d420000000000000000000000000000000000000000000000000de0b6b3a7640000";
        let bytes = hex::decode(raw_message).unwrap();

        // Act
        let payload = WarpPayload::from(&bytes);
        let component_address = payload.component_address();

        // Assert
        assert_eq!(account, component_address);
        assert_eq!(payload.amount, Decimal::one());
    }
}
