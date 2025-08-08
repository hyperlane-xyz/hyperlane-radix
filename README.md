# Hyperlane-Radix

> [!WARNING]  
> This project is currently under development and not intended to be used in production.

This project is an implementation of Hyperlane for the Radix DLT, designed for 
a seamless interchain communication following the Hyperlane spec. 

## [src/contracts](./src/contracts)
`contracts` is intended to implement the fundamental functionalities of the 
Hyperlane protocol to dispatch and process messages, which can then be used by
applications like `warp`. It includes mailboxes, hooks, Interchain Security 
Modules (ISMs) as well as the Warp application for token transfers.

## [src/types](./src/types)
`types` contains structs which are reused across multiple components, containing
basic Hyperlane types like messages, metadata and payloads, as well as others.

## Development

Getting started:

```
https://docs.radixdlt.com/docs/setting-up-for-scrypto-development
```
C-Make version (You need to use Cmake 3.31)

```
(base) ➜  hyperlane-radix (main) cmake --version
cmake version 3.31.7

CMake suite maintained and supported by Kitware (kitware.com/cmake).
```

This command creates and initiates all required Hyperlane components for full 
token bridging.

```
source setup.sh
```

Run specific actions like sending a remote transfer.

```
resim run manifest/warp/collateral/transfer_remote.rtm
```

Run test suite.

```
scrypto test -- --nocapture
```

## Resources for Radix

- [Examples](https://github.com/radixdlt/scrypto-examples)
- [Challenges](https://github.com/radixdlt/scrypto-challenges)
- [APIs](https://docs.radixdlt.com/docs/network-apis)
- [LayerZero implementation](https://github.com/radixdlt/layerzero/blob/main/tools/lz-cli/src/lz_core_api_client.rs)
- [Step by step guide](https://docs.radixdlt.com/docs/learning-step-by-step)


## Future Enhancements

- Use scrypto-interfaces
