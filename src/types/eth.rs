use scrypto::prelude::*;

use crate::types::Bytes32;

#[derive(Debug, Clone, PartialEq, Eq, Copy, Sbor, Hash, PartialOrd, Ord)]
#[sbor(transparent)]
pub struct EthAddress([u8; 20]);

impl From<Hash> for EthAddress {
    fn from(value: Hash) -> Self {
        EthAddress(value.lower_bytes())
    }
}

impl From<[u8; 20]> for EthAddress {
    fn from(value: [u8; 20]) -> Self {
        EthAddress(value)
    }
}

impl AsRef<[u8]> for EthAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub fn hash_concat(left: impl AsRef<[u8]>, right: impl AsRef<[u8]>) -> Hash {
    let mut bytes = left.as_ref().to_vec();
    bytes.extend(right.as_ref());

    keccak256_hash(bytes)
}

pub fn eth_hash(msg: &[u8]) -> Hash {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", msg.len());
    let mut bytes: Vec<_> = prefix.into_bytes();
    bytes.extend(msg);
    keccak256_hash(bytes)
}

pub fn domain_hash(local_domain: u32, address: &[u8]) -> Hash {
    let mut bytes = local_domain.to_be_bytes().to_vec();
    bytes.extend(address);
    bytes.extend("HYPERLANE".as_bytes());
    keccak256_hash(bytes)
}

pub fn announcement_domain_hash(local_domain: u32, address: &[u8]) -> Hash {
    let mut bytes = local_domain.to_be_bytes().to_vec();
    bytes.extend(address);
    bytes.extend("HYPERLANE_ANNOUNCEMENT".as_bytes());
    keccak256_hash(bytes)
}

pub fn announcement_digest(
    storage_location: &str,
    local_domain: u32,
    mailbox_address: Bytes32,
) -> Hash {
    let mut domain_hash = announcement_domain_hash(local_domain, mailbox_address.as_ref()).to_vec();
    domain_hash.extend(storage_location.as_bytes());

    eth_hash(keccak256_hash(domain_hash).as_ref())
}

/// recover the eth address from the signature of the given hash
pub fn recover_eth_address(digest: &Hash, signature: &Secp256k1Signature) -> EthAddress {
    // For the CryptoUtils the recovery Id must be moved to the beginning
    // And it must be converted from an eth id (27/28) to a normal id (0/1)
    let mut signature: Vec<u8> = signature.to_vec();
    let last = signature.pop().expect("sig: invalid Secp256k1 signature");
    // Sub 27 of the recovery id according to this - https://eips.ethereum.org/EIPS/eip-155
    signature.insert(0, last - 27);

    let signature = Secp256k1Signature(
        signature
            .try_into()
            .expect("sig: can't parse Secp256k1 signature"),
    );

    let pubkey =
        CryptoUtils::secp256k1_ecdsa_verify_and_key_recover_uncompressed(digest, signature);

    // ethereum address is the hash of the uncompressed public key
    // exclude the first byte - which is always 0x4 to indicate Secp256k1
    let pubkey_bytes = pubkey.0;
    let address = keccak256_hash(&pubkey_bytes[1..]);
    address.into()
}
