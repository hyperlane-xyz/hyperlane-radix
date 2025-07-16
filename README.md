# Hyperlane-Radix

> [!WARNING]  
> This project is currently under development and not intended to be used in production.

## Resources for Radix

- [Examples](https://github.com/radixdlt/scrypto-examples)
- [Challenges](https://github.com/radixdlt/scrypto-challenges)
- [APIs](https://docs.radixdlt.com/docs/network-apis)
- [LayerZero implementation](https://github.com/radixdlt/layerzero/blob/main/tools/lz-cli/src/lz_core_api_client.rs)
- [Step by step guide](https://docs.radixdlt.com/docs/learning-step-by-step)

## Development

Getting started:

```
https://docs.radixdlt.com/docs/setting-up-for-scrypto-development
```
C-Make version

```
(base) ➜  hyperlane-radix (main) cmake --version                                  ✱
cmake version 3.31.7

CMake suite maintained and supported by Kitware (kitware.com/cmake).
```

Setting up local

```
source setup.sh
```

Running local

```
resim run manifest/warp/collateral/transfer_remote.rtm
```

Run tests

```
scrypto test -- --nocapture
```

## TODOS

- `k256` should be checked or changed to an audited version
- Trait problem, a component like hooks should always have a given interface
- ISM:
  - Routing ISM
- IGP:
  - IGP set destination gas config methods
  - Public / Protected Methods enforcen
  - move from Decimals to I192
- MerkleTreeHook:
  - Double check the merkle tree implementation
- Mailbox:
  - similar issue to the MerkleTreeHook have to figure out caller
    - proposed solution: Mailbox issues badges to apps and checks validity

- Check on Finality (How do the Shards work) -> Consensus engine
- have a dedicated metadata function for involved addresses which the relayer calls first
- WARNING: Double check that proofs can not be forwarded; Proofs should be specific to that recipient, should not work for other recipients.
- Proxy Contract upgradability
