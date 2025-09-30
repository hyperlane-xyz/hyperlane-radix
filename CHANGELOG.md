<!--

"Features" for new features.
"Improvements" for changes in existing functionality.
"Deprecated" for soon-to-be removed features.
"Bug Fixes" for any bug fixes.
"API Breaking" for breaking exported APIs used by developers building on SDK.

-->

# CHANGELOG

An '!' indicates an API breaking change.

## [v1.1.0](https://github.com/hyperlane-xyz/hyperlane-radix/releases/tag/v1.1.0) - 2025-09-02

### Features

! [#28](https://github.com/hyperlane-xyz/hyperlane-radix/pull/28) Updatable HypToken resource metadata.

## [v1.0.0](https://github.com/hyperlane-xyz/hyperlane-radix/releases/tag/v1.0.0) - 2025-09-01

**Initial Release of the Hyperlane Radix implementation** 🚀

This module integrates the **Hyperlane messaging protocol**
([Hyperlane Docs](https://docs.hyperlane.xyz/)), enabling seamless interchain
communication. It also provides full support for **token bridges**,
secured by **multi-signature Interchain Security Modules**.

### **Key Features**

- **Mailbox Functionality** – Send and receive messages securely across chains.
- **Warp Routes (Token Bridging)**
  - **Collateral Tokens** – Native asset bridging.
  - **Synthetic Tokens** – Wrapped asset representation.
- **Interchain Security Modules (ISMs)**
  - **Routing-ISM** – Enables domain specific ISM verification.
  - **Merkle-Root-Multisig-ISM** – Secure verification using Merkle roots.
  - **MessageId-Multisig-ISM** – Ensures integrity with message ID-based validation.
- **Post Dispatch Hooks**
  - **Merkle Tree Hook** – Supports Merkle-based verification for Multisig ISMs.
  - **InterchainGasPaymaster** – Provides gas prices for destination chains and interchain gas payments.
