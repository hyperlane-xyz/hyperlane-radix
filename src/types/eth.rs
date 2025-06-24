//! Black-box (mostly) digital signature creation and verifification
//!
//! WARNING: This is not audited and just about the minimum viable
//! implemenation.
//!
//! Implementation uses the "RustCrypto" crates.  This works well enough but is
//! far from what should probably be used in production.
//!
//! Those creates we depend on here (k256 and friends) are not audited, and this
//! code is also not audited.  Additionally, the documentation for these crates
//! leaves a lot to be desired.  This works but the choices may not be optimal
//! for future on-ledger use
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
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
    let hash = keccak256_hash(bytes);
    hash
}

pub fn domain_hash(local_domain: u32, address: &[u8]) -> Hash {
    let mut bytes = local_domain.to_be_bytes().to_vec();
    bytes.extend(address);
    bytes.extend("HYPERLANE".as_bytes());
    return keccak256_hash(bytes);
}

pub fn announcement_digest(
    storage_location: &str,
    local_domain: u32,
    mailbox_address: Bytes32,
) -> Hash {
    let mut domain_hash = domain_hash(local_domain, mailbox_address.as_ref()).to_vec();
    domain_hash.extend(storage_location.as_bytes());

    keccak256_hash(domain_hash)
}

/// recover the eth address from the signature of the given hash
pub fn recover_eth_address(digest: &[u8], signature: &Secp256k1Signature) -> EthAddress {
    let mut signature = signature.0.clone();

    // Sub 27 of the recovery id according to this - https://eips.ethereum.org/EIPS/eip-155
    signature[64] -= 27;

    let recovery_id =
        RecoveryId::from_byte(signature[64]).expect("recover: invalid recovery on signature");

    // k256 expects the signature to not include the recovery id
    let signature = &signature[..64];

    let signature = Signature::from_slice(signature).expect("recover: invalid signature");
    let pubkey = VerifyingKey::recover_from_prehash(digest, &signature, recovery_id)
        .expect("verify: failed to recover public key");

    // ethereum address is the hash of the uncompressed public key
    // exculde the first byte - which is always 0x4 to indicate Secp256k1
    let pubkey_bytes = pubkey.to_encoded_point(false).to_bytes();
    let address = keccak256_hash(&pubkey_bytes[1..]);
    address.into()
}
