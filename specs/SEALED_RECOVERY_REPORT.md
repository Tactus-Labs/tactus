# Independent A3 cold recovery

A new read-only `recover-sealed` process reconstructs the authenticated A3 anchor,
Schedule, active lanes and current immutable snapshot from canonical CKB blocks.
It takes only trusted deployment identities and the CKB RPC address. It does not
read the publisher's memory, journal, indexer, wallet keys or saved state JSON.

Both **CKB 0.121.0 and 0.210.0** pass **15 fresh-process reconstructions and three
wrong-domain rejection controls per version**, alongside the existing 210 committed
transaction events and 69 expected script rejections. A recovery result is used
to build the replacement seal after a planned rollback. **Production readiness,
G2 and G6 remain OPEN**: this recovers publication duties, not validity proofs,
proven settlement or unplanned network failover.

## Trusted inputs and output

```bash
TACTUS_CKB_RPC_ADDR=127.0.0.1:18714 \
  cargo run --locked --bin recover-sealed -- \
  CKB_GENESIS_HASH GATE_TYPE_SCRIPT_HEX ANCHOR_TYPE_SCRIPT_HEX
```

The three positional arguments are exact deployment configuration: a 32-byte CKB
genesis hash, the complete Molecule gate type script, and the complete Molecule
anchor type script. Both scripts use `data1`, pinning their program content hashes.
The gate role/identity and anchor identity lengths are checked before scanning.
Supplying a different chain, gate or anchor is an error, not a request to adopt
whatever state an indexer finds.

The JSON result contains the pinned block height/hash, network identity, canonical
admission/seal/batch counts, full state bytes, exact outpoints, locks, type scripts,
capacity reservations and current snapshot bytes. Active queues retain the bytes
needed for the next seal. The current snapshot retains its complete contents even
when its publication cursor is exhausted. `settlement` is explicitly `NOT_PROVEN`.

The configured CKB node remains the consensus trust boundary. The observer is not
a new CKB consensus client and does not independently execute every CKB script or
verify PoW. The supplied deployment configuration must be trusted. RPC timeouts and
response size are bounded by the existing transport; the number of blocks in a
full scan grows with chain history. This is a cold correctness path, not a qualified
production archival service or fast checkpoint implementation.

## Replay and canonicality checks

The observer pins the current tip, scans block heights from zero, checks every
parent link and the exact genesis hash, then confirms that the pinned block still
belongs to the canonical chain. Normal extension after the pinned block is allowed.
Replacement of that block invalidates the result and requires a fresh scan.
Outpoints represent the pinned prefix; later normal transactions may spend them,
so callers must handle ordinary stale-input rejection when submitting work.

For the selected deployment, replay requires:

- One unique gate and anchor genesis, with Type IDs derived from the actual first
  input and absolute output indices, correct rollup/anchor binding, all configured
  genesis lanes and their required capacity reservations.
- Connected single-successor cells with unchanged capacity and expected locks.
  Consuming a tracked head without its replacement, recreating one without its
  predecessor or advancing the anchor without the gate fails.
- Every ordinary lane update to equal one authenticated append from its previous
  state; malformed roots, queue resets and no-op updates fail.
- Every seal to use all configured current lanes, reset exactly those lanes,
  produce the exact next Schedule and retain an immutable snapshot of those lanes.
- Every batch to publish immutable bytes that pass the bounded batch codec and
  the Schedule's mandatory-prefix transition, yielding the exact new anchor and
  Schedule. An internally consistent anchor with an omitted priority prefix fails.

Unrelated output cells, including counterfeit snapshot bytes published elsewhere,
are not adopted. Snapshot provenance follows the replayed all-lane seal; internal
consistency of standalone serialized snapshot bytes is insufficient.

## Real-node experiment

For each of 1, 2 and 4 configured lanes, the sealed suite starts a fresh observer:

1. At genesis.
2. After active heads have filled while an earlier snapshot finishes processing.
3. On the branch containing the second seal and its first processing batch.
4. After that seal and batch are rolled back.
5. After replacement sealing and canonical processing finish, including the
   maximum 33,385-byte snapshot in the four-lane case.

Every recovered protocol field is compared with the independently maintained
publisher state. The harness then discards its protocol state objects and decodes
replacement objects from the fresh observer's output. The recovered outpoints,
queue bytes and Schedule drive the next operations. After rollback, the second
fee payer seals those recovered heads and processes their duties successfully.
Fixture fee-wallet bookkeeping and the in-process EVM comparison are still restored
by the harness; the claim here is independent protocol-state recovery. Separate
[CKB-to-EVM recovery evidence](CKB_EVM_RECOVERY_REPORT.md) covers executed-state
reconstruction for its tested anchor workload.

The orphan observer's pinned block is explicitly rejected after rollback. The
final observer's pin remains valid after an ordinary new block. Fresh processes
also reject an incorrect chain genesis hash, gate identity and anchor identity,
checking the expected diagnostic rather than treating arbitrary process failures
as success.

Seven host tests replay an archived, signed, one-lane canonical prefix and mutate
its genesis, predecessor, capacity, lock, lane data, seal completeness, snapshot,
mandatory prefix and gate survival. The unmodified prefix reconstructs 11
admissions, one seal, 16 batches, eight active messages and the original three-message
snapshot. These tests complement real-node execution; the mutated histories are
unit inputs, not claims that CKB accepted invalid transactions.

## Reproduction and retained evidence

```bash
TACTUS_DEVNET_SUITE=replay-sealed \
  CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_CKB_VERSION=0.210.0 TACTUS_DEVNET_SUITE=replay-sealed \
  CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

The launcher builds and hashes both `replay-sealed` and `recover-sealed`. Existing
CI sealed-suite jobs therefore execute the new observers on both CKB versions.
[Retained evidence](evidence/sealed-cold-recovery) includes raw transaction and
observer results, manifests, configuration, logs, summaries and checksums. Exact
source hashes identify the tested changes before their local commit; this is not
a claim that remote CI ran on unpushed work.

Rustfmt, strict workspace Clippy and **89 workspace tests** pass. The five CKB
script ELFs, protocol wire formats and Cargo.lock are unchanged. During validation,
a separate deterministic regression exposed a journal ownership issue involving
duplicated descriptors; commit `4bc6104` fixes explicit lock release before the
final workspace and real-node runs recorded here.

A later [two-node network experiment](NETWORK_REORG_REPORT.md) also exercises
controlled P2P branch replacement and independent EVM recovery without `truncate`.

Still required are proof-bound history/settlement reconstruction by another prover,
authenticated execution genesis, network-driven reorgs, checkpoint/archival
qualification and admission liveness under sustained targeted contention. Old
snapshots remain on chain; the returned state is the current duty and active heads,
not a database of proven or claimable withdrawals.
