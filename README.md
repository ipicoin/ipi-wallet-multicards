# IPI Wallet Multicards

`ipi-wallet-multicards` is the standalone CosmWasm and deployment repository
for shared IPI Card Vaults and their transparent Payment Relay layer. Multiple
physical IPI Cards attach to one vault address with exactly the same one-of-N
authority. There are no card numbers, primary cards, backup cards or privileged
roles.

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
- **Payment Relay status:** source and tests are ready in `payment-relay/`, but
  its Wasm has not yet been stored on `ipi-testnet-1`. Vault relay payments stay
  disabled until the server deploys the tracked artifact and the wallet is
  configured with the resulting `IPI_PAYMENT_RELAY_CODE_ID`.

A Testnet `code_id`, transaction hash or contract address is valid only on its
own chain. It must never be copied into the Mainnet wallet configuration.

## Repository boundary

| Location | Responsibility |
| --- | --- |
| `src/` | CosmWasm membership, transfer and feegrant logic |
| `payment-relay/` | Five-hop, zero-retention, vault-bound payment relay contract |
| `artifacts/ipi_card_vault.wasm` | Git-tracked optimized deployment artifact |
| `artifacts/checksums.txt` | Expected SHA-256 checksum |
| `deployments/` | Immutable per-network deployment evidence |
| `PAYMENT-RELAY-HANDOFF.md` | Exact relay build, store, verification and wallet handoff procedure |
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

## Transparent Payment Relay protocol

Each vault can create one wallet-managed set of five separate relay instances,
slots `0..4`. Every instance is immutable, stores no key and exposes its bound
vault through the public `Config {}` query. A relay accepts only one positive
`aipi` coin, verifies the active vault controller and forwards the entire
amount. It retains no payment balance.

For a vault payment, `ipi-wallet` signs one atomic transaction containing:

1. a vault execution transferring the exact payment amount to the active card
   controller;
2. a controller execution supplying the exact amount to the first relay;
3. recursive execution through all five relays in a new cryptographically
   random order;
4. a final native bank send to the recipient.

The first relay requires exactly four remaining unique slots, so a
controller-originated route cannot omit a relay. Any failure rolls back the
whole transaction. Custom `ipi_payment_relay` events disclose the payment ID,
vault, controller, exact route, recipient and amount.

This is not anonymity. It prevents a basic merchant interface from receiving
the high-value vault as the immediate bank sender, while leaving the full route
transparent to explorers, analysts and the read-only **Deanonimizer** tab built
into `ipi-pokedex`. Anyone capable of reading public chain data can trace it;
no privileged tracing backdoor exists.

## Build and verification

Required contract toolchain: Rust 1.85.0 with
`wasm32-unknown-unknown`.

```sh
cargo +1.85.0 fmt -- --check
cargo +1.85.0 clippy --locked --all-targets -- -D warnings
cargo +1.85.0 test --locked
cargo +1.85.0 build --locked --release --target wasm32-unknown-unknown

cargo +1.85.0 fmt --manifest-path payment-relay/Cargo.toml -- --check
cargo +1.85.0 clippy --manifest-path payment-relay/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.85.0 test --manifest-path payment-relay/Cargo.toml --locked
cargo +1.85.0 build --manifest-path payment-relay/Cargo.toml --locked --release --target wasm32-unknown-unknown
```

Build the reproducible CosmWasm 2.x artifact from the repository root:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source=ipi_card_vault_cache,target=/target \
  --mount type=volume,source=ipi_cosmwasm_registry_cache,target=/usr/local/cargo/registry \
  --entrypoint sh cosmwasm/optimizer:0.16.1 -c \
  'rustup toolchain install 1.85.0-x86_64-unknown-linux-musl --profile minimal --target wasm32-unknown-unknown && rustup default 1.85.0-x86_64-unknown-linux-musl && cd /code && /usr/local/bin/optimize.sh .'

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

The Payment Relay has its own lockfile and optimized artifact under
`payment-relay/`. Build it from that directory with the same optimizer image;
never deploy an untracked `target/` output.

The tracked Payment Relay artifact is 236954 bytes with SHA-256:

```text
3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553
```

## Testnet deployment

The first deployment completed successfully on 2026-09-06 as code ID `2`.
[`deployments/ipi-testnet-1.json`](deployments/ipi-testnet-1.json) records the
source commit, artifact checksum, transaction hashes, smoke contract and
verified runtime properties.

The completed vault deployment can be independently checked against
[`deployments/ipi-testnet-1.json`](deployments/ipi-testnet-1.json). Follow
[`PAYMENT-RELAY-HANDOFF.md`](PAYMENT-RELAY-HANDOFF.md) to deploy the additional
relay code from a secured server connected to a synchronized `ipi-testnet-1`
node. The handoff covers:

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
disabled for that network. Relay payments require their own deployment record
and `IPI_PAYMENT_RELAY_CODE_ID` in addition to the vault code ID.
