# Security status

This repository is an unaudited laboratory contract for `ipi-testnet-1`. Do not
use it to protect material value.

Every active card intentionally has full one-of-N authority: it can transfer
all vault funds, invite another card and remove another active card. Adding a
card therefore adds another complete authorization path.

Never report private keys, card passwords, recovery credentials, mnemonic
phrases, keyring contents or private operational logs in a public issue. A
useful report contains only public transaction hashes, contract addresses,
code IDs, block heights, software versions and sanitized errors.
