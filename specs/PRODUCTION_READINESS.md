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

## Work that still blocks the user's production objective

| Priority | Boundary | Required completion evidence |
|---|---|---|
| P0 | Full Experiment A / G2 | Add mandatory forced processing beyond the measured challenge-only failure; finish authenticated A3 sealing/switching, proof-bound pending-record recovery, hostile miner policies and unplanned reorgs. Individual A2 authenticity, bounded overload/drain and planned-reorg carry-forward now have measurements. |
| P0 | Batch admission / W-12 | The implemented input encoding/inline-DA bounds have optional authenticated A2 input prefixes, but still need mandatory priority-set enforcement, L1 timestamp bounds, on-chain/proved binding to the new Ethereum envelope and deterministic rejection semantics, plus proved execution resource limits. |
| P0 | Execution / G5 | Authenticate the implemented pinned execution profile; add state checkpoints, broader differential conformance and ordinary Ethereum tool deployment; replace experimental genesis/supply limits. |
| P0 | Validity settlement / G3 | Real prover and CKB verifier with wrong-state, wrong-order, wrong-domain and wrong-key rejection, plus a full proven batch. |
| P0 | Recovery / G6 | Authenticate genesis allocation; extend measured CKB-to-EVM reconstruction to proving inputs and settlement by another prover, with unplanned network reorgs. |
| P0 | Bridge and exits / G7 | Deposit/withdrawal conservation, replay resistance and operator-independent exit evidence; no release on experimental cursors. |
| P1 | Operations / G9 | Qualify the local journal under hardware/long-run faults; add network-driven reorg handling, monitored archival retrieval, independent operators and long-duration fault injection. |
| P1 | Governance / G8 | Enforced upgrade boundaries and exit-preserving rules. |
| P1 | Performance / G4 | Published deployment thresholds and sustained workload measurements including DA and proving cost. |

## Next implementation order

First close Experiment A's enforcement gap and define the production batch
admission envelope. Keep A1 as a correctness reference and reject live mutable
A3 dependencies for canonical anchors. Treat A2 obligations and authenticated
sealed snapshots as candidate constructions requiring implementation and hostile
execution, not approved alternatives.

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
