# Sustained active-lane and sealed-snapshot load

The new `replay-load` experiment measures repeated admission, queue saturation,
mandatory publication and stopped-arrival drain on the allocation-bound A3 gate.
It complements the A1/A2 admission races and authenticated A2 omission comparator;
it does not replace them or complete Experiment A/G2 by itself.

## Controlled workload

The complete matrix has **36 cases per CKB version**: lane counts 1/2/4, load
regimes L1/L2/L3, construction windows of 1/3 quanta, and two dependency policies.
Each case has a fresh genesis, eight empty warm-up batches and an empty seal,
followed by 32 consecutive construction windows. Warm-up is excluded from rates.

- L1 offers one message **per lane per quantum**. Offered aggregate demand grows
  with lane count.
- L2 offers **one aggregate message per quantum**, rotating lanes deterministically.
  Offered aggregate demand stays fixed as lane count changes.
- L3 offers the same aggregate count as L2, all to lane zero at the last quantum
  before submission. Retries also target that last quantum. This is an adversarial
  burst schedule, not a Poisson process or random independent arrival model.

A quantum is exactly four generated CKB blocks. An admission transaction appends
one message to each selected nonfull lane, using separately authenticated lane
scripts; empty remaining quantum blocks are still generated. Grouping independent
lane appends into one transaction keeps mining overhead per quantum constant
across lane counts. This models a cooperating admission publisher, not independent
wallet fee competition; the [matched admission races](ADMISSION_CONTENTION_REPORT.md)
cover that separate boundary. Batch/seal overhead is outside the offered-load
clock but included in elapsed CKB blocks and measured admission/churn rates.
Thus the labels describe **offered-load schedules**, not a claim that measured
on-chain update rates remain fixed after backpressure.

Actor 0 funds admissions; actor 1 independently funds batches and seals. Every
candidate is built and signed before its arrival window and must pass a real
`estimate_cycles` call at that point. The mandatory-sealed arm depends only on the
immutable authenticated snapshot. The diagnostic arm adds current lane heads as
live cell dependencies to the otherwise identical mandatory-gate construction.
This intentionally recreates a dependency strategy already rejected for production.
It is never a second supported admission protocol.

A failed candidate must produce CKB's exact resolution rejection (`-301`), while
its anchor, gate and funding inputs remain live. Its next attempt is rebuilt in
the next scheduled window. There is no hidden immediate quiescent repair inside
the measured window. A successful batch executes every required payload through
the serial EVM classifier and advances the same execution anchor.

Each offered message is a unique 64-byte malformed envelope. Every processed slot
must receive `Malformed`; no included Ethereum transaction, useful EVM TPS or
validity settlement is inferred from this workload. These small payloads isolate
queue/dependency effects. Valid signed deployment/storage execution remains
covered by the separate execution and network-reorg suites.

## Backpressure and accounting

Each lane retains at most eight active messages. An offered message encountering a
full lane stays in an explicit FIFO queue outside CKB; it has **not been admitted**.
The measured window records offered, admitted and processed counts, both queue
sizes, candidate survival, per-lane churn, fees, bytes, VM cycle estimates and host
construction time. Submitted transaction wire sizes are checked against CKB's
packed transaction response. Fees are recomputed from retained input/output
capacities by the independent analyzer.

After the 32 windows, arrivals stop. A separate bounded drain retries every remaining
offered message, continues mandatory seals and processes the entire finite backlog.
The driver rejects duplicate admissions, unadmitted/duplicate processing, changed
payloads and execution-anchor mismatches. Every admitted message must be processed
within 16 subsequent canonical batches. The report includes CKB-block latency as
well: a finite canonical-batch bound does not establish a wall-clock admission or
processing deadline while candidates stall.

Drain throughput is never folded into the active-window rates. Host seconds include
RPC and deliberate laboratory polling delays; generated CKB blocks are not a
mainnet clock. Neither rate is a production capacity claim. VM estimates for
abandoned candidates are **pre-churn estimate work**, not cycles proven spent by a
miner on a rejected transaction. Stale candidates pay zero chain fees; bandwidth,
construction and estimation work are still recorded. Retained capacity covers
new immutable batch data and snapshots in the active window, excluding the initial
lane reservation and deployment.

## Measured results

Both CKB 0.121.0 and 0.210.0 completed all 36 cases. Their independently derived
admission/publication counts, fees, retained capacity and canonical-batch delay
statistics agree. Three cases differ by one elapsed generated block, and some
admission-delay percentiles differ by up to three blocks; the retained tables
preserve those differences and the resulting per-block rates. Host timings also
vary.
Each version contains **1,152 active candidate windows**, 3,328 offered messages,
4,942 committed transaction records and 280 expected stale-candidate rejections.
Every offered message is eventually admitted and classified after the separately
measured stopped-arrival drain; none is silently dropped.

Across the 18 mandatory-sealed cases per version, **576/576 active batches commit**.
The 18 live-dependency diagnostic cases commit **296/576**, with 280 stale rejections.
The respective active-window admission/processing totals are 928/696 and 616/300
out of the same 1,664 offered messages per arm. These totals combine controlled
scenarios; they are not a single steady-state throughput measurement.

