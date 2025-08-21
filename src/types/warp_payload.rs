use scrypto::prelude::*;
use std::ops::Div;

use crate::types::Bytes32;

#[derive(Debug, PartialEq)]
pub enum WarpPayloadError {
    PayloadTooShort,
    DivisibilityTooHigh(u32),
    DivisibilityTooLowForAmount(Decimal, u32),
    PayloadAmountTooLarge,
}

/// A full Hyperlane message between chains
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WarpPayload {
    /// 32-byte Address in destination convention
    pub recipient: Bytes32,
    /// 32-byte Amount
    amount: I192,
}

impl WarpPayload {
    pub fn try_new_with_divisibility(
        recipient: Bytes32,
        amount: Decimal,
        divisibility: u32,
    ) -> Result<Self, WarpPayloadError> {
        if divisibility > Decimal::SCALE {
            return Err(WarpPayloadError::DivisibilityTooHigh(divisibility));
        }

        let divisor = I192::from(10u64.pow(Decimal::SCALE - divisibility));

        if amount.attos() % divisor != I192::zero() {
            return Err(WarpPayloadError::DivisibilityTooLowForAmount(
                amount,
                divisibility,
            ));
        }

        let amount = amount
            .attos()
            .div(I192::from(10u64.pow(Decimal::SCALE - divisibility)));

        Ok(Self { recipient, amount })
    }

    pub fn component_address(&self) -> ComponentAddress {
        // Extract Component address first 32 bytes.
        // Although radix only uses 30 bytes for the address.
        // The first two bytes are zero.
        let mut arr = [0u8; 30];
        arr.copy_from_slice(&self.recipient.as_ref()[2..32]);
        ComponentAddress::new_or_panic(arr)
    }

    pub fn get_amount(&self, divisibility: u32) -> Decimal {
        use std::ops::Mul;
        Decimal::from_attos(
            self.amount
                .mul(I192::from(10u64.pow(Decimal::SCALE - divisibility))),
        )
    }
}

impl TryFrom<Vec<u8>> for WarpPayload {
    type Error = WarpPayloadError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() < 64 {
            return Err(WarpPayloadError::PayloadTooShort);
        }

        let recipient: Bytes32 = bytes[0..32].into();

        // The next 32 bytes encode the amount.
        // In the future it might be possible that the warp payload carries additional metadata.
        let mut b = bytes[32..64].to_vec();
        b.reverse();
        let amount = U256::from_le_bytes(b.as_ref());

        let amount = I192::try_from(amount).map_err(|_| WarpPayloadError::PayloadAmountTooLarge)?;

        // I192 is signed, but Warp only support positive amounts. If the amount is negative,
        // the payload is rejected.
        if amount.is_negative() {
            return Err(WarpPayloadError::PayloadAmountTooLarge);
        }

        Ok(Self { recipient, amount })
    }
}

impl From<WarpPayload> for Vec<u8> {
    fn from(w: WarpPayload) -> Self {
        let mut amount = w.amount.to_le_bytes();
        amount.reverse();

        let mut message_vec: Vec<u8> = vec![];
        message_vec.extend_from_slice(w.recipient.as_ref());
        // For the Radix implementation the payload only supports 24 (instead of 32 bytes)
        // Therefore, we pad the amount with 8 zero bytes.
        message_vec.extend_from_slice(&[0; 8]);
        message_vec.extend_from_slice(&amount);
        message_vec
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
        let payload = WarpPayload::try_new_with_divisibility(address, amount, 18).unwrap();
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
        let payload = WarpPayload::try_new_with_divisibility(address, amount, 18).unwrap();
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
    pub fn warp_payload_maximum_amount() {
        // Arrange

        // 0000000000000000 7fffffffffffffff ffffffffffffffff ffffffffffffffff
        // hex(2**191 - 1)
        let bytes: Vec<u8> =
            hex::decode("00000000000000007fffffffffffffffffffffffffffffffffffffffffffffff")
                .unwrap();
        let mut raw_message: Vec<u8> = vec![0; 32];
        raw_message.extend_from_slice(&bytes.as_ref());

        // Act
        let payload = WarpPayload::try_from(raw_message);

        // Assert
        assert!(payload.is_ok());
    }

    #[test]
    pub fn warp_payload_maximum_amount_plus_one() {
        // Arrange

        // 0000000000000000 8000000000000000 0000000000000000 0000000000000000
        // hex(2**191 - 1 + 1)
        let bytes: Vec<u8> =
            hex::decode("0000000000000000800000000000000000000000000000000000000000000000")
                .unwrap();
        let mut raw_message: Vec<u8> = vec![0; 32];
        raw_message.extend_from_slice(&bytes.as_ref());

        // Act
        let payload = WarpPayload::try_from(raw_message);

        // Assert
        assert_eq!(
            payload.unwrap_err(),
            WarpPayloadError::PayloadAmountTooLarge
        );
    }

    #[test]
    pub fn warp_payload_use_smaller_decimals_than_allowed() {
        // Arrange & Act
        let w = WarpPayload::try_new_with_divisibility(
            Bytes32::zero(),
            Decimal::from_subunits(I192::one()),
            17,
        );
        // Assert
        assert_eq!(
            w.unwrap_err(),
            WarpPayloadError::DivisibilityTooLowForAmount(Decimal::from_subunits(I192::one()), 17)
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
        let payload = WarpPayload::try_new_with_divisibility(address, amount, 18).unwrap();
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
        let payload = WarpPayload::try_from(bytes).unwrap();
        let component_address = payload.component_address();

        // Assert
        assert_eq!(account, component_address);
        assert_eq!(payload.amount, I192::from(10u64.pow(18)));
    }
}
