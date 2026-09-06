# IPI Card Vault

This contract is covered by the repository's proprietary license.

This standalone repository owns the CosmWasm contract, its tracked deployment
artifact and the IPI Testnet server handoff. The Electron application consuming
the resulting code ID lives in the separate sibling repository `ipi-wallet`.

CosmWasm contract implementing one shared IPI account controlled by an
unbounded, on-chain set of IPI Cards. Every active card has full one-of-N
authority and may invite another card. An invitation becomes active only after
the invited card submits `accept_invitation` from its own controller address.
Cards have no numbers, labels, owners or privileged roles.

## Access model

- The instantiating address is the first active card but receives no special
  authority.
- Every active card can transfer native `aipi`, invite another card, cancel an
  invitation, or remove an active card.
- Invitations belong to the shared vault rather than to the member that created
  them. Any active card may cancel one.
- The final active card cannot be removed.
- Membership uses keyed storage and paginated queries; transfers do not iterate
  through the member set.
- The vault grants every active controller the same fee allowance, restricted
  to CosmWasm execution messages. An invitation receives a bounded allowance
  for acceptance; removal or cancellation revokes it.

Instantiate without an admin so that no external key can replace the contract
code. The contract accepts `aipi` at instantiation, and its address can receive
later bank transfers normally. Execute calls reject attached funds. Inviting a
card transfers one `aipi` to its controller so a previously unused address has
an on-chain account before it accepts.

## Validation

```sh
cargo +1.85.0 test --locked
cargo +1.85.0 build --locked --release --target wasm32-unknown-unknown
```

For a reproducible CosmWasm 2.x deployment artifact, run from the repository
root:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source=ipi_card_vault_cache,target=/target \
  --mount type=volume,source=ipi_cosmwasm_registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1
```

Do not use optimizer 0.17.x: it emits contracts requiring CosmWasm 3.x. The
optimized Wasm and `artifacts/checksums.txt` are intentionally tracked by Git.
Verify the checksum, store the Wasm on IPI, and configure the resulting code ID
in IPI Wallet. The complete procedure is in
[`SERVER-HANDOFF.md`](SERVER-HANDOFF.md).
