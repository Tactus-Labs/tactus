# A3 admission composed with a canonical SettlementTip

On 10 October 2026, **CKB 0.210.0** completed the preparation path joining two
A3 lanes, authenticated sealing, nine canonical publications, a typed ending
checkpoint and an atomic SettlementTip deployment. The actual run records
**17 committed transactions and four precise script rejections**. This is a
verified proof input, **not a completed A3 execution proof or settlement**.
G2, G3 and production readiness remain OPEN; there is no withdrawal authority.

## Exact measured path

The genesis transaction creates the Anchor, schedule, both lanes, canonical
allocation and uninitialized Tip atomically. Tip output index 5 binds the CKB
chain, exact Anchor type, unchanged SP1 guest key, checkpoint program and
allocation commitment. Its state and header roots remain zero until proof.

Two fee-paying actors admit four inputs in alternating lanes: signed Ethereum
nonces 0 and 1, malformed wire byte `01`, and signed nonce 2. Eight empty genesis
epoch batches advance the gate to its seal boundary. The complete two-lane seal
commits the original lane histories. The next mandatory batch consumes its four
round-robin duties, including the malformed input at slot 2. Native execution
records `Success, Success, Malformed, Success`, with three transactions/receipts;
the malformed input consumes zero EVM gas and has no Ethereum transaction index.

Actual CKB script checks reject omission of the full mandatory prefix, reordering
the first two inputs and omission of the malformed duty, each at the A3 gate
with error 15. A malformed execution proof reaches SettlementTip verification
and rejects with error 9. No obligation is labeled proof-settled.

Fresh processes independently recover four admissions, one seal and nine batches
from canonical CKB blocks, and recover **nine published, zero settled batches**
from the unchanged Tip. An independent Python checker reconstructs lane history
hashes, binds the complete immutable snapshot, decodes raw batch payloads and
reconciles the ordered duties with the exported interval and pending Tip. Nine
altered-evidence controls reject substituted snapshots, reordered batches, changed
lane/Tip identities, missing invalid slots, false fulfillment/readiness and a
rejection from the wrong RPC cause.

## Execution and proving input

The unchanged shared guest statement independently replays canonical allocation
and all nine batches, matching all 768 expected public bytes:

- Journal SHA-256: `72382e25dce7c3cba0e47702e9dcbc0d3ddf87c52e6126237bcbb78c5473240f`.
- Final state root: `1dc8ce7e500d0a3107311c94119589e9208e8517992ac275041a0242ebd5fd45`.
- Final Ethereum header: `1030192b9be2e4061b123f3224090a9d3a9eb868c5554d78fbd2a28db6ad78cd`.
- Proof scope: zero prefix batches, nine interval batches, unchanged execution
  rules and guest ELF. Native replay is not zkVM proof generation.

Separately built **Geth 1.17.8** matches state, transaction and receipt roots,
logs bloom, gas, accepted receipt status and rejection indices over all nine
Ethereum blocks. Its JSON t8n interface cannot represent the malformed envelope;
the adapter retains that original slot and its bytes explicitly, submits only
representable signed envelopes, and keeps the full canonical execution alongside
the projected comparison. **No independent Geth malformed-input classification
is claimed.** The canonical proof input itself retains all four original slots.

The earlier real first-interval proof has a different deployment domain and is
rejected by the shared bounded proof loader for this A3 input. That loader is an
artifact guard, not a cryptographic verifier. A dedicated A3 Groth16 proof and
actual Tip consumption remain necessary. `TACTUS_SEALED_PROOF_DIR` enables the
prepared real-proof path, including public-field mutation controls, replay
rejection and cold recovery, but that positive path is not measured by this run.

## Reproduction and retained evidence

[Raw evidence, manifest, node configuration, proving input, checker results and
Geth outputs](evidence/sealed-settlement-input/) are retained with SHA-256 hashes.
The source manifest describes the actual measured source at launch; later test,
formatting and oracle additions are separate changes. Only CKB 0.210.0 is used.
The user's existing node is not involved.

```sh
TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_DEVNET_RPC_PORT=18724 TACTUS_DEVNET_P2P_PORT=18725 \
CKB_BIN=/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
python3 scripts/check-sealed-settlement.py /path/to/run/evidence.json
python3 scripts/test-sealed-settlement.py
cargo run --locked --manifest-path proofs/sp1/Cargo.toml \
  -p tactus-o1-proof-journal --example check-chain-input -- /path/to/run/proving-input.json
cargo run --locked -p tactus-o1-execution --example sealed-settlement-oracle -- \
  /path/to/run/proving-input.json /path/to/oracle-output
python3 scripts/check-execution-geth.py /path/to/geth-1.17.8/evm /path/to/oracle-output
```

Local driver tests, Clippy for both affected crates, the archived first-settlement
and P2P rollback checkers, the new evidence controls and the independent Geth
comparison pass. CI includes archive integrity, the evidence controls, canonical
statement replay and the Geth comparison; remote CI execution is not claimed.
