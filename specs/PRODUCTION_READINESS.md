# Production readiness

Status: **NOT READY**, 10 October 2026. G1–G9 remain OPEN. Passing the mechanism
suite is necessary implementation evidence; it does not authorize a production
rollup deployment or custody of user assets.

## Implemented and measured

- Deterministic A1/A2/A3 simulations and conditional decisions.
- CKB-VM OrderingHead creation and succession, singleton identity, checked
  counters, fixed capacity, immutable deployment code and a permissionless lock.
- Two independent fee-paying actors, real txpool competition, stale input and
  dependency rejection, and script-specific negative tests.
- An isolated, funded devnet launcher, raw transaction evidence, code/config/source
  hashes, bounded RPC waits and a CI mechanism job.
- Canonical OrderingHead reconstruction from CKB blocks after a planned local
  reorg. No operator snapshot or indexer is used by that reconstruction.

See [devnet results](EXPERIMENT_A_DEVNET_REPORT.md) and the
[fixture wire format and limitations](DEVNET_WIRE_FORMAT.md).

- Canonical bounded multi-block input encoding, tested against independent Python
  vectors, plus a distinct CKB-VM anchor enforcing atomic immutable data publication.
- Full batch-input recovery from canonical CKB blocks, including a maximum-size
  publication and replacement of an orphaned branch. This is input recovery,
  not recovery of executed EVM state. See [batch results](BATCH_INPUT_REPORT.md).

- Pinned Shanghai serial EVM execution with signed Ethereum envelopes, total input
  rejection outcomes, real account/storage/transaction/receipt tries and block
  hashes. Empty blocks, fee accounting and in-memory atomic replay are tested.
  See [execution rules and limits](EXECUTION_V1.md). This is not proof-backed
  settlement.
- Independent Geth 1.17.8 comparison: 9 scenarios, 14 blocks, matching Ethereum
  roots, gas, logs bloom and rejection indices. Frozen independent results are
  enforced by Rust tests; see [comparison evidence](EXECUTION_DIFFERENTIAL_REPORT.md).

- Durable append-only batch journal with exclusive writer locking, fsync publication,
  corruption checks and EVM replay on restart. Separate-process CLI restart and
  interrupted-write layouts are tested; see [journal contract](EXECUTION_JOURNAL.md).

- Real CKB input publications now reconstruct executed EVM state in independent
  processes across a planned rollback/replacement on both CKB versions. Ordinary
  tip growth is accepted; wrong chain/type/genesis and replay-time reorgs are
  rejected. See [recovery evidence and limits](CKB_EVM_RECOVERY_REPORT.md).

- Authentic individual A2 message/lock state machine with consensus-mature
  challenges, bounded input-ordered publication and immutable pending records.
  The [real-node comparator](A2_OBLIGATION_REPORT.md) demonstrates that standalone
  challenge markers still fail forced inclusion; G2 remains OPEN.

- The [bounded A3 sealed schedule](A3_SEALED_V1.md) now has a mandatory CKB gate,
  authentic all-lane seals, retained snapshots and FIFO prefix enforcement.
  [Real-node evidence](A3_SEALED_REPORT.md) measures hostile switching, active-lane
  churn, full queues, maximum snapshots and planned seal/batch rollback. This
  establishes publication duties under batch progress, not wall-clock liveness,
  admission fairness or proof settlement.

- [Matched authenticated admission races](ADMISSION_CONTENTION_REPORT.md) compare
  five arms on both CKB versions, with independent fee cells and identical delays
  and absolute fees. A3 targeted same-lane conflicts retain A1's failure modes;
  independent A2 and disjoint A3 admissions succeed in this bounded schedule.

- [Independent A3 cold recovery](SEALED_RECOVERY_REPORT.md) reconstructs the
  Schedule, lane queues and current snapshot from canonical blocks in fresh
  processes. Recovered state drives replacement sealing after a planned rollback;
  wrong domains and orphan pins fail. This does not recover validity settlement.

- A [two-node P2P partition/reorg experiment](NETWORK_REORG_REPORT.md) now replaces
  eight original canonical blocks, recovers a peer-only A3 admission, rolls back
  executed contract state and resumes mandatory processing on both CKB versions.
  It uses real peer synchronization, not `truncate`; broader network qualification
  and validity settlement remain outstanding.

- [Canonical genesis publication](GENESIS_ALLOCATION_REPORT.md) now binds the
  allocation in the anchor type and retains its exact bytes in an immutable output.
  Independent recovery derives genesis from CKB and rejects different caller
  allocations. The updated domain passes the Geth comparison and both CKB suites;
  CKB-enforced proof-bound state-root derivation and production supply policy remain open.

- [Sustained lane load](SUSTAINED_LOAD_REPORT.md) now measures repeated queue
  saturation, dual admission/batch progress and stopped-arrival drain under
  deterministic L1/L2/L3 schedules. This exposes backpressure and dependency
  failure without claiming useful Ethereum TPS or seal-submission fairness.
  The [Experiment A acceptance audit](EXPERIMENT_A_ACCEPTANCE_MATRIX.md) preserves
  remaining miner-policy, admission-fairness and proof-fulfillment requirements.

- [Execution proof guest](EXECUTION_PROOF_V1.md) now executes a real three-transfer
  batch inside SP1 and matches the native/Geth roots. Its canonical public journal
  binds deployment context, allocation, interval history and both state roots.
  This guest-execution evidence does not establish proof-accepted CKB settlement,
  production proving cost, trusted setup/key provenance or withdrawal authority.

