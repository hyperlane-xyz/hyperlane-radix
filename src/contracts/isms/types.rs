use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub enum Types {
    UNUSED,
    ROUTING,
    AGGREGATION,
    LEGACYMULTISIG,
    MERKLEROOTMULTISIG,
    MESSAGEIDMULTISIG,
    NULL, // used with relayer carrying no metadata
    CCIPREAD,
    ARBL2TOL1,
    WEIGHTEDMERKLEROOTMULTISIG,
    WEIGHTEDMESSAGEIDMULTISIG,
    OPL2TOL1,
}
