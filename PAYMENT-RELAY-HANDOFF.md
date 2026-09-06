# IPI Payment Relay — server handoff

Status: ready for the first `ipi-testnet-1` deployment. Updated 2026-09-06.

This file is the complete handoff for the node server after the repository is
pushed. The short operator sequence is:

1. pull `main` with fast-forward only and record the resulting commit;
2. verify the tracked relay artifact and run its tests;
3. store the relay Wasm once on `ipi-testnet-1`;
4. verify the committed transaction, on-chain checksum and `Everybody`
   instantiate permission;
5. perform a no-admin relay smoke test against a vault whose active member is
   the smoke controller;
6. complete `deployments/ipi-testnet-1-payment-relay.template.json` as a new
   immutable deployment record;
7. return the new relay code ID to both `ipi-wallet` and `ipi-pokedex`.

The server may begin with:

```sh
git pull --ff-only origin main
git status --short
git rev-parse HEAD
(cd payment-relay/artifacts && sha256sum --check checksums.txt)
```

`git status --short` must be empty before deployment. Do not deploy an artifact
from an uncommitted server-side rebuild.

## Outcome required from the server

Store the reviewed `payment-relay/artifacts/ipi_payment_relay.wasm` on the
target IPI chain, verify the committed checksum and instantiate permission, and
return the new chain-specific `code_id`. The server does not create end-user
relay instances: `ipi-wallet` creates five immutable instances for each vault
after an active physical card reviews and signs the one-time setup.

The existing vault code ID `2` on `ipi-testnet-1` is unchanged and requires no
migration. Do not redeploy or migrate existing vault instances.

The reviewed Payment Relay source/artifact commit is
`8a94bc495f2d536b29208f5ec7a3b03c5176e805`.

The repository can contain later documentation and deployment-record commits.
The tracked bytes are approved only when their SHA-256 and length still match
the values below.

## Preflight

Use a synchronized non-validator operational node and a separate funded
deployer account. Never use or export validator consensus keys.

Verify from the reviewed source commit:

```sh
cargo +1.85.0 fmt --manifest-path payment-relay/Cargo.toml -- --check
cargo +1.85.0 clippy --manifest-path payment-relay/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.85.0 test --manifest-path payment-relay/Cargo.toml --locked
(cd payment-relay/artifacts && sha256sum --check checksums.txt)
```

The reviewed artifact is `ipi_payment_relay.wasm`, 236954 bytes, SHA-256
`3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553`.

Suggested session variables (replace only values that depend on the actual
node installation):

```sh
export IPI_NODE_BIN=/absolute/path/to/the/node-binary
export IPI_DEPLOYER_KEY=ipi-vault-deployer
export IPI_KEYRING_BACKEND=os
export IPI_CHAIN_ID=ipi-testnet-1
export IPI_NODE_RPC=tcp://127.0.0.1:26657
export IPI_GAS_PRICE=1500000000aipi
export IPI_RELAY_ARTIFACT="$PWD/payment-relay/artifacts/ipi_payment_relay.wasm"
export IPI_RELAY_SHA256=3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553
```

Read the node service `ExecStart` and command help rather than assuming the
binary name or flags. Confirm the deployer address and balance without printing
any private key material:

```sh
"$IPI_NODE_BIN" version --long
"$IPI_NODE_BIN" status --node "$IPI_NODE_RPC"
"$IPI_NODE_BIN" query wasm params --node "$IPI_NODE_RPC" --output json

export IPI_DEPLOYER_ADDRESS="$(
  "$IPI_NODE_BIN" keys show "$IPI_DEPLOYER_KEY" \
    --keyring-backend "$IPI_KEYRING_BACKEND" \
    --address
)"

"$IPI_NODE_BIN" query bank balances "$IPI_DEPLOYER_ADDRESS" \
  --node "$IPI_NODE_RPC" \
  --output json
sha256sum "$IPI_RELAY_ARTIFACT"
"$IPI_NODE_BIN" tx wasm store --help
```

Stop if the node is catching up, reports another chain, the checksum differs,
the deployer lacks `aipi`, code upload is not allowed, or user instantiation
cannot be set to `Everybody`.

