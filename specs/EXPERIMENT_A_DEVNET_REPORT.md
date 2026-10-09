
## Current blocker (recorded 2026-10-09)

The A1 replay is blocked at the SECP256K1/blake160 lock, and the evidence
isolates the cause outside this driver:

- molecule encodings are byte-identical to `ckb-cli molecule encode` ground
  truth; ckbhash and blake160 (truncated ckbhash) match `ckb-cli util`
  oracles; signatures recover locally to the exact lock args;
- **a transaction built and signed by `ckb-cli` itself — spending coinbase
  cells this chain paid to the ckb-cli-derived lock args — is rejected by
  this node with secp error -31** (recovered-pubkey/args mismatch), and
  self-signed ones with -101;
- the secp code cell (genesis tx0 output 1, data hash `709f3fda…`) matches
  the documented latest system-script version.

Conclusion: SECP signature verification fails chain-wide on this local
**ckb v0.210.0 devnet** regardless of signer. Next steps for the next
session: (1) retry on an LTS ckb release (e.g. v0.118.x/v0.121.x), (2) or
trace the secp script with `ckb debug-print`/cycle accounting to locate the
verification divergence, (3) verify the devnet genesis secp cell data against
the released `ckb-system-scripts` artifact byte-for-byte.
