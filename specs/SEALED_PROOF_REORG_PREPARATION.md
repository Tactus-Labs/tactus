# A3 proved-obligation rollback preparation

This document preserves the pre-measurement design. The prepared mode is now
measured successfully; see [real A3 proof fulfillment and rollback](SEALED_PROOF_SETTLEMENT_REPORT.md)
for actual transactions, costs, raw archives and verification. The active queue
has completed and no prover restart was needed.

After canonical A3 publication and checkpoint creation, both CKB 0.210.0 nodes
synchronize and partition. The primary must perform real proof verification,
advance its Tip and cold-recover four settled duties. The other branch consumes
only the proof's fee input and grows longer. Actual P2P rejoining must orphan the
proof transaction, restore the initial Tip, and let fresh observers on both nodes
recover **four published, zero settled duties**, without changing their admission,
seal, publication, payload or native execution identities.

The original signed transaction must fail to resolve its consumed funding. A
replacement consumes the restored Tip and the peer's fresh fee output, reusing
exactly the same proof, journal, checkpoint and successor data. Both cold observers
must then agree on four settled duties and **one** canonical proved transition.
The orphan transition must not be counted twice. No new proof or altered batch
input is permitted as a substitute for proof reuse.

`scripts/check-sealed-proof-reorg.py` independently reconciles the actual first
proof phase, historical commit/rejection counts, fee conflict, orphan states,
canonical restored-Tip consumption, exact reused witness, and both pairs of
cold reports. It does not perform cryptographic verification itself. The positive
path now has actual receipts in the linked result archive. The preparation guard
rejects existing unproved A3 evidence as a completed proof-reorg result.

Clippy and the 22 driver unit tests pass. Three invalid launcher combinations
(missing proof, conflicting pending-duty reorg mode, or a non-1 reorg flag) fail
before node startup. The launcher allocates a second isolated node only for this
explicit mode. The current pending-duty reorg/crash suite remains separate.

```sh
TACTUS_SEALED_PROOF_DIR=/absolute/path/to/completed-a3-proof \
TACTUS_SEALED_PROOF_REORG=1 TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_DEVNET_RPC_PORT=18734 TACTUS_DEVNET_P2P_PORT=18735 \
TACTUS_PEER_RPC_PORT=18736 TACTUS_PEER_P2P_PORT=18737 \
CKB_BIN=/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
python3 -B scripts/check-sealed-proof-reorg.py /path/to/run/evidence.json \
  /absolute/path/to/completed-a3-proof
```

G2/G3/G6 remain open for broader production requirements despite the now measured
proof-bound A3 fulfillment and rollback. Compilation and queued work are
not a passing experiment.