Confirm the node reports the intended chain ID and is caught up. Replace the
placeholder binary, key, gas price and flags below with the canonical values
for that node release; review its help output before signing:

```sh
"$IPI_NODE_BIN" tx wasm store "$IPI_RELAY_ARTIFACT" \
  --from "$IPI_DEPLOYER_KEY" \
  --keyring-backend "$IPI_KEYRING_BACKEND" \
  --chain-id "$IPI_CHAIN_ID" \
  --node "$IPI_NODE_RPC" \
  --gas auto \
  --gas-adjustment 1.4 \
  --gas-prices "$IPI_GAS_PRICE" \
  --broadcast-mode sync \
  --output json \
  --yes
```

If this node version requires an explicit instantiate-permission flag, append
only the exact `Everybody` flag confirmed by `tx wasm store --help`.

Record the store transaction hash and wait for committed success. From the
committed transaction, obtain `code_id`, then query code info and download the
on-chain bytes using the node release's canonical Wasm query commands.

Required checks:

- transaction `code` is `0` on `ipi-testnet-1`;
- SHA-256 of downloaded on-chain Wasm equals `checksums.txt`;
- instantiate permission is `Everybody`;
- source commit, local checksum, transaction hash and code ID are recorded.

Do not treat the synchronous broadcast response as execution success. Query the
transaction after inclusion in a block:

```sh
export IPI_RELAY_STORE_TX=PASTE_COMMITTED_STORE_TX_HASH
"$IPI_NODE_BIN" query tx "$IPI_RELAY_STORE_TX" \
  --node "$IPI_NODE_RPC" \
  --output json

export IPI_PAYMENT_RELAY_CODE_ID=PASTE_CODE_ID_FROM_STORE_CODE_EVENT
"$IPI_NODE_BIN" query wasm code-info "$IPI_PAYMENT_RELAY_CODE_ID" \
  --node "$IPI_NODE_RPC" \
  --output json
```

The transaction must have `code: 0`. The lowercase `data_hash` from code info
must equal `$IPI_RELAY_SHA256` and the permission must be `Everybody`.

## Smoke verification

The relay can only be instantiated by an active member of its configured vault.
Use a disposable no-admin Testnet Card Vault controlled by the smoke account,
or make the smoke account a member of a designated test vault. Instantiate one
relay with no admin:

The existing empty Testnet smoke vault may be reused:

```text
vault:      ipi1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq5df8fy
controller: ipi1l2vqczqpduvtn5jteplzq9vxj2dc793pv6rcrh
vault code: 2
```

Reuse it only after a smart `member` query proves that
`$IPI_DEPLOYER_ADDRESS` is currently its active member. If the deployer is a
different address, create a disposable no-admin vault with code ID `2` or add
that deployer through the normal invitation flow. A non-member cannot
instantiate an approved relay.

After selecting the smoke vault:

```sh
export IPI_SMOKE_VAULT=ipi1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq5df8fy

"$IPI_NODE_BIN" query wasm contract-state smart "$IPI_SMOKE_VAULT" \
  "{\"member\":{\"address\":\"$IPI_DEPLOYER_ADDRESS\"}}" \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" tx wasm instantiate "$IPI_PAYMENT_RELAY_CODE_ID" \
  "{\"vault\":\"$IPI_SMOKE_VAULT\",\"slot\":0}" \
  --label "ipi-payment-relay-smoke-$IPI_PAYMENT_RELAY_CODE_ID-slot-0" \
  --no-admin \
  --from "$IPI_DEPLOYER_KEY" \
  --keyring-backend "$IPI_KEYRING_BACKEND" \
  --chain-id "$IPI_CHAIN_ID" \
  --node "$IPI_NODE_RPC" \
  --gas auto \
  --gas-adjustment 1.4 \
  --gas-prices "$IPI_GAS_PRICE" \
  --broadcast-mode sync \
  --output json \
  --yes
```

Confirm `--no-admin` with `tx wasm instantiate --help` before signing. Query the
committed transaction, export the `_contract_address`, and verify it:

