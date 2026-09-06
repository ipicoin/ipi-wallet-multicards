# Security status

This repository is an unaudited laboratory contract for `ipi-testnet-1`. Do not
use it to protect material value.

## Payment Relay limitations

The relay path provides receiver-balance privacy only. All five hops, the vault,
controller, amount and recipient are public-chain data. Do not describe it as a
mixer, anonymity protocol or confidential transaction system.

Relay instances must have no admin, use the exact approved code ID, bind to the
expected immutable Card Vault and cover slots `0..4` exactly once. The wallet
must review and verify the full route. A relay configuration alone establishes
the binding to a vault; transaction events establish that a particular payment
used the complete route.

Every active card intentionally has full one-of-N authority: it can transfer
all vault funds, invite another card and remove another active card. Adding a
card therefore adds another complete authorization path.

Never report private keys, card passwords, recovery credentials, mnemonic
phrases, keyring contents or private operational logs in a public issue. A
useful report contains only public transaction hashes, contract addresses,
code IDs, block heights, software versions and sanitized errors.
