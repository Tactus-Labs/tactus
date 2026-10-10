# A3 obligation recovery across a real P2P reorg

On 10 October 2026, two **CKB 0.210.0** nodes replaced an eight-block branch
containing the A3 seal and its mandatory publication. Fresh observers on both
nodes restored **four admitted duties, zero sealed/published duties, eight
published batches and zero settled batches**. A new complete seal and publication
then restored four published duties and nine batches, still with zero settlement.
This qualifies pending-duty rollback and republication, not proof settlement,
miner fairness, an unplanned network fault, or production readiness.

## Actual fork and recovery

Both nodes first synchronize the atomic Anchor/A3/SettlementTip deployment,
four admissions and eight empty genesis-epoch batches. After actual P2P
partition, the main node seals both lanes and publishes the four required inputs
with a typed ending checkpoint. The peer consumes only the seal transaction's
fee input and mines the longer branch. This prevents the orphan seal from
silently returning through the transaction pool. Reconnection uses peer block
synchronization; neither `truncate` nor `submit_block` is used.

The old seal and publication cease to be canonical, and their checkpoint becomes
non-live. Both fresh `recover-obligations` processes return identical results at
the winning block: each original `(gate, lane, sequence, payload, admission)`
identity remains present, but its seal, publication and outcome are absent.
The atomic Tip remains uninitialized. The observer does not keep an orphan
publication marker or turn publication into proof fulfillment.

The exact old signed seal fails with `TransactionFailedToResolve`, naming its
consumed funding outpoint. Using the restored authenticated lanes and new peer
fee output creates the replacement seal:
`0x83857cc725d91ff40c7e11f67f98375601cd31f73f25ff5d6c379430eb4ac723`.
The restored Anchor and replacement gate then create publication:
`0x0c19ce375ea66cffe8c5839e229559c83bf61e27fcd3cc8191507db9e1397041`.

Replacement snapshot, canonical batch bytes, Anchor state and checkpoint data
match their original values. Both nodes recover the same four publication
locations and native outcomes, including the malformed input at slot 2. No
admission is duplicated, omitted or labeled proof-settled.

## Evidence and checks

The run contains **20 historical committed records, including the orphan seal
and publication, and five rejections**. This does not mean all 20 are currently
canonical. The independent checker reconciles the original pending-duty run,
fee-only conflicting transaction, branch heights, orphan statuses, exact rejected
seal, restored lanes, replacement snapshot/publication and both pairs of fresh
observer reports. Eight corrupted-evidence controls reject retained orphan state,
a live orphan checkpoint, a different peer prefix, a missing duty, wrong replacement
identity, a local-truncation claim and fabricated settlement.

[Raw evidence and SHA-256 manifest](evidence/obligation-reorg/) retain the complete
transaction history, recovery reports, both node logs/configurations and measured
source manifest. CKB RPC normalizes an absent output type as `null`; reconciliation
normalizes only that optional field before comparing complete transaction fields.
The future A3 proof checker uses the same observed normalization for canonical
consumption records. This does not permit a changed type script.

P2P connection, partition, mining and convergence helpers are now shared with the
already measured first-proof rollback harness. Clippy, 22 driver tests and the
retained first-proof reorg checker pass after extraction. CI checks archived A3
rollback evidence and its corrupted controls; remote CI execution is not claimed.

```sh
TACTUS_OBLIGATION_REORG=1 TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_DEVNET_RPC_PORT=18734 TACTUS_DEVNET_P2P_PORT=18735 \
TACTUS_PEER_RPC_PORT=18736 TACTUS_PEER_P2P_PORT=18737 \
CKB_BIN=/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
python3 -B scripts/check-obligation-reorg.py /path/to/run/evidence.json
python3 -B scripts/test-obligation-reorg.py
```

The launcher permits this mode only in the A3 suite without an A3 proof directory,
checks all four ports, starts only its isolated nodes and stops those nodes on
exit. A future proof-consuming A3 rollback needs separate measured evidence.
G2/G3/G6 and production readiness remain OPEN.
