# A3 mandatory sealed-gate experiment

Both **CKB 0.121.0 and 0.210.0** completed the same authenticated-gate workload:
**210 committed transaction events and 69 expected rejections per version**.
This establishes a real mandatory publication mechanism under canonical batch
progress. **Full Experiment A is incomplete; G2 and production readiness remain
OPEN.** It does not establish fair admission, wall-clock liveness or validity
settlement.

Unlike the earlier immutable-copy control, this construction authenticates all
configured lane heads at sealing and forces every canonical anchor advance to
co-spend its Schedule. A builder cannot omit that gate, continue the old epoch
past its quota, select an unrelated snapshot or skip its mandatory prefix. Exact
rules, wire bytes and adapter operations are in [A3_SEALED_V1.md](A3_SEALED_V1.md).

## Measured outcomes

| Configured lanes | Active-head updates while signed batch waits | Snapshot messages after switch | Last post-seal message included within subsequent batches |
|---:|---:|---:|---:|
| 1 | 8 | 8 | 10 |
| 2 | 16 | 16 | 12 |
| 4 | 32 | 32 | 16 |

Each scenario fills the active lanes after the first seal and positions the
tracked message last in their round-robin order. The already signed snapshot
batch commits after all active updates, with its builder's fee cell independent
of the admitting actor. Conversely, a lane admission signed before an ordinary
Schedule advance still commits afterward: admission has no mutable Schedule
dependency. These are real submitted transactions, not a simulation of survival.

Every batch processes the next mandatory prefix of at most four snapshot
messages. At eight batches, attempting a ninth fails until a complete seal
switches the snapshot. The tracked message's 10/12/16-batch measurements include
the remaining eight batches of the preceding snapshot and its new snapshot's
processing position. They are **batch counts, not CKB block counts or seconds**.
The test does not turn a global halt into a wall-clock progress guarantee.

For each lane count, a planned reorg orphans the second seal and its first
processing batch. The original active heads and prior Schedule are checked live
again; the orphan snapshot is absent. A different fee payer reseals and processes
the restored obligations. The older canonical snapshot remains live and cannot
be used after the new switch. This demonstrates canonical rollback of obligations
and independent signing authority. This original run restored its own bookkeeping from saved state. The later
[independent cold recovery experiment](SEALED_RECOVERY_REPORT.md) reconstructs
Schedule, lanes and current snapshot in fresh processes and uses them to reseal
after rollback. Proving-state recovery remains outstanding.

The four-lane scenario additionally admits **32 maximum-size, 1024-byte payloads**,
seals a **33,385-byte** authentic snapshot and processes it over eight bounded
batches. All complete batch bytes are published through the existing immutable
inline-DA anchor. The pinned EVM executor replays the canonical batches, including
rollback of the orphan execution branch. Test messages are synthetic rejected EVM
inputs, so this is publication/total-outcome evidence, not successful application
execution or proof generation. Signed Ethereum execution has separate
[Geth differential evidence](EXECUTION_DIFFERENTIAL_REPORT.md).

## Adversarial controls

There are 23 negative cases per lane count, each freshly signed and checked
against the exact sealed-program hash, script location and error code:

- Omit the Schedule from an anchor spend; burn, split, take over or drain the
  Schedule; switch before the batch quota is used.
- Recreate a lane without a fresh gate genesis; reset its active queue without
  sealing; perform a no-op on a partly filled or full lane; admit a ninth message.
- Attempt a ninth batch before sealing; omit a configured lane; change snapshot
  bytes; substitute a mutable snapshot lock; hide an unmetered anchor advance
  inside a seal.
- Destroy a retained snapshot; use an unrelated dependency; omit or reorder the
  mandatory prefix; exceed the witness allocation budget.
- Keep a processed snapshot indefinitely or reference it after the required
  switch.

The counterfeit dependency is a real committed cell with internally consistent
snapshot bytes. It fails because it is not the Schedule's authenticated snapshot.
The mutable-lock control supplies sufficient occupied capacity, so a capacity
error cannot stand in for the expected protocol rejection. Expected script
failures are separated from unrelated consensus failures and RPC transport errors.

