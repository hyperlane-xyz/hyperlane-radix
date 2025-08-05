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
            .expect(&format!("Multisig: unable to get signature at {}", i));

        let signer = recover_eth_address(&digest, signature);

        while validator_index < validator_count && signer != validators[validator_index] {
            validator_index += 1;
        }

        if validator_index >= validator_count {
            panic!("Multisig: threshold not reached")
        }

        validator_index += 1;
    }

    true
}
