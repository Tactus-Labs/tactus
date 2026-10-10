# Experimental Groth16 proof receipts

Status: real Groth16 proof and actual CKB acceptance measured on both 0.121.0
and 0.210.0, 10 October 2026. Each node committed two receipts and rejected all
28 negative controls. The original prover was OOM-killed in final wrapping; its
retained recursive witness completed successfully in a fresh process. See the
[measured report and retained proof](EXECUTION_GROTH16_PROOF_REPORT.md).

The isolated `proofs/ckb-proof-check` workspace pins `sp1-verifier` 6.8.1 with
default features disabled. Its RISC-V type script verifies a Groth16 proof using
the crate's embedded wrapper verification key and a 32-byte SP1 guest key in the
type arguments. Keeping the prover and verifier in separate workspaces preserves
the root Cargo.lock, which is part of the existing execution rules commitment.
Both CKB verifier components now use the narrowly scoped
[lazy initialization adapter](../proofs/vendor/lazy_static-1.5.1/CKB_ADAPTER.md)
after a real-node test exposed unsupported atomic instructions in the upstream
initializer. Cryptographic algorithms and keys are unchanged.

## Cell and witness

A receipt has exactly one group output, no group input, and exactly 768 bytes of
output data: the entire [execution public journal](EXECUTION_PROOF_V1.md).
The verifier reads those actual cell bytes; the transaction cannot substitute a
separate public-value claim in its witness. Both spending and replacing a receipt
fail, regardless of its lock. Reusing a proof to create a second receipt is allowed.

The first global input's canonical Molecule WitnessArgs.input_type contains:

```
"TO1G1601" || proof_length_u32_le || SP1_SDK_proof_bytes
```

The complete witness is bounded to 8,192 bytes and proof bytes to 1..4,096.
Truncation, trailing framing bytes, missing input_type, wrong journal length and
wrong key length fail. The verifier receives the SDK's wire proof bytes, not its
serialized host proof container. No development circuit setup is used.

Return codes are 2 for missing witness, 3 for oversized witness, 4 for witness
loading failure, 5 for framing, 6 for key/script encoding, 7 for cryptographic
verification, 8 for receipt group shape, and 9 for journal loading/length.

## Reproduction and evidence

`scripts/run-proof-experiment.sh 0 prove-groth16` builds the native Go-backed
SP1 host, uses release circuit mode, generates a local proof, tests public-value
and guest-key mutations, and verifies again in a fresh process. Set
`SP1_GROTH16_CIRCUIT_PATH` to the directory containing the released `v6.1.0`
Groth16 artifacts. SDK 6.8.1 names that circuit version; it is not an SDK downgrade.
The runner records circuit-file hashes. Released key identity alone is not an
independent trusted-setup or compiler audit.

Once that command succeeds, use its `proof` directory with:

```bash
TACTUS_PROOF_DIR=/absolute/path/to/completed/proof \
TACTUS_DEVNET_SUITE=replay-proof-verifier \
CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
python3 scripts/check-proof-receipts.py /absolute/path/to/evidence.json /absolute/path/to/completed/proof
```

The real-node driver first estimates the valid transaction, then requires exact
type-script return codes for 28 negative controls. It publishes two receipts and
records actual committed transaction bytes, estimated VM cycles and locked
capacity. Both versions completed these assertions; raw transactions and logs are retained
in the report archive.
The independent Python checker reconciles journals, witnesses, mutation offsets,
receipt outpoints and rejection identities; it does not perform cryptography.

## Settlement boundary

These receipts authenticate computation under their selected guest key. They do
not authenticate an ordering deployment or advance SettlementTip. The first
fixture proof has explicitly synthetic network/ordering/settlement identities.
Anyone can select another guest key and create a separately typed receipt; a
future settlement script must pin its own authorized key and exact deployment.

Settlement still needs authenticated canonical history, predecessor state/header
continuity, actual CKB and allocation identities, and contiguous interval checks.
An immutable untyped data cell is not a canonical history certificate. A live
anchor dependency also becomes stale whenever ordering advances. [Immutable checkpoints](HISTORY_CHECKPOINT_V1.md) now authenticate Anchor
successors, including measured P2P reorg rejection. [SettlementTip](SETTLEMENT_TIP_V1.md)
has measured atomic genesis and negative controls, with valid-proof succession
still awaiting its first real-domain proof. This journal profile authorizes no
withdrawals. G3 and the production goal remain open.
