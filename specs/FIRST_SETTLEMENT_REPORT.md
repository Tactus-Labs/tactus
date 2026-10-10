# First real settlement and P2P rollback

On 10 October 2026, **CKB 0.210.0** accepted the real canonical-domain Groth16
proof and advanced the authenticated SettlementTip from its uninitialized state
to one proved batch. The complete first-settlement suite records **5 commits and
40 script rejections**, followed by independent cold recovery. A separate actual
P2P experiment then rolls that settlement back and reapplies the same proof with
fresh funding. These results establish this bounded canonical settlement path;
**G1–G9 remain OPEN, and no custody or withdrawal authority exists.**

## Verified first transition

Settlement transaction:
`0xa5faa43b8070b58439a67dfcf6dd6b2b0eb0c117a36beca51b95f7c40df78de0`.

The transaction consumes the atomic genesis Tip and authenticates the immutable
ending checkpoint. It verifies the [real canonical-domain proof](CHAIN_GROTH16_PROOF_REPORT.md),
preserves the Tip's type, lock and 554 CKB capacity, and sets the exact state root
`39d75d240f98e864a088b6003e2f94134baec5a63bd0bdea9c955e14427527e7`
and Ethereum header hash
`ee01d4c2d0b3055cce4ef9342eeeda4517f232a3e1789170d253335693b62b9d`.

Node-confirmed complete wire size is **2,366 bytes**, fee **1 CKB**, and measured
verification cost **3,985,647,009 CKB cycles**. This uses about 39.9% of the unchanged
10-billion-cycle laboratory block limit. It is a substantial proving/verification
cost, not production throughput qualification.

The existing 20 bootstrap controls still reject. Another 20 controls cover all
14 public-field changes, two proof-byte changes, coordinated journal/output state
and header changes, replay against the live successor, and rollback to genesis.
Coordinated mutations reach the real cryptographic verifier and fail there.
Fresh `recover-settlement` reconstructs one published batch and exactly one
proved transition from canonical blocks without publisher execution state.

## Spent outputs and bounded RPC waits

CKB reports the spent genesis output as `unknown` through `get_live_cell`, while
both its creation and its consuming proof transaction remain `committed`.
Treating only the string `dead` as proof of spending caused the initial harness
to report failure after a valid proof transaction had already committed.

The corrected harness requires the predecessor to be non-live, its creation to
be canonical, and a canonical successor transaction to consume that exact outpoint
exactly once. Full observations are retained and checked independently. Five
falsified-evidence controls reject missing canonical creation/consumption, the
wrong input, a live predecessor and a changed consumer identity. An unknown cell
by itself is never treated as an established consumption.

An earlier cryptographic negative control exceeded the generic 15-second RPC
read budget. `estimate_cycles` and `send_transaction` now have a bounded 120-second
read budget; other RPC reads remain at 15 seconds. The timed-out attempt remains
an incomplete run and is not counted as a script rejection. Both failed attempts,
source manifests and the direct spent-cell observation are archived alongside
the successful full rerun.

## Real P2P rollback and same-proof recovery

A second run synchronizes the initial deployment and publication to two nodes,
then partitions them. One branch commits the proof. The other spends only that
transaction's fee input and grows longer. Reconnection uses actual peer block
synchronization, without `truncate` or `submit_block`, replacing **4 blocks**.

Both fresh recovery processes then report one published batch, **zero settled
batches and zero proved transitions**, and the restored original Tip. The orphan
proof output is non-live and its transaction is not canonical. Resubmitting the
original signed proof transaction fails to resolve its spent fee input.

A newly signed transaction consumes the restored Tip and a fresh fee output while
reusing the exact proof, journal, checkpoint and successor data:
`0x4c52e7878a288ad3daf8e2c296ba021ec22e8fd822fd45ee9772d55cd601cec9`.
Both nodes then independently recover one canonical proved transition and the
same state/header roots. The orphan transition is not counted twice.

The reorg run has **7 historical committed records, including the orphan, and
41 rejections**. It is not a claim of seven presently canonical transactions.
The independent checker validates proof reuse, the competing fee input, branch
heights, restored cells, the original-transaction rejection and identical reports
from the two fresh recovery processes. Five altered-evidence controls fail.

## Scope and reproduction

[Raw evidence, node configurations, manifests and checker outputs](evidence/first-settlement/)
are retained with SHA-256 checksums. CI rechecks the actual archived transactions
and corrupted-evidence controls; this Python reconciliation does not re-execute
cryptography or consensus. The measured node performed the script verification.
Remote CI execution is not claimed.

```sh
TACTUS_SETTLEMENT_PROOF_DIR="$PWD/specs/evidence/chain-groth16-proof/proof" \
TACTUS_DEVNET_SUITE=replay-settlement-bootstrap CKB_BIN=/path/to/ckb-0.210.0 \
  scripts/run-devnet-experiments.sh
# Add TACTUS_SETTLEMENT_REORG=1 for the two-node experiment.
python3 -B scripts/test-first-settlement.py
python3 -B scripts/test-settlement-reorg.py
```

Sequential nonempty intervals, proof-bound A3 obligations, independent proving,
unplanned faults, custody/exits, finality policy and production resource limits
remain unqualified. The second interval's real proof is running separately. This
planned P2P rollback demonstrates recovery of this deployment, not a production
finality guarantee or an independent light-client consensus check.
