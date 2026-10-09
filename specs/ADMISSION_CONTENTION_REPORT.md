# Matched A1/A2/A3 admission contention

Both **CKB 0.121.0 and 0.210.0** complete **120 admission races each** with
identical classifications. A2 independent messages and A3 writes to different
lanes admit both actors in every tested race. A targeted A3 lane has the same
admission conflict outcomes as the A1 shared head. **Adding lanes alone does not
remove targeted admission starvation. Full Experiment A, G2 and production
readiness remain OPEN.**

## Method

`replay-admission` compares five arms, each with delays of 0/1/3/6 generated CKB
blocks, delayed-wallet fees of 1/2/10 times the first actor's fee, and two repeats
per combination. These are deterministic controls, not independent random trials
or an estimate of a population success probability.

Every race starts from a newly committed head/network and two independent live
fee cells. Both transactions are signed against that initial state. Actor 0
broadcasts an admission first at 1 CKB fee; after the selected delay actor 1
broadcasts its already signed admission. Both use distinct, fixed 64-byte payloads
that are identical across arms. Before the delayed broadcast, the driver records
whether its protocol input and fee input are still live. Candidate cycle estimates
must succeed before either broadcast. The driver waits at most 20 further blocks
for a canonical winner, or both canonical transactions for independent inputs.

This is **admission versus admission**, complementing the older A1
[builder batch versus wallet enqueue experiment](EXPERIMENT_A_DEVNET_REPORT.md).
A1 publishes a payload commitment; A2/A3 store the actual bytes. A2 uses the
real authenticated Message type/lock, and A3 uses the real authenticated lane
program and complete gate genesis. The four-lane targeted arm writes both
messages to lane 0; the disjoint arm writes to lanes 0 and 1. The latter is a
favorable routing control, not an adversary that promises to avoid the victim.

All arms share the same deployed code dependencies and node configuration.
Absolute fees and payload sizes match; transaction sizes, cycle costs, retained
capacity and guarantees do not. This is not an equal fee-density comparison.
Fresh setup transactions and code deployment are excluded from admission counts.

## Outcomes per version

| Arm | Races | Victim committed | Both committed | Live pool rejection | Accepted but lost | Stale protocol input |
|---|---:|---:|---:|---:|---:|---:|
| A1 shared head | 24 | 8 | 0 | 6 | 4 | 6 |
| A2 independent messages | 24 | 24 | 24 | 0 | 0 | 0 |
| A3 one lane | 24 | 8 | 0 | 6 | 4 | 6 |
| A3 four lanes, targeted | 24 | 8 | 0 | 6 | 4 | 6 |
| A3 four lanes, disjoint | 24 | 24 | 24 | 0 | 0 | 0 |

For each of the three conflicting arms, victim outcomes per two repeats are:

| Delay in CKB blocks | Fee 1× | Fee 2× | Fee 10× |
|---:|---|---|---|
| 0 | 0/2; live pool rejection | 2/2 committed | 2/2 committed |
| 1 | 0/2; live pool rejection | 2/2 committed | 2/2 committed |
| 3 | 0/2; live pool rejection | 0/2; accepted but lost | 0/2; accepted but lost |
| 6 | 0/2; stale input | 0/2; stale input | 0/2; stale input |

A live input does not imply a successful replacement: at delay 3 a higher-fee
candidate can be accepted while the earlier transaction still wins canonical
inclusion. At delay 6 the protocol input is already spent and even the 10× fee
cannot repair it. The victim's independent funding remains live at submission
in every case. An accepted candidate is never counted as a canonical admission.

Each run records **168 canonical admissions**, **120 setup transactions**,
**2 deployment transactions**, and **36 submission rejections**. There are 240
candidate cycle estimates, including losers. No conflicting pair commits twice;
all independent pairs commit both transactions.

## Cost observations

| Arm | Admission wire bytes | Estimated cycles per candidate |
|---|---:|---:|
| A1 shared head | 1,034 | 1,671,383–1,732,715 |
| A2 independent messages | 999 | 1,681,579–1,741,740 |
| A3 one lane | 985 | 1,784,915–1,830,724 |
| A3 four lanes, targeted | 985 | 1,763,400–1,821,069 |
| A3 four lanes, disjoint | 985 | 1,763,400–1,834,479 |

Wire size includes witnesses and is cross-checked against the node's packed
transaction for every canonical admission. It is not cycle-weighted virtual
size. Each candidate and its full outputs, exact fee, estimate and canonical
transaction/block identity are retained in the evidence.

The new A2 message reserves 396 CKB in this 64-byte case. A3 updates an existing
8,472 CKB lane reservation, sized for its maximum queue, while A1 updates an
existing 336 CKB head. These reservations and genesis overheads are different
capital costs; an A3 update is not a new independent message cell. This run does
not estimate economic security, amortized seal/proof costs or release capital
from already included, unproven obligations.

## Interpretation and remaining work

Independent A2 admission still does not force subsequent processing, as shown by
the [challenge-only counterexample](A2_OBLIGATION_REPORT.md). A3's mandatory sealed
prefix binds publication **after admission**, as shown by the
[sealed-gate experiment](A3_SEALED_REPORT.md); this matrix exposes its remaining
same-lane admission contention. Neither favorable routing nor a higher fee closes
that gap under a targeted attacker.

This matrix uses fresh, empty queues and deterministic Dummy-node block generation.
It does not implement sustained fixed aggregate/per-lane load, saturation fairness,
selective miner censorship, network propagation, unplanned network reorgs, execution
proofs or settlement. The separate sealed suite covers bounded full queues and
planned rollback, which cannot substitute for those missing tests. The production
selection must still address admission liveness and proof-bound fulfillment.

## Reproduction and retained evidence

```bash
TACTUS_DEVNET_SUITE=replay-admission \
  CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_CKB_VERSION=0.210.0 TACTUS_DEVNET_SUITE=replay-admission \
  CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

The launcher preserves raw evidence, source/program/config hashes, logs and a
summary. [Archived evidence](evidence/admission-contention) contains compressed
raw evidence and launcher summaries, a compact comparison, configuration,
manifests and checksums. Manifests name the preceding commit plus exact source
hashes because changes were tested before committing. CI now runs this matrix on
both CKB versions; remote CI execution is not claimed for these local changes.

Extracting the shared lane construction helpers also passed the existing A3
sealed-gate suite: 210 committed events and 69 expected negative cases on CKB
0.121.0. Workspace tests, Rustfmt and strict workspace Clippy pass. Protocol
wire formats, script ELFs and Cargo.lock are unchanged by this milestone.