```sh
export IPI_SMOKE_RELAY=PASTE_RELAY_CONTRACT_ADDRESS

"$IPI_NODE_BIN" query wasm contract "$IPI_SMOKE_RELAY" \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" query wasm contract-state smart "$IPI_SMOKE_RELAY" \
  '{"config":{}}' \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" query bank balances "$IPI_SMOKE_RELAY" \
  --node "$IPI_NODE_RPC" \
  --output json
```

Query `{"config":{}}` and require exactly:

```json
{
  "vault": "ipi1_SMOKE_VAULT",
  "slot": 0,
  "denom": "aipi",
  "relay_count": 5
}
```

Also confirm the instance has no administrator. A non-member instantiation and
an out-of-range slot must fail. A full payment smoke test should create slots
`0..4`, forward a small exact amount through all five, verify the recipient
delta, verify every relay balance remains zero, and preserve all five
`ipi_payment_relay` events.

Do not send funds to the relay except through its reviewed `forward` execute.
Relays intentionally have no recovery or sweep entry point; an accidental
native bank transfer would remain there.

## Return to the wallet operator

Return a signed or otherwise authenticated deployment record containing:

- network and chain ID;
- source commit;
- artifact filename, byte length and SHA-256;
- store transaction hash and committed height;
- Payment Relay `code_id`;
- `Everybody` instantiate permission evidence;
- smoke transaction and five smoke relay addresses;
- no-admin, config, exact-recipient-delta and zero-retention evidence.

Start from
`deployments/ipi-testnet-1-payment-relay.template.json`, replace every `null`
and the empty relay list with committed evidence, then rename it as the final
deployment record. Do not change the reviewed checksum or artifact length.

The wallet operator then sets:

```sh
export IPI_CARD_VAULT_CODE_ID=2
export IPI_PAYMENT_RELAY_CODE_ID=<verified-testnet-relay-code-id>
```

Restart/rebuild the desktop wallet and create the five-relay set from
`Send from vault`. Until `IPI_PAYMENT_RELAY_CODE_ID` is present, the wallet must
keep relay payments disabled.

The Pokédex operator sets the same verified relay code ID so the built-in
Deanonimizer accepts only the reviewed contract:

```sh
export IPI_REST_ENDPOINT=https://rest-testnet.ipi.io
export IPI_PAYMENT_RELAY_CODE_ID=<verified-testnet-relay-code-id>
```

Restart both applications after changing their controlled environments. Verify
one small wallet payment end-to-end, copy the final relay address and
transaction hash into Pokédex, and require Pokédex to return the same vault,
controller, recipient, amount and ordered five-relay route shown by the public
transaction. This check is required before the feature is treated as deployed.

`IPI_CARD_VAULT_CODE_ID=2` remains unchanged. Never substitute the vault code
ID for the relay code ID.

## Deployment record and repository return

Copy the template instead of overwriting it:

```sh
cp deployments/ipi-testnet-1-payment-relay.template.json \
  deployments/ipi-testnet-1-payment-relay.json
```

Fill every `null`, the five relay addresses and all verification booleans from
committed public evidence. Run a JSON parser, inspect the diff, commit the final
record and push it through the normal reviewed repository workflow. Do not add
mnemonics, keyring data, passwords, private keys or internal server paths.

The final operator response must include at minimum:

```text
network=ipi-testnet-1
source_commit=8a94bc495f2d536b29208f5ec7a3b03c5176e805
artifact=payment-relay/artifacts/ipi_payment_relay.wasm
artifact_bytes=236954
on_chain_sha256=3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553
store_tx=...
store_height=...
payment_relay_code_id=...
instantiate_permission=Everybody
smoke_tx=...
smoke_vault=...
smoke_relays=...,...,...,...,...
all_instances_have_no_admin=true
recipient_delta_matches=true
all_relay_balances_zero=true
five_public_route_events_verified=true
wallet_payment_verified=true
pokedex_trace_verified=true
```

## Mainnet migration

Testnet code IDs and addresses have no Mainnet meaning. After IPI migrates to
Mainnet, repeat the audit, reproducible optimizer build, store, checksum,
permission and full smoke procedure on the canonical Mainnet chain. Publish a
new immutable deployment record and configure the Mainnet wallet only with the
new Mainnet code IDs and endpoints.
