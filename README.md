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

## Future Enhancements

- Use scrypto-interfaces

## Contributing

Thank you for considering contributing to this project.

**Overview**

- The latest state of development is on `main`.
- `main` must always pass `scrypto test` and `cargo fmt`.
- Everything must be covered by tests.

**Creating a Pull Request**

- Check out the latest state from main and always keep the PR in sync with main.
- Use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#specification).
- Only one feature per pull request.
- Write an entry for the Changelog.
- Write tests covering 100% of your modified code.
- The Github actions must pass.

**Legal**

You agree that your contribution is licenced under the given [LICENSE](LICENSE) 
and all ownership is handed over to authors listed in the section below.

## License

This project is licensed under the Apache License, Version 2.0.  
See the [LICENSE](LICENSE) file for the full terms.

Copyright 2025 Abacus Works
