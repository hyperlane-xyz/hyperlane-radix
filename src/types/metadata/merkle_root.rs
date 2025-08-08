use scrypto::crypto::Secp256k1Signature;
use scrypto::prelude::*;

use crate::types::Bytes32;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct MultisigIsmMerkleRootMetadata {
    pub origin_merkle_tree_hook: Bytes32,
    pub message_index: u32,
    pub message_id: Bytes32,
    pub merkle_proof: [Bytes32; 32],
    pub signed_checkpoint_index: u32,
    pub validator_signatures: Vec<Secp256k1Signature>,
}

const MERKLE_TREE_HOOK: usize = 0;
const MESSAGE_INDEX: usize = 32;
const MESSAGE_ID: usize = 36;
const MERKLE_PROOF: usize = 68;
const SIGNED_CHECKPOINT_INDEX: usize = 1092;
const SIGNATURES_OFFSET: usize = 1096;
const SIGNATURE_LENGTH: usize = 65;

/// Format of metadata:
/// [   0:  32] Origin merkle tree address
/// [  32:  36] Index of message ID in merkle tree
/// [  36:  68] Signed checkpoint message ID
/// [  68:1092] Merkle proof
/// [1092:1096] Signed checkpoint index (computed from proof and index)
/// [1096:????] Validator signatures (length := threshold * 65)
///
/// Note that the validator signatures being the length of the threshold is
/// not enforced here and should be enforced by the caller.
impl From<Vec<u8>> for MultisigIsmMerkleRootMetadata {
    fn from(bytes: Vec<u8>) -> Self {
        let bytes_len = bytes.len();
        // Require the bytes to be at least big enough to include a single signature.
        if bytes_len < SIGNATURES_OFFSET + SIGNATURE_LENGTH {
            panic!("MerkleRootMetadata: invalid metadata length");
        }

        let origin_merkle_tree_hook: Bytes32 = bytes[MERKLE_TREE_HOOK..MESSAGE_INDEX].into();
        let message_index_bytes: [u8; 4] = bytes[MESSAGE_INDEX..MESSAGE_ID]
            .try_into()
            .expect("MerkleRootMetadata: invalid metadata length");
        let message_index = u32::from_be_bytes(message_index_bytes);
        let message_id: Bytes32 = bytes[MESSAGE_ID..MERKLE_PROOF].into();

        let mut merkle_proof = [Bytes32::default(); 32];
        let merkle_proof_bytes = &bytes[MERKLE_PROOF..SIGNED_CHECKPOINT_INDEX];

        // Each Bytes32 is 32 bytes, so we need to process them in chunks
        for i in 0..32 {
            let start = i * 32;
            let end = start + 32;
            merkle_proof[i] = merkle_proof_bytes[start..end].into();
        }

        let signed_checkpoint_index_bytes: [u8; 4] = bytes
            [SIGNED_CHECKPOINT_INDEX..SIGNATURES_OFFSET]
            .try_into()
            .expect("MerkleRootMetadata: invalid metadata length");
        let signed_checkpoint_index = u32::from_be_bytes(signed_checkpoint_index_bytes);

        let signature_bytes_len = bytes_len - SIGNATURES_OFFSET;
        // Require the signature bytes to be a multiple of the signature length.
        // We don't need to check if signature_bytes_len is 0 because this is checked
        // above.
        if signature_bytes_len % SIGNATURE_LENGTH != 0 {
            panic!("MerkleRootMetadata: invalid metadata length");
        }
        let signature_count = signature_bytes_len / SIGNATURE_LENGTH;
        let mut validator_signatures = Vec::with_capacity(signature_count);
        for i in 0..signature_count {
            let signature_offset = SIGNATURES_OFFSET + (i * SIGNATURE_LENGTH);
            let signature = Secp256k1Signature::try_from(
                &bytes[signature_offset..signature_offset + SIGNATURE_LENGTH],
            )
            .expect("MerkleRootMetadata: was unable to parse signature");
            validator_signatures.push(signature);
        }

        Self {
            origin_merkle_tree_hook,
            message_index,
            signed_checkpoint_index,
            message_id,
            merkle_proof,
            validator_signatures,
        }
    }
}
