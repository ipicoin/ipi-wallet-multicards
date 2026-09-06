# IPI Payment Relay — server handoff

## Outcome required from the server

Store the reviewed `payment-relay/artifacts/ipi_payment_relay.wasm` on the
target IPI chain, verify the committed checksum and instantiate permission, and
return the new chain-specific `code_id`. The server does not create end-user
relay instances: `ipi-wallet` creates five immutable instances for each vault
after an active physical card reviews and signs the one-time setup.

The existing vault code ID `2` on `ipi-testnet-1` is unchanged and requires no
migration. Do not redeploy or migrate existing vault instances.

## Preflight

Use a synchronized non-validator operational node and a separate funded
deployer account. Never use or export validator consensus keys.

Verify from the reviewed source commit:

```sh
cargo +1.85.0 fmt --manifest-path payment-relay/Cargo.toml -- --check
cargo +1.85.0 clippy --manifest-path payment-relay/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.85.0 test --manifest-path payment-relay/Cargo.toml --locked
cd payment-relay/artifacts
sha256sum --check checksums.txt
```

The reviewed artifact is `ipi_payment_relay.wasm`, 236954 bytes, SHA-256
`3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553`.

Confirm the node reports the intended chain ID and is caught up. Replace the
placeholder binary, key, gas price and flags below with the canonical values
for that node release; review its help output before signing:

```sh
ipid status
ipid tx wasm store payment-relay/artifacts/ipi_payment_relay.wasm \
  --from <deployer-key> \
  --chain-id ipi-testnet-1 \
  --gas auto \
  --gas-adjustment 1.4 \
  --gas-prices <approved-aipi-gas-price> \
  --broadcast-mode sync \
  --yes
```

Record the store transaction hash and wait for committed success. From the
committed transaction, obtain `code_id`, then query code info and download the
on-chain bytes using the node release's canonical Wasm query commands.

Required checks:

- transaction `code` is `0` on `ipi-testnet-1`;
- SHA-256 of downloaded on-chain Wasm equals `checksums.txt`;
- instantiate permission is `Everybody`;
- source commit, local checksum, transaction hash and code ID are recorded.

## Smoke verification

The relay can only be instantiated by an active member of its configured vault.
Use a disposable no-admin Testnet Card Vault controlled by the smoke account,
or make the smoke account a member of a designated test vault. Instantiate one
relay with no admin:

```json
{"vault":"ipi1_SMOKE_VAULT","slot":0}
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

## Mainnet migration

Testnet code IDs and addresses have no Mainnet meaning. After IPI migrates to
Mainnet, repeat the audit, reproducible optimizer build, store, checksum,
permission and full smoke procedure on the canonical Mainnet chain. Publish a
new immutable deployment record and configure the Mainnet wallet only with the
new Mainnet code IDs and endpoints.
