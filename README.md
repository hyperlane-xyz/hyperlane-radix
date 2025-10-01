# Hyperlane-Radix

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

## Deployment

This code is deployed on the Radix Mainnet. Binary generation is handled 
deterministically using [Scrypto Builder](https://docs.radixdlt.com/docs/scrypto-builder) 
within a GitHub Action workflow.

| Version  | Components         | Package                                                                                                                                                                                 |
|----------|--------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `v1.0.0` | Mailbox, ISMs, IGP | [package_rdx1pk3ldj3ktxuw6sv5txspjt2a8s42c7xxcn6wnf5yuytdrcqhpflfkc](https://dashboard.radixdlt.com/package/package_rdx1pk3ldj3ktxuw6sv5txspjt2a8s42c7xxcn6wnf5yuytdrcqhpflfkc/summary) |
| `v1.1.0` | WarpTokens         | [package_rdx1pkzmcj4mtal34ddx9jrt8um6u3yqheqpfvcj4s0ulmgyt094fw0jzh](https://dashboard.radixdlt.com/package/package_rdx1pkzmcj4mtal34ddx9jrt8um6u3yqheqpfvcj4s0ulmgyt094fw0jzh/summary) |

Component configurations and deployed routes are maintained in the [Hyperlane Registry](https://github.com/hyperlane-xyz/hyperlane-registry/tree/main/chains/radix).

## Audit

This code was audited by Zellic prior to deployment. 
From August 15 to August 22, 2025, Zellic conducted a security assessment of 
Hyperlane, reviewing Hyperlane Radix's code for vulnerabilities, design flaws, 
and general weaknesses in security posture.

[Hyperlane - Radix - Zellic Audit Report](https://github.com/Zellic/publications/blob/6b0207586e075d5d1937d5cad23ac5ef79f75805/Hyperlane%20-%20Radix%20-%20Zellic%20Audit%20Report.pdf)

## License

This project is licensed under the Apache License, Version 2.0.  
See the [LICENSE](LICENSE) file for the full terms.

Copyright 2025 Abacus Works, Inc.