All configured head identities derive from one unique gate genesis. Each seal
consumes every configured current head and retains its exact snapshot. Thus the
completeness claim is over the **fixed configured lane set**, not an arbitrary
set of independent cells discovered by an indexer. Positive appends are bounded
by eight messages per lane per epoch; an empty update cannot provide unbounded
valid churn.

## Resource measurements

Both CKB versions returned identical cycle estimates for these transaction bytes.
All 210 committed events have an estimate. Whole-transaction values, including
funding-lock verification, span **1,620,743–15,309,016 cycles**.

| Operation | Whole-transaction estimated cycles |
|---|---:|
| First complete seal, 1–4 lanes of short messages | 2,286,852–4,009,120 |
| Maximum 33,385-byte snapshot seal | 15,309,016 |
| Process each prefix of that maximum snapshot | 11,114,566–11,140,105 |

The program limits inputs, outputs, resolved dependencies and witnesses, bounds
witness bytes before allocation, and rejects an individual script exceeding its
20-million-cycle acceptance budget. These are mechanism limits and measured
samples, not sustained TPS or public-network fee/prover economics. Gas is enforced
by the local pinned EVM profile and has not been proved to a CKB settlement script.

Active lanes reserve enough capacity for maximum data and retain that capacity
across updates. Snapshots are permanently locked in this experiment. There is no
snapshot garbage collection, deposit release, withdrawal or proof-backed pending
obligation cleanup. Filling every lane deliberately demonstrates admission
backpressure; it does **not** show that a new honest user can obtain an admission
slot against sustained hostile competition.

## Reproduce and inspect

```bash
TACTUS_DEVNET_SUITE=replay-sealed CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-sealed TACTUS_CKB_VERSION=0.210.0 CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

The launcher creates a funded isolated dummy chain, uses two independent signing
accounts, pins source/binary/configuration hashes and stops only its own node.
Portable evidence is in [evidence/a3-sealed-gate](evidence/a3-sealed-gate/), with
[0.121.0 summary](evidence/a3-sealed-gate/ckb-0.121.0-summary.json),
[0.210.0 summary](evidence/a3-sealed-gate/ckb-0.210.0-summary.json), compressed full
transaction records, complete configurations, replay logs, manifests and
[SHA256SUMS](evidence/a3-sealed-gate/SHA256SUMS). Source inventories were compared
to the final implementation. The manifests intentionally record the pre-commit
base and then-untracked implementation files.

Sealed-program ELF SHA-256:
`11b9a15db893414950cf05dafe5565a330a19eba7293112f278b8261abc9f62a`.
Six committed events (one seal and one batch per lane count) are later orphaned;
210 is an event count, not final-canonical throughput. The three surviving
networks have 24, 24 and 32 canonical batches, respectively.

Local validation passed **81 Rust tests**, workspace formatting and Clippy,
RISC-V-target Clippy for the sealed program, and the actual RISC-V ELF build.
Adding the package changes the Cargo.lock-bound execution rules domain; all
9 Geth scenarios / 14 blocks were independently rerun, and the frozen result and
raw archive refreshed. CI includes the new node-version matrix workload; no
remote CI execution is claimed for these unpushed changes.

## Decision

Retain A3 as a candidate with **authenticated mandatory-publication evidence**.
The gate removes the builder's option to keep emitting unrelated batches forever
while ignoring an already admitted sealed obligation. It does not solve competing
admission, hostile public miner scheduling, real-time service bounds, unplanned
network reorgs, proof settlement or economically sustainable retention.

The next Experiment A work must compare admission contention, saturation and
carry-forward behavior under the same adversarial workloads as A1/A2, rather than
promoting this bounded scenario into a complete G2 pass. Production still requires
the complete execution → proof → verified settlement → exit path described in
[PRODUCTION_READINESS.md](PRODUCTION_READINESS.md).