The three-quantum window results below are the same on both node versions. Each
sealed case spans 524 generated CKB blocks and commits all 32 candidates. Counts
exclude its subsequent drain. “Outside” means offered but not admitted to CKB;
“processed” means classified as malformed by the serial executor.

| Lanes | Load | Offered | Admitted | Processed | Outside | Diagnostic batches / 32 |
|---:|---|---:|---:|---:|---:|---:|
| 1 | L1 per lane | 96 | 32 | 24 | 64 | 23 |
| 1 | L2 fixed aggregate | 96 | 32 | 24 | 64 | 23 |
| 1 | L3 targeted | 96 | 32 | 24 | 64 | 16 |
| 2 | L1 per lane | 192 | 64 | 48 | 128 | 23 |
| 2 | L2 fixed aggregate | 96 | 64 | 48 | 32 | 20 |
| 2 | L3 targeted | 96 | 32 | 24 | 64 | 16 |
| 4 | L1 per lane | 384 | 128 | 96 | 256 | 23 |
| 4 | L2 fixed aggregate | 96 | 96 | 72 | 0 | 16 |
| 4 | L3 targeted | 96 | 32 | 24 | 64 | 16 |

The final column is the paired diagnostic arm's batch count; all other counts
refer to its mandatory-sealed counterpart. For fixed aggregate demand, four lanes
admit the entire offered workload in this finite window: admission is 96/524
messages per generated CKB block and batch progress is 32/524 batches per block.
Per-lane demand grows faster than service under L1, while concentrating all demand
on one lane under L3 leaves the additional lanes unused. Snapshot isolation
preserves batch construction, but it does not remove either admission bottleneck.

A saturated lane stops accepting updates. This can make a live-dependency
candidate survive more often even while the outside backlog grows. For example,
K=1/L1 diagnostic survival increases from 16/32 with one quantum to 23/32 with
three quanta, while its outside backlog rises from 16 to 72 messages. Candidate
survival alone would therefore give a misleading capacity assessment.

For K=4, sealed, three-quanta cases, admission-delay p50/p95/p99 are respectively
404/536/548 generated blocks under L1, 4/4/4 under L2 and 396/532/548 under L3.
These percentiles include messages admitted during the explicitly stopped-arrival
drain; they do not promise those latencies under indefinitely continuing load.
All cases satisfy the enforced maximum of 16 canonical batches between admission
and processing. That batch-count bound neither covers waiting outside CKB nor
establishes a proof or settlement deadline.

The full 72 independently reconciled rows, all latency samples, per-lane churn,
wire sizes, fee/capacity accounting and host timings remain in the archive. The
existing admission suite also passes a separate 0.210.0 regression (290 committed
records, 36 expected rejections) after extraction of the shared wire-size helper.

## Evidence and independent checks

The [retained archive](evidence/sustained-load) includes raw evidence, both node
configurations, replay/node logs, source and executable hashes, independently
derived tables and checksums. Run manifests describe the files present when each
run launched. The Python analyzer and its corruption tests were added during the
long runs; each derived table separately pins the analyzer hash. The launch-time
workflow and runner are retained separately: the later CI timeout increase from
45 to 90 minutes and registration of the completed seal-contention suite change
no executed local load code. No on-chain
program changes are involved in this experiment.

`scripts/analyze-load.py` decodes retained lane/batch wire bytes independently of
the Rust driver. It reconciles each message's admission/publication heights and
batch number, exact per-lane FIFO order, all 32 windows, actual fees, retained
capacity and offered/admitted/processed conservation. It requires the complete
36-case matrix. The retained-evidence tests reject fabricated throughput, missing
messages, wrong heights/batch numbers, hidden churn, changed fees/capacity,
omitted/duplicated scenarios and modified raw payload bytes. This is an evidence
consistency check, not an independent CKB consensus or validity verifier.

```bash
TACTUS_DEVNET_SUITE=replay-load CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
python3 scripts/analyze-load.py artifacts/load-RUN/evidence.json artifacts/load-RUN/analysis.json
python3 scripts/test-load-analysis.py
```

For compatibility, set `TACTUS_CKB_VERSION=0.210.0` and its matching binary. CI
runs both live load matrices and audits the retained evidence; local results do
not assert that remote CI ran before pushing.

The [separate fresh seal-contention experiment](SEAL_CONTENTION_REPORT.md) removes
this suite's favorable seal serialization and measures finite valid enqueue churn
against rebuilt seals. Its conditional result does not resolve admission fairness.

## Remaining qualification

This deterministic finite workload is not long-duration operational qualification,
a stochastic survival estimate, a hostile-miner test, proof-bound obligation
fulfillment or a network-fault experiment. It cannot pass G2/G4 alone. Full
Experiment A still needs miner/admission fairness qualification, proof-bound
pending processing and recovery, broader network faults and production economics.
