use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub enum Types {
    /// INVALID ISM
    Unused,
    /// Routing ISM (defers to another ISM)
    Routing,
    /// Aggregation ISM (aggregates multiple ISMs)
    Aggregation,
    /// Legacy ISM (DEPRECATED)
    LegacyMultisig,
    /// Merkle Proof ISM (batching and censorship resistance)
    MerkleRootMultisig,
    /// Message ID ISM (cheapest multisig with no batching)
    MessageIdMultisig,
    /// No metadata ISM (no metadata)
    Null,
    /// Ccip Read ISM (accepts offchain signature information)
    CcipRead,
}
