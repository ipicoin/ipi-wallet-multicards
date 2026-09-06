# Changelog

## Unreleased

- Split Card Vault and Payment Relay into two peer contract directories in a
  shared Cargo workspace.
- Restored the complete Card Vault and Payment Relay sources, locked dependency
  manifests, tests, CI, security notes and deployment records from Git history.
- Kept release Wasm files under `contracts/` and updated recovered tooling and
  documentation to use the current artifact layout.
- Added the immutable, zero-retention five-hop Payment Relay contract, tests,
  optimized checksummed Wasm artifact and deployment records.
- Documented receiver-balance privacy and full public-chain traceability through
  `ipi-deanonimizer`.
- Recorded the successful independently verified `ipi-testnet-1` deployment as
  code ID `2`, including its store transaction and no-admin smoke contract.
- Added one shared IPI address controlled by fully equal, unnumbered one-of-N
  cards.
- Added card invitations, acceptance, cancellation and active-card removal.
- Added vault-sponsored CosmWasm execution fees with a bounded invitation
  allowance and equal active-member allowances.
- Added the optimized, checksummed Wasm artifact and IPI Testnet deployment
  documentation.
- Documented the mandatory independent Mainnet deployment and verification
  process after the IPI network migration.
