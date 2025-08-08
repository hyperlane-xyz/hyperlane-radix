use scrypto::crypto::{keccak256_hash, IsHash};
use scrypto::prelude::*;

use crate::types::eth::{domain_hash, eth_hash};

#[derive(ScryptoSbor)]
pub enum MessageSender {
    Component(Global<AnyComponent>),
    Account(Global<Account>),
}

pub const MESSAGE_VERSION: u8 = 3;

/// A Stamped message that has been committed at some nonce
pub type RawHyperlaneMessage = Vec<u8>;

#[derive(Clone, Eq, PartialEq, Hash, Sbor, ScryptoEvent, PartialOrd, Ord, Copy, Default, Debug)]
#[sbor(transparent)]
pub struct Bytes32([u8; 32]);

impl Bytes32 {
    /// Returns a zeroed out Bytes32
    pub fn zero() -> Self {
        Bytes32([0u8; 32])
    }
}

impl From<ComponentAddress> for Bytes32 {
    fn from(value: ComponentAddress) -> Self {
        let mut bytes = [0u8; 32];
        let src = value.as_bytes();
        let len = src.len();
        // Copy component address bytes starting from position 2 in the destination array
        bytes[32 - len..].copy_from_slice(src);
        Bytes32(bytes)
    }
}

impl From<Hash> for Bytes32 {
    fn from(value: Hash) -> Self {
        Bytes32(value.0)
    }
}

impl From<Bytes32> for Hash {
    fn from(value: Bytes32) -> Self {
        Hash(value.0)
    }
}

impl From<[u8; 32]> for Bytes32 {
    fn from(bytes: [u8; 32]) -> Self {
        Bytes32(bytes)
    }
}

impl From<&[u8; 32]> for Bytes32 {
    fn from(bytes: &[u8; 32]) -> Self {
        Bytes32(*bytes)
    }
}

impl From<&[u8]> for Bytes32 {
    fn from(bytes: &[u8]) -> Self {
        let bytes: [u8; 32] = bytes.try_into().expect("Unable to parse bytes to bytes32");

        Bytes32(bytes)
    }
}

impl AsRef<[u8]> for Bytes32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Bytes32> for ComponentAddress {
    fn from(bytes: Bytes32) -> Self {
        // component addresses are 30 bytes long
        // remove the first 2 bytes
        let mut address_bytes = [0u8; NodeId::LENGTH];
        address_bytes.copy_from_slice(&bytes.0[32 - NodeId::LENGTH..32]);
        ComponentAddress::new_or_panic(address_bytes)
    }
}

/// A full Hyperlane message between chains
#[derive(Clone, Eq, PartialEq, Hash, ScryptoSbor, ScryptoEvent, Debug)]
pub struct HyperlaneMessage {
    /// 1   Hyperlane version number
    pub version: u8,
    /// 4   Message nonce
    pub nonce: u32,
    /// 4   Origin domain ID
    pub origin: u32,
    /// 32  Address in origin convention
    pub sender: Bytes32,
    /// 4   Destination domain ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: Bytes32,
    /// 0+  Message contents
    pub body: Vec<u8>,
}

impl HyperlaneMessage {
    pub fn new(
        nonce: u32,
        origin: u32,
        sender: Bytes32,
        destination: u32,
        recipient: Bytes32,
        body: Vec<u8>,
    ) -> Self {
        Self {
            version: MESSAGE_VERSION,
            nonce,
            origin,
            sender,
            destination,
            recipient,
            body,
        }
    }

    pub fn digest(
        &self,
        merkle_tree_hook: Bytes32,
        checkpoint_root: Bytes32,
        checkpoint_index: u32,
    ) -> Hash {
        let message_id = self.id();
        let mut digest = domain_hash(self.origin, merkle_tree_hook.as_ref()).to_vec();
        digest.extend(checkpoint_root.as_ref());
        digest.extend(checkpoint_index.to_be_bytes());
        digest.extend(message_id.as_ref());

        let digest = keccak256_hash(digest);
        eth_hash(digest.as_ref())
    }

    pub fn message_digest(
        origin: u32,
        message_id: Bytes32,
        merkle_tree_hook: Bytes32,
        checkpoint_root: Bytes32,
        checkpoint_index: u32,
    ) -> Hash {
        let mut digest = domain_hash(origin, merkle_tree_hook.as_ref()).to_vec();
        digest.extend(checkpoint_root.as_ref());
        digest.extend(checkpoint_index.to_be_bytes());
        digest.extend(message_id.as_ref());

        let digest = keccak256_hash(digest);
        eth_hash(digest.as_ref())
    }
}

impl From<RawHyperlaneMessage> for HyperlaneMessage {
    fn from(m: RawHyperlaneMessage) -> Self {
        HyperlaneMessage::from(&m)
    }
}

impl From<&RawHyperlaneMessage> for HyperlaneMessage {
    fn from(m: &RawHyperlaneMessage) -> Self {
        let version = m[0];
        let nonce: [u8; 4] = m[1..5].try_into().unwrap();
        let origin: [u8; 4] = m[5..9].try_into().unwrap();
        let sender: [u8; 32] = m[9..41].try_into().unwrap();
        let destination: [u8; 4] = m[41..45].try_into().unwrap();
        let recipient: [u8; 32] = m[45..77].try_into().unwrap();
        let body = m[77..].into();
        Self {
            version,
            nonce: u32::from_be_bytes(nonce),
            origin: u32::from_be_bytes(origin),
            sender: sender.into(),
            destination: u32::from_be_bytes(destination),
            recipient: recipient.into(),
            body,
        }
    }
}

impl From<&HyperlaneMessage> for RawHyperlaneMessage {
    fn from(m: &HyperlaneMessage) -> Self {
        let mut message_vec = vec![];
        message_vec.push(m.version);
        message_vec.extend_from_slice(&m.nonce.to_be_bytes());
        message_vec.extend_from_slice(&m.origin.to_be_bytes());
        message_vec.extend_from_slice(m.sender.0.as_ref());
        message_vec.extend_from_slice(&m.destination.to_be_bytes());
        message_vec.extend_from_slice(m.recipient.0.as_ref());
        message_vec.extend_from_slice(&m.body);
        message_vec
    }
}

impl From<HyperlaneMessage> for Vec<u8> {
    fn from(m: HyperlaneMessage) -> Self {
        (&m).into()
    }
}

impl HyperlaneMessage {
    /// Convert the message to a message id
    pub fn id(&self) -> Bytes32 {
        let message_bytes: RawHyperlaneMessage = self.into();
        let result = keccak256_hash(message_bytes);
        result.as_bytes().into()
    }
}
