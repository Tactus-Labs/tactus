# Transition-authenticated history checkpoints

Status: bounded mechanism verified on isolated CKB 0.121.0 and 0.210.0,
10 October 2026. This authenticates ordering history, not EVM settlement.

## Contract

The isolated `proofs/ckb-history-checkpoint` program is a creation-only type
script. Its arguments are exactly the 32-byte hash of the associated Anchor type
script. Its sole group output contains the complete 200-byte `TO1ANC01` Anchor
state. It rejects any group input or second group output, so checkpoints cannot
be consumed, replaced or merged.

Creation requires exactly one transaction input and one output with the selected
Anchor type hash. Thus that Anchor's type program must execute in this same
transaction. Merely naming an Anchor in cell_deps is insufficient. The checkpoint
must equal the actual Anchor output byte for byte. An additional check requires
the batch cursor to advance by exactly one without overflow and preserves the
rollup, execution, DA, limits and chain domain fields. The Anchor program itself
enforces the batch commitment, data publication and complete transition rules.

Input and output searches are bounded to 64 cells per side. A transaction beyond
that boundary is rejected. Missing/multiple Anchor matches return 4, malformed
checkpoint/Anchor data 5, non-advancing/domain-changing states 6, and mismatched
checkpoint contents 7. Wrong argument length returns 1; receipt group shape
violations return 2. Genesis-only checkpoint creation is not supported.

A consumer must pin the full checkpoint type, including its code hash/hash type
and the actual authorized Anchor type hash. An arbitrary checkpoint for an
attacker-selected type is not authenticated history of the intended rollup.
No code update, operator key, or untyped data cell may substitute for that identity.

Checkpoints are optional additions to existing valid Anchor advances. They do not
modify Anchor validation, execution semantics, the root Cargo.lock, or the guest
ELF already being proved. This is not yet a mandatory checkpoint retention policy.

## Measured real-node boundary

Raw transactions, node configuration, manifests, logs and the exact script are
retained in [the evidence archive](evidence/history-checkpoint/manifest.json).
Both node versions used fresh funded dummy chains and deterministic mining.
The tests used two empty-input ordering batches and an explicit laboratory rules
hash; they do not demonstrate Ethereum execution or a production deployment.

| Measurement per version | CKB 0.121.0 | CKB 0.210.0 |
|---|---:|---:|
| Committed transactions including setup | 5 | 5 |
| Accepted checkpoints | 2 | 2 |
| Rejected negative controls | 22 | 22 |
| First complete advance transaction cycles | 1,799,031 | 1,799,031 |
| Second complete advance transaction cycles | 1,807,877 | 1,807,877 |

These are complete transaction estimates, including the Anchor and lock programs,
not isolated checkpoint-instruction counts. The second advance includes the first
checkpoint as a live dependency. Both checkpoint cells remain live with exact
data after both advances. By contrast, adding the original, now-spent Anchor as
a dependency fails resolution on the same node.

The 22 controls are ten checkpoint field mutations; wrong Anchor identity;
short key; short/long data; duplicate checkpoint outputs; creation without an
Anchor transition; Anchor supplied only as a dependency; consumption/replacement
of each of the two checkpoints; and the stale mutable-Anchor dependency.
Script failures must identify the exact checkpoint code and expected return code.
Each mutated transaction is signed again so lock failures cannot mask the result.

`scripts/check-history-checkpoints.py` independently reconciles the raw transition
outpoints, serialized Anchor type hash, checkpoint bytes, historical dependency,
field mutations and script rejection identities. Five deliberately corrupted
evidence copies are rejected. This checker does not execute scripts; the real
nodes supply that evidence.

## Reproduction and remaining work

```bash
TACTUS_DEVNET_SUITE=replay-history-checkpoint CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
python3 scripts/check-history-checkpoints.py /absolute/path/to/evidence.json
```

Use `TACTUS_CKB_VERSION=0.210.0` with that version's binary for the compatibility
run. The launcher refuses occupied ports and stops only its own node.

A future SettlementTip can validate the proven ending Anchor against a pinned
checkpoint dependency while current ordering continues. That consumer is not
implemented or qualified here. Proof/public-journal binding, actual deployment
identity, contiguous predecessor settlement, proof-bound obligation discharge,
withdrawal conservation and checkpoint recovery across unplanned reorgs remain
open. A subsequent [two-node P2P reorg experiment](CHECKPOINT_REORG_REPORT.md)
now replaces four canonical blocks on both CKB versions, rejects dependencies on
orphaned checkpoints, and accepts the winning checkpoint. That controlled result
does not qualify unplanned faults or proof-consuming settlement. G3 remains OPEN.
