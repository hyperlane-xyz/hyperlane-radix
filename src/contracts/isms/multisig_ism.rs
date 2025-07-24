use scrypto::prelude::*;

use crate::types::{recover_eth_address, EthAddress};

pub fn verify_multisig(
    digest: Hash,
    signatures: &[Secp256k1Signature],
    validators: &[EthAddress],
    threshold: usize,
) -> bool {
    let mut validator_index = 0usize;
    let validator_count = validators.len();

    for i in 0..threshold {
        let signature = signatures
            .get(i)
            .expect(format!("MessageIdMultisig: unable to get signature at {}", i).as_str());

        let signer = recover_eth_address(&digest.0, signature);

        while validator_index < validator_count && signer != validators[validator_index] {
            validator_index += 1;
        }

        if validator_index >= validator_count {
            panic!("MessageIdMultisig: threshold not reached")
        }

        validator_index += 1;
    }

    true
}