- [First real execution proof](EXECUTION_CORE_PROOF_REPORT.md) now proves that
  batch with a local SP1 core STARK, passes 15 public-value/key rejection controls
  and verifies in a fresh process. The proof and measured workstation cost are
  retained. Compression, authenticated CKB verification and settlement succession
  remain absent; this does not close G3 or qualify production proving economics.

- [Transition-authenticated history checkpoints](HISTORY_CHECKPOINT_V1.md) now
  retain exact Anchor successor states in immutable typed cells. Both CKB versions
  reject 22 negative controls and allow later ordering to reference old checkpoints
  while rejecting a spent mutable Anchor dependency. This supplies a bounded
  history mechanism. A [real P2P checkpoint reorg](CHECKPOINT_REORG_REPORT.md) now
  rejects an orphaned checkpoint dependency and accepts its canonical replacement
  on both versions; proof-consuming SettlementTip and unplanned-fault qualification
  are still absent. The [Groth16 receipt harness](PROOF_RECEIPT_V1.md) is implemented,
  with its first real compressed proof still pending at this milestone.

- [Repeated fresh seal contention](SEAL_CONTENTION_REPORT.md) now attains the
  finite `8 × lane_count` valid-append budget, rejects further full-queue churn,
  seals the complete set and processes every payload across two consecutive
  epochs on both versions. This is conditional canonical progress, not a miner
  inclusion promise or protection against a user losing every admission race.

## Work that still blocks the user's production objective

| Priority | Boundary | Required completion evidence |
|---|---|---|
| P0 | Full Experiment A / G2 | Add mandatory forced processing beyond the measured challenge-only failure; qualify independent-wallet load and hostile inclusion policy beyond the measured finite seal-churn budget, sustained queue/dependency workload and targeted-lane failure, proof-bound pending-record recovery, hostile miner policies and unplanned reorgs. Individual A2 authenticity, bounded overload/drain and planned-reorg carry-forward now have measurements. |
| P0 | Batch admission / W-12 | The implemented input encoding/inline-DA bounds have optional authenticated A2 input prefixes and an A3 mandatory sealed-set gate, but still need the production enforcement selection, L1 timestamp bounds, on-chain/proved binding to the new Ethereum envelope and deterministic rejection semantics, plus proved execution resource limits. |
| P0 | Execution / G5 | Bind published genesis and the implemented execution profile into a verified state transition; add state checkpoints, broader differential conformance and ordinary Ethereum tool deployment; replace experimental genesis/supply limits. |
| P0 | Validity settlement / G3 | Compress the measured local core proof into the final proof form; implement and qualify the CKB verifier, authenticated history and SettlementTip succession with wrong-state, wrong-order, wrong-domain and wrong-key rejection. The first local full-batch proof does not establish CKB-accepted settlement. |
| P0 | Recovery / G6 | Extend allocation-bound CKB-to-EVM reconstruction to proving inputs and settlement by another prover, with unplanned network reorgs. |
| P0 | Bridge and exits / G7 | Deposit/withdrawal conservation, replay resistance and operator-independent exit evidence; no release on experimental cursors. |
| P1 | Operations / G9 | Qualify the local journal under hardware/long-run faults; add network-driven reorg handling, monitored archival retrieval, independent operators and long-duration fault injection. |
| P1 | Governance / G8 | Enforced upgrade boundaries and exit-preserving rules. |
| P1 | Performance / G4 | Published deployment thresholds and sustained workload measurements including DA and proving cost. |

## Next implementation order

First close Experiment A's enforcement gap and define the production batch
admission envelope. Keep A1 as a correctness reference and reject live mutable
A3 dependencies for canonical anchors. Treat A2 obligations and authenticated
sealed snapshots as candidate constructions with bounded mechanism evidence,
requiring admission fairness, proof-bound fulfillment and hostile-network
qualification before selection as a production mechanism.

Then implement one complete path: Ethereum transaction → deterministic execution
→ CKB-published data and anchor → validity proof → verified settlement → withdrawal.
Only extend throughput, fast confirmations or external DA after this path is
recoverable without its original operator. No stand-in proof, simulated inclusion
or hash-only publication may be used to pass these gates.

## Local validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked --bin tactus-o1-experiment-a
CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
```

The launcher defaults to reference CKB 0.121.0 and isolated RPC/P2P ports
18714/18715. `TACTUS_DEVNET_RPC_PORT` and `TACTUS_DEVNET_P2P_PORT` select other ports.
`TACTUS_CKB_VERSION=0.210.0` selects the explicit compatibility run. The selected
binary must match the selected version. Each run creates a fresh directory under
`artifacts/a123-*`; configuration, manifest, logs, evidence and summary are kept
there, and its own node is stopped on exit. Existing nodes are never stopped or
reconfigured. The fixed keys are for these funded dummy chains only.

CI also runs the reference devnet suite and uploads evidence on failure. Its
presence in the workflow is not a claim that remote CI has run on unpushed changes.

The input-publication suite can be run independently with
`TACTUS_DEVNET_SUITE=replay-batch CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh`.
Its code and wire format are separate from the A1 bare-commitment reference so the
latter cannot serve as an accidental fallback for the input-publication anchor.

The authenticated A2 comparator runs with
`TACTUS_DEVNET_SUITE=replay-priority CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh`.
A completed comparator records `forced_inclusion: FAILED`: this is the expected
adversarial finding, never a G2 pass. Its rules and scope are specified in
[A2_OBLIGATION_V1.md](A2_OBLIGATION_V1.md).
