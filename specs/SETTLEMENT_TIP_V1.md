# SettlementTip prototype and atomic genesis

Status: atomic initialization and rejection boundaries verified on CKB 0.121.0
and 0.210.0, 10 October 2026. **No valid execution proof has yet advanced this
Tip. G3 remains OPEN.** The contract has no deposit or withdrawal authority.

## Pinned deployment and state

The isolated `proofs/ckb-settlement` type script uses 168-byte arguments:
`TO1CFG01`, CKB genesis hash, Anchor type hash, SP1 guest key, checkpoint code hash,
and genesis-allocation commitment. Each hash/key occupies 32 bytes. Applications
must pin the complete approved deployment; arbitrary caller-selected configuration
does not identify that deployment. The bootstrap driver reads its actual devnet
genesis and uses the already-proved guest key, not synthetic 01/02/03 identities.

The 280-byte Tip data is `TO1TIP01`, one initialized byte, seven zero reserved
bytes, the full 200-byte Anchor state, a 32-byte Ethereum state root, and a 32-byte
Ethereum header hash. Creation requires an uninitialized Tip with zero root/hash
placeholders alongside the corresponding real Anchor genesis output. The Anchor
must have no matching input, and its rollup/allocation arguments must match the
Tip configuration. This prevents later bootstrapping from a live Anchor dependency.
The Anchor's tested unique-genesis rule supplies the identity boundary.

After creation there is exactly one group input and one group output, with
unchanged capacity and lock. The permissionless head lock binds the Tip type hash.
No administrator signature can replace the proof checks.

## Proof-consuming transition

The first Tip group input's WitnessArgs.input_type is:

```
TO1SETW1 || execution_journal_768_bytes || proof_length_u32_le || SDK_Groth16_wire_proof
```

The entire witness is bounded to 8,192 bytes, proof bytes to 1..4,096, and cell
searches to 64 inputs/outputs/dependencies. Framing is exact; trailing bytes fail.
The script checks the TO1PRF01 profile, configured network/Anchor/allocation,
its own actual type hash, rollup and chain. The journal's before/after Anchor
states must equal the consumed/created Tip states. Initialized Tips also bind
both predecessor state and header hashes. Batch, block and timestamp cursors
advance strictly. The first proof derives genesis execution from the canonical
allocation; zero placeholders are not accepted as a proved Ethereum state.

The exact ending Anchor must be present in a live dependency with the pinned
[checkpoint type](HISTORY_CHECKPOINT_V1.md), including its Anchor argument and
`data1` code hash. The current mutable Anchor is unnecessary. Finally, the real
SP1 Groth16 verifier checks the entire journal, pinned guest key and release
wrapper VK. There is no mock-success branch or skipped verification mode.

Error codes: 1 configuration; 2 group shape; 3 Tip data; 4 bootstrap/Anchor lookup;
5 witness framing; 6 journal deployment/profile; 7 state continuity; 8 ending
checkpoint; 9 cryptographic verification; 10 capacity/lock preservation.

## Measured evidence and limits

[Retained raw evidence](evidence/settlement-bootstrap/manifest.json) records four
committed transactions and 20 script-specific rejections per version. Initialization
uses 1,828,426 cycles for the complete transaction. Both versions create identical
deployment identities and export identical expected proof journals.

Controls reject nonzero initial roots, premature initialization, reserved/short
data, wrong allocation, duplicate Tips, absent or dependency-only Anchor genesis,
missing checkpoint, wrong deployment/continuity fields, and empty or malformed
proofs. The malformed-proof control deliberately supplies invalid bytes and must
fail. It does **not** establish that a valid proof is accepted or that later
settlement succeeds. Three host tests separately cover canonical framing,
first-transition binding, later predecessor roots and strict advancement.

The driver reconstructs allocation and batch bytes from canonical CKB history
before exporting `proving-input.json`. A separate native example in the exact
guest journal crate replays both exports and matches all 768 journal bytes.
This is independent execution checking, not a zk proof. The Python evidence
checker reconciles deployment scripts, allocation, typed checkpoints, journal
fields, interval digest and exact script rejection identities.

The first real-node attempt exposed an upstream runtime incompatibility:
lazy_static's atomic `lr.w` initialization is unsupported by CKB-VM. The narrowly
scoped [single-thread adapter](../proofs/vendor/lazy_static-1.5.1/CKB_ADAPTER.md)
changes initialization only. Its explicit target guard, one-time initialization,
reentrancy/poison controls and upstream Cargo-checksum provenance are checked.
SP1/BN254 arithmetic, verification keys and the execution guest remain unchanged.
Valid-proof acceptance must still be measured on both CKB versions.

```bash
TACTUS_DEVNET_SUITE=replay-settlement-bootstrap CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
python3 scripts/check-settlement-bootstrap.py /absolute/path/to/evidence.json
cargo +1.97.1 run --locked --manifest-path proofs/sp1/Cargo.toml -p tactus-o1-proof-journal --example check-chain-input -- /absolute/path/to/proving-input.json
```

Accepted proof succession, replay rejection after settlement, independent prover
recovery and Tip rollback under reorgs remain required. Production genesis/supply,
L1 time bounds, obligation discharge, custody/exits and governance are also open.
