use scrypto::crypto::Secp256k1Signature;
use scrypto::prelude::*;

use crate::types::Bytes32;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct MultisigIsmMessageIdMetadata {
    pub origin_merkle_tree_hook: Bytes32,
    pub merkle_root: Bytes32,
    pub merkle_index: u32,
    pub validator_signatures: Vec<Secp256k1Signature>,
}

const MERKLE_TREE_HOOK: usize = 0;
const MERKLE_ROOT_OFFSET: usize = 32;
const MERKLE_INDEX_OFFSET: usize = 64;
const SIGNATURES_OFFSET: usize = 68;
const SIGNATURE_LENGTH: usize = 65;

/// Format of metadata:
/// [   0:  32] Origin merkle tree address
/// [  32:  64] Signed checkpoint root
/// [  64:  68] Signed checkpoint index
/// [  68:????] Validator signatures (length := threshold * 65)
///
/// Note that the validator signatures being the length of the threshold is
/// not enforced here and should be enforced by the caller.
impl From<Vec<u8>> for MultisigIsmMessageIdMetadata {
    fn from(bytes: Vec<u8>) -> Self {
        let bytes_len = bytes.len();
        // Require the bytes to be at least big enough to include a single signature.
        if bytes_len < SIGNATURES_OFFSET + SIGNATURE_LENGTH {
            panic!("MessageIdMetadata: invalid metadata length");
        }

        let origin_merkle_tree_hook: Bytes32 = bytes[MERKLE_TREE_HOOK..MERKLE_ROOT_OFFSET].into();
        let merkle_root: Bytes32 = bytes[MERKLE_ROOT_OFFSET..MERKLE_INDEX_OFFSET].into();
        // This cannot panic since SIGNATURES_OFFSET - MERKLE_INDEX_OFFSET is 4.
        let merkle_index_bytes: [u8; 4] = bytes[MERKLE_INDEX_OFFSET..SIGNATURES_OFFSET]
            .try_into()
            .expect("MessageIdMetadata: invalid metadata length");
        let merkle_index = u32::from_be_bytes(merkle_index_bytes);

        let signature_bytes_len = bytes_len - SIGNATURES_OFFSET;
        // Require the signature bytes to be a multiple of the signature length.
        // We don't need to check if signature_bytes_len is 0 because this is checked
        // above.
        if signature_bytes_len % SIGNATURE_LENGTH != 0 {
            panic!("MessageIdMetadata: invalid metadata length");
        }
        let signature_count = signature_bytes_len / SIGNATURE_LENGTH;
        let mut validator_signatures = Vec::with_capacity(signature_count);
        for i in 0..signature_count {
            let signature_offset = SIGNATURES_OFFSET + (i * SIGNATURE_LENGTH);
            let signature = Secp256k1Signature::try_from(
                &bytes[signature_offset..signature_offset + SIGNATURE_LENGTH],
            )
            .expect("MessageIdMetadata: was unable to parse signature");
            validator_signatures.push(signature);
        }

        Self {
            origin_merkle_tree_hook,
            merkle_root,
            merkle_index,
            validator_signatures,
        }
    }
}

impl Into<Vec<u8>> for MultisigIsmMessageIdMetadata {
    fn into(self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.origin_merkle_tree_hook.as_ref());
        bytes.extend_from_slice(&self.merkle_root.as_ref());
        bytes.extend_from_slice(&self.merkle_index.to_be_bytes());

        self.validator_signatures.iter().for_each(|signature| {
            bytes.extend_from_slice(signature.as_ref());
        });

        bytes
    }
}
