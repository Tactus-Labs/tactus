# CKB authenticated reads of settled EVM state

Status: measured on **CKB 0.210.0**. This is an immutable read certificate,
not a vault, bridge, deposit path, or withdrawal authorization. G1–G9 remain OPEN.

## Result

The actual A3 nine-batch Groth16 proof settles the canonical Tip, then a separate
CKB-VM type script authenticates three account/slot reads against that live Tip.
All 23 transactions commit; all 25 negative controls reject at the expected
script and error code. The A3 baseline contributes 18 commits/eight rejections;
the new phase contributes deployment, three reads, owner destruction and 17
negative controls. The first 26 baseline records still reconcile with the real
A3 proof archive, including the original settlement transaction hash.

| Read | CKB cycles | Transaction bytes | MPT witness bytes | Certificate capacity |
| --- | ---: | ---: | ---: | ---: |
| Funded sender | 1,864,239 | 1,424 | 299 | 406 CKB |
| Transfer recipient | 1,866,045 | 1,333 | 208 | 406 CKB |
| Absent zero address | 1,742,512 | 1,222 | 97 | 406 CKB |

Each new transaction pays the fixture's one-CKB fee. Certificate destruction
returns capacity to its ordinary lock owner; retained canonical creator and
consumer transactions establish the spend even when `get_live_cell` returns
`unknown`. The initialized SettlementTip remains live with identical data.
These are devnet measurements, not a production fee or throughput prediction.

## Verification boundary

`proofs/ckb-state-proof` is an isolated, pinned no-std crate using alloy-trie MPT
verification. Its type script arguments are `TO1MPT01` plus the trusted
SettlementTip type-script hash. Creation requires exactly one group output,
zero group inputs, and exactly one live cell dependency with that identity.
The Tip must be initialized. State root, settled EVM header hash, and batch
boundary must match the certificate. The caller must select the actual trusted
SettlementTip identity; accepting arbitrary script arguments does not establish
rollup provenance.

Account proofs verify the Keccak address path and RLP nonce, balance, storage
root and code hash. Slot proofs verify the Keccak 32-byte key path and RLP value;
a zero slot is an exclusion proof. Absent accounts require zero metadata and
an empty storage proof. Empty tries have the unique empty proof encoding.

A certificate can be destroyed under its owner lock. Replacement/mutation is
rejected even with unchanged data. Certificates authenticate the root at their
creation, not the current root forever. Copying a historical certificate must
never be treated as authorization to release assets. Reorg behavior of these
certificates has not yet been separately qualified.

## Wire and resource bounds

The exact 272-byte certificate is:

| Offset | Field |
| --- | --- |
| 0..8 | `TO1EVMR1` |
| 8..40 | State root |
| 40..72 | Settled EVM header hash |
| 72..80 | Settled batch count, little endian |
| 80..100 | Account address |
| 100 | Exists flag, 0 or 1 |
| 101..104 | Zero reserved bytes |
| 104..112 | Nonce, little endian |
| 112..144 | Balance, big endian |
| 144..176 | Storage root |
| 176..208 | Code hash |
| 208..240 | Storage key, big endian |
| 240..272 | Storage value, big endian |

Witness 0's Molecule `WitnessArgs.output_type` contains `TO1MPW01`, then account
and storage proof vectors. Each vector has a u16 little-endian node count;
each node has a u16 little-endian byte length followed by its RLP bytes. Limits:
65 nodes/vector, 1,024 bytes/node, 128 KiB proof, 132 KiB outer witness, and 64
resolved dependencies. Trailing, truncated, oversized or noncanonical framing
fails closed. The ordinary secp signature includes the output-type witness.

## Validation and retained evidence

- Actual CKB negatives cover root/header/batch mismatch; address/nonce/balance/
  storage-root/code-hash forgery; invented storage; reserved bytes; absent,
  trailing and truncated proof; absent or incorrect Tip; duplicate outputs;
  and certificate replacement at input 1.
- Four native tests cover 285 account/slot fixture cases independently verified
  by pinned Geth, including nonzero storage and absent accounts, plus tampering,
  bounds and Tip binding. The three live A3 reads contain only zero storage
  slots; native nonzero-storage cases do not imply a measured nonzero-storage
  CKB transaction against this settled A3 root.
- The evidence checker reconciles the real proof, deployed program identity,
  exact typed outputs/witnesses, Tip dependency, query bytes, rejection origins,
  capacity conservation, cycles, and canonical destruction. Fourteen forged
  evidence controls must fail. This checker is not a cryptographic verifier.
- Driver tests (22), crate tests (4), format checks and Clippy pass. Root Cargo.lock
  and the A3 proof/guest/settlement program remain unchanged.

Archive: [evidence and measurements](evidence/settled-state-proof/0.210.0/check.json),
with compressed raw receipts, node log, configs, source/binary manifest and
SHA256SUMS. Raw run: `artifacts/sealed-settlement-GPBYbhjr`.
Verifier ELF SHA-256:
`3f4f86e1bfdd87d45360443ab6e88721d118b17cdfa799cc85dc1f6870f33396`.

Reproduce with an isolated port pair:

```sh
CKB_BIN=/path/to/ckb-0.210.0 \
TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_SEALED_PROOF_DIR="$PWD/specs/evidence/sealed-chain-proof/proof" \
TACTUS_STATE_PROOFS_JSON="$PWD/specs/evidence/observer-state-proofs/state-proofs.json" \
TACTUS_CKB_RPC_ADDR=127.0.0.1:18744 \
TACTUS_DEVNET_RPC_PORT=18744 TACTUS_DEVNET_P2P_PORT=18745 \
scripts/run-devnet-experiments.sh
python3 -B scripts/test-settled-state-proof.py
cargo test --locked --manifest-path proofs/ckb-state-proof/Cargo.toml
```
