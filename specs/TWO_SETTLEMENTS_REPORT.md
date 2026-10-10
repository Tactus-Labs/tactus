# Two consecutive real proof-consuming settlements

On 10 October 2026, **CKB 0.210.0** accepted two distinct canonical-domain
Groth16 execution proofs in sequence. The second consumes the first proved Tip,
advances from batch one to batch two and changes the Ethereum state/header roots.
The actual suite records **seven committed transactions and 62 precise script
rejections**. Fresh recovery distinguishes publication from settlement and ends
with exactly **two published batches, two settled batches and two proved
transitions**. G3 and production readiness remain OPEN; no withdrawal authority
is implemented.

## Completed second proof

The unchanged guest replays canonical allocation and the first batch as its
prefix, then proves the second nonempty interval containing the signed nonce-3
transfer. Native replay and zkVM execution produce the exact expected journal.
The completed proof has 768 public bytes, a 356-byte CKB wire proof and a
2,463-byte SDK container. Its journal SHA-256 is
`9ade3461ac1261d3b171b0729f150e7cc327906552d47fd3cd3fcdd10156ea5c`.
Fifteen real cryptographic controls reject fourteen modified public fields and
one wrong guest key. A fresh SDK process independently replays the canonical
input and verifies the proof against the actual pinned ELF.

This run also completes the first end-to-end use of the automated staged runner.
The recursive phase took **3,486.01 seconds** (58:06.01), with maximum RSS
20,887,040 KiB. The coordinator retained and fsynced the complete 2,802,650-byte
witness, intentionally terminated its own child with SIGTERM, then ran final
wrapping in a fresh process. Wrapping plus its 15 verification controls took
**131.04 seconds** (2:11.80 externally), with maximum RSS 17,538,792 KiB.
Fresh verification initialized its prover in 41.38 seconds and then succeeded.

The 131-second figure is not total proving latency. The original child exit -15
is the recorded deliberate phase handoff, not an OOM or an uninterrupted
monolithic success. This demonstrates the supported staged workflow; it does not
establish production affordability, sustained throughput, independent operators
or bounded full-prefix replay cost. The guest and circuit verification are
unchanged, without mock proofs, development circuits or key-verification bypass.

[Second proof, complete witness, native/SDK logs and manifests](evidence/second-chain-proof/)
are retained. The launch environment's `proof_completed=false` describes launch,
while `completion.json` records verified success. `wrap-result.json` preserves
wrapper metadata before the fresh canonical verification fields were added.

## Actual CKB continuity

| Transition | Transaction | Cycles | Complete wire bytes | Fee |
|---|---|---:|---:|---:|
| Genesis → batch 1 | `0xe66f6edcdde555bda68dceb081e7d9bdd70da8986b45fb3dbcc42b30b27fd496` | 3,985,610,270 | 2,403 | 1 CKB |
| Batch 1 → batch 2 | `0xad2d56cbd76f42d785fd99b14f521ae77dc263668b3aac60cb7b241df8933b55` | 3,983,695,883 | 2,403 | 1 CKB |

Both transitions preserve the exact 554 CKB Tip capacity, type and lock. The
second requires the first proved state/header and authenticates its own immutable
ending checkpoint. Its final state root is
`1afe4d4380885cb37d1f7179daa70cd8b97e381bee3e291581c5c4b9e7484698`,
and final Ethereum header is
`b4da86f67ab735f65fc31c5ee2875d18197ea8bae40e75480e871d93efc46c70`.
Both predecessors are established as consumed through their canonical creation
and exact canonical consumer, even where `get_live_cell` returns `unknown`.

The 62 rejections include the bootstrap and first-proof controls, attempting the
second proof against an uninitialized Tip before the first proof, second-proof
public-field and proof-byte changes, coordinated state/header substitutions,
replaying either proof against the final successor, and rollback to the first
Tip. A published second batch does not advance the proved boundary by itself.
The independent checker reconciles exact proof bytes, journal continuity,
checkpoints, fee cells, on-chain cycles, complete wire sizes and cold recovery.
Six corrupted-evidence controls reject a changed boundary, live predecessor,
wrong/noncanonical consumer, wrong proof source and false fee.

[Raw node receipts, logs, source/config manifests and checker output](evidence/two-settlements/)
are retained with checksums. CI reconciles them and runs the corruption controls;
that Python check does not independently re-execute cryptography. Remote CI
execution is not claimed.

```sh
TACTUS_SETTLEMENT_PROOF_DIR="$PWD/specs/evidence/chain-groth16-proof/proof" \
TACTUS_SECOND_SETTLEMENT_PROOF_DIR="$PWD/specs/evidence/second-chain-proof/proof" \
TACTUS_DEVNET_SUITE=replay-settlement-bootstrap CKB_BIN=/path/to/ckb-0.210.0 \
  scripts/run-devnet-experiments.sh
python3 -B scripts/check-two-settlements.py /path/to/run/evidence.json \
  specs/evidence/chain-groth16-proof/proof specs/evidence/second-chain-proof/proof
python3 -B scripts/test-two-settlements.py
```

The dedicated nine-batch A3 proof is now running separately. Its proof-consuming
obligation fulfillment and rollback, key/setup provenance review, production
resource limits, independent proving/recovery and asset exits remain separate
requirements.
