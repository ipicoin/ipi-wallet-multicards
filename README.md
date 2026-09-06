# IPI Wallet Multicards

`ipi-wallet-multicards` is the standalone CosmWasm contract and deployment
repository for attaching multiple physical IPI Cards to one shared IPI wallet
address. Every accepted card has exactly the same one-of-N authority. There are
no card numbers, primary cards, backup cards or privileged roles.

This repository is not the desktop wallet and it is not a long-running server
service. It produces a Wasm contract that an operator submits from a secured IPI
node server to the target chain. After `store`, the contract runs on-chain. The
separate `ipi-wallet` application consumes the verified `code_id` returned by
that deployment.

## Status and deployment targets

This is unaudited laboratory software. It must not protect material value.

- **Current target:** deployed on `ipi-testnet-1` as code ID `2` for integration
  and physical-card tests. The independently verified chain record is in
  [`deployments/ipi-testnet-1.json`](deployments/ipi-testnet-1.json).
- **Current artifact:** `artifacts/ipi_card_vault.wasm`, exactly matching the
  on-chain checksum for code ID `2`.
- **Mainnet status:** not deployed and not approved. After IPI migrates to
  Mainnet, an operator must validate the final Mainnet runtime, rebuild from an
  audited release, deploy the Wasm through a synchronized Mainnet node server,
  run the on-chain smoke tests and configure the wallet with the new Mainnet
  `code_id`.

A Testnet `code_id`, transaction hash or contract address is valid only on its
own chain. It must never be copied into the Mainnet wallet configuration.

## Repository boundary

| Location | Responsibility |
| --- | --- |
| `src/` | CosmWasm membership, transfer and feegrant logic |
| `artifacts/ipi_card_vault.wasm` | Git-tracked optimized deployment artifact |
| `artifacts/checksums.txt` | Expected SHA-256 checksum |
| `deployments/` | Immutable per-network deployment evidence |
| `SERVER-HANDOFF.md` | Exact Testnet node-operator deployment and verification procedure |
| `ipi-wallet` repository | Electron UI, physical-card signing and configured `code_id` consumption |

Card firmware, card issuance and the IPI node implementation are outside this
repository.

## Authority model

The contract address is the shared receive address. Each physical card retains
its own private key and controller address.

- The instantiating controller becomes the first active card but gains no
  special authority.
- Every active card can transfer native `aipi`, invite another controller,
  cancel any pending invitation and remove any active card.
- Invitations belong to the vault, not to the card that created them.
- An invited controller becomes active only after signing
  `accept_invitation` with its own key.
- The last active card cannot be removed.
- Transfers use keyed membership checks and do not iterate through all cards.
- Every active card receives the same vault-funded fee allowance, restricted to
  `/cosmwasm.wasm.v1.MsgExecuteContract`.
- A pending invitation receives only a bounded acceptance allowance. Removing
  a card or cancelling its invitation revokes the corresponding allowance.

The contract sends `1 aipi` when creating an invitation so that a previously
unused controller address exists on-chain before acceptance. Contract execute
calls reject attached funds. Vault instances must be created without an admin,
preventing an external migration key from replacing their code.

This is deliberately full one-of-N authority: compromising any one active card
and its credential can compromise the entire shared balance. Adding a card adds
another complete authorization path.

## Build and verification

Required contract toolchain: Rust 1.85.0 with
`wasm32-unknown-unknown`.

```sh
cargo +1.85.0 fmt -- --check
cargo +1.85.0 clippy --locked --all-targets -- -D warnings
cargo +1.85.0 test --locked
cargo +1.85.0 build --locked --release --target wasm32-unknown-unknown
```

Build the reproducible CosmWasm 2.x artifact from the repository root:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source=ipi_card_vault_cache,target=/target \
  --mount type=volume,source=ipi_cosmwasm_registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1

cd artifacts
sha256sum --check checksums.txt
```

Optimizer `0.17.x` must not be substituted: it emits contracts requiring
CosmWasm 3.x. The optimized Wasm is intentionally tracked in Git so the server
deploys the reviewed bytes rather than an undocumented local build.

The currently tracked artifact is 228947 bytes with SHA-256:

```text
fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454
```

## Testnet deployment

The first deployment completed successfully on 2026-09-06 as code ID `2`.
[`deployments/ipi-testnet-1.json`](deployments/ipi-testnet-1.json) records the
source commit, artifact checksum, transaction hashes, smoke contract and
verified runtime properties.

Follow [`SERVER-HANDOFF.md`](SERVER-HANDOFF.md) when independently checking or
repeating the procedure from a secured server connected to a synchronized
`ipi-testnet-1` node. The handoff covers:

1. node, chain, permission, deployer-account and checksum preflight;
2. `MsgStoreCode` submission and committed transaction verification;
3. on-chain checksum and instantiate-permission verification;
4. immutable no-admin smoke instantiation;
5. contract-state and feegrant checks;
6. return of the verified Testnet `code_id` to `ipi-wallet`.

The deployer must be a separate funded operational account. Never use or export
the validator consensus key.

## Required Mainnet deployment after migration

Migration of the IPI network does not automatically publish this contract on
Mainnet. Before enabling multicard wallets on Mainnet, the server operator must
perform a new deployment from a reviewed release of this repository.

Mainnet readiness requires all of the following:

1. Freeze and audit the exact source commit and tracked Wasm release.
2. Confirm the canonical Mainnet chain ID, native denomination and decimals,
   gas price, node binary and RPC endpoints. Do not copy Testnet values. The
   current contract hardcodes `aipi`; a different Mainnet denomination requires
   a source change, new tests, a new artifact and renewed review.
3. Confirm that the Mainnet runtime supports `cosmwasm_2_0`, protobuf
   `CosmosMsg::Any`, `MsgGrantAllowance`, `MsgRevokeAllowance` and unrestricted
   user instantiation for the stored code.
4. Re-run formatting, clippy, all tests, release build and optimizer from the
   audited commit; record the release checksum.
5. Place the reviewed repository on the secured Mainnet node server and submit
   the Wasm to the Mainnet chain using a separate funded deployer account.
6. Verify the committed `store` transaction, on-chain checksum, code permission
   and new Mainnet `code_id`.
7. Instantiate an empty no-admin smoke vault on Mainnet and verify its config,
   first member and restricted feegrant.
8. Publish a Mainnet deployment record containing the source commit, artifact
   checksum, store transaction hash, code ID and smoke-test contract address.
9. Configure the Mainnet build of `ipi-wallet` with that new code ID and the
   canonical Mainnet endpoints, then repeat the two-card integration test.

Mainnet enablement must stop if any runtime capability, denomination,
permission, checksum or smoke-test result differs from the reviewed
expectation. `SERVER-HANDOFF.md` is currently Testnet-specific; its chain,
endpoint, gas and artifact values must be replaced by a separately reviewed
Mainnet handoff rather than edited ad hoc during deployment.

## Definition of a successful release

A release is usable by `ipi-wallet` only when the target chain has all of these
recorded and independently verifiable:

- source commit and release tag;
- exact Wasm SHA-256;
- successful `store` transaction hash;
- on-chain `code_id` with `Everybody` instantiate permission;
- successful no-admin smoke contract;
- expected `aipi` config, initial member and restricted feegrant;
- wallet build configured for the same chain and code ID.

Until that record exists for a network, the multicard feature must remain
disabled for that network.
