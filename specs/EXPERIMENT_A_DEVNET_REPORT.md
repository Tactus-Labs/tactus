# Experiment A — CKB mechanism-tier report

**Date:** 10 October 2026 (JST). **Status:** reproducible mechanism suite passed
on CKB 0.121.0 and 0.210.0. **Full Experiment A: incomplete. G1–G9: OPEN.
Production ready: no.** This report supersedes the earlier blocked A1 probe.

## Reproduce and inspect

```bash
CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_CKB_VERSION=0.210.0 CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

The launcher creates a fresh funded dummy chain, enables IntegrationTest only on
its loopback RPC, uses two different fee-paying SECP keys, generates blocks in a
controlled order and stops its own node. It does not alter existing chains.
Each generated block must reach both the canonical tip and the txpool before the
next block is requested. A failed control or unexpected RPC error exits nonzero.

Checked-in evidence is under [`evidence/experiment-a/`](evidence/experiment-a/):

- [`ckb-0.121.0-summary.json`](evidence/experiment-a/ckb-0.121.0-summary.json) and
  [`ckb-0.210.0-summary.json`](evidence/experiment-a/ckb-0.210.0-summary.json).
- Compressed raw transaction/RPC records for each version, with SHA-256 sums,
  compiler/code/source manifests, complete node configuration and chain spec.
- Decompress `ckb-VERSION-transactions.json.gz` to inspect signed transaction
  objects, transaction and block hashes, cycle estimates and exact rejections.

Both versions use the same deterministic workload. The genesis hash is
`0xddd6aea22911805968ca3f89bda08aa9d8024fc00b2c9ffb07b1bcd170673c6a`.
The node configuration, including txpool fee/RBF settings, is preserved rather
than inferred from observed outcomes. This is deterministic block generation,
not a claim about public miners, sustained TPS or probabilistic inclusion latency.

## A1 — real fee competition versus stale inputs

Two samples per matrix cell, 24 races total per node version. A builder broadcasts
first; a separately funded wallet signs against the same head, waits the stated
number of fully processed CKB blocks, then broadcasts. Fees are 1/2/10 CKB versus
the builder's 1 CKB. These are absolute test fees, not recommended production fees.
The same results were observed on both versions.

| Signing delay (blocks) | Wallet fee ratio | Wallet commits / 2 | Stale before broadcast / 2 | Pool rejects while head live / 2 |
|---:|---:|---:|---:|---:|
| 0 | 1× | 0 | 0 | 2 |
| 0 | 2× | 2 | 0 | 0 |
| 0 | 10× | 2 | 0 | 0 |
| 1 | 1× | 0 | 0 | 2 |
| 1 | 2× | 2 | 0 | 0 |
| 1 | 10× | 2 | 0 | 0 |
| 3 | 1× | 0 | 0 | 2 |
| 3 | 2× | 0 | 0 | 0 |
| 3 | 10× | 0 | 0 | 0 |
| 6 | 1× | 0 | 2 | 0 |
| 6 | 2× | 0 | 2 | 0 |
| 6 | 10× | 0 | 2 | 0 |

At delay 3, accepting the higher-fee transaction into the pool did not make it the
canonical winner under this proposal schedule. At delay 6, the consumed head is
invalid at every fee; the wallet's separate funding cell was checked to remain
live before broadcast. No conflicting pair both committed.

**Decision:** `KeepA1AsReferenceOnly`. This controlled schedule demonstrates the
failure mode, not a statistical estimate of real-wallet starvation probability.

## A2 — independent-cell omission baseline

Eight ordinary independent message cells were committed successfully. The builder
then committed eight batches over 32 generated blocks while ignoring every
message. All eight remained unconsumed; forced processing was zero.

**Decision:** `G2NotPassed`. These are independent-cell mechanics, **not** a
complete A2 Priority Message/Obligation Cell implementation. Authentication,
challenge enforcement, backlog limits and consumed-but-unproven settlement
recovery remain unimplemented and untested. A zero processed count here cannot be
presented as testing a protocol that does not yet exist.

The later [authenticated A2 comparator](A2_OBLIGATION_REPORT.md) adds actual
message locks, unique identities, mature challenges, bounded prefixes and pending
records. It replaces the above baseline's implementation gap with measured
individual-obligation controls and a stronger challenge-only counterexample.
Mandatory forced processing and proven settlement remain absent; the historical
ordinary-cell baseline is not retroactively presented as that implementation.

## A3 — live dependencies and immutable-copy controls

24 scenarios per version: 1/2/4 lanes × no churn / per-lane updates / one aggregate
update / targeted update × live references / immutable copies.

| Control | Scenarios | Anchors committed | Invalidated |
|---|---:|---:|---:|
| Live heads, no churn | 3 | 3 | 0 |
| Live heads, at least one referenced lane updated | 9 | 0 | 9 |
| Immutable copies, all schedules | 12 | 12 | 0 |

All lane updates were valid on-chain enqueues. A3 anchor fee funding stayed
independent of the updating actor. Stale dependencies were rejected by CKB cell
resolution. Thus adding fees cannot make a spent dependency live again.

**Decisions:** reject the live-head dependency strategy under this adversarial
schedule; immutable controls remain
`ConditionalEnforcementPrimitiveUnimplemented`. Copies lack enforced provenance,
epoch sealing and mandatory snapshot switching. Processing delay is **unmeasured**.
The per-lane/aggregate rows are controlled update schedules, not Poisson-rate or
throughput measurements. L2 and L3 each update one referenced lane in this fixture;
their identical outcome is not independent evidence about attacker timing.

## Script and recovery controls

Eleven freshly signed invalid cases were rejected by the expected OrderingHead
**type script**, rather than an unrelated lock or malformed transaction:

- Burn, split and removal of the head type.
- Lock takeover and state-capacity drain.
- Genesis-bound policy mutation, cursor beyond tail, skipped batch number and
  wrong accumulator.
- Missing commitment witness and re-creation of an existing identity from another
  genesis seed.

A second independent key successfully enqueued against a head created by the
first. Valid successors still committed after the rejection cases. Host tests
also cover counter overflow, invalid prior cursor state and canonical encoding.

The planned reorg control committed one branch, truncated four blocks, recovered
the prior head from canonical CKB blocks, committed an alternative branch through
the other actor, then independently recovered that successor. No operator snapshot
or indexer was used by the recovery function. This is ordering-cell recovery on a
local planned reorg; network-driven reorgs, EVM state and proving inputs are not
covered.

Each run has **198 committed-event records and 32 rejection records**. One committed
transaction is subsequently orphaned deliberately; these counts are event counts,
not unique final-canonical throughput. There are 174 `estimate_cycles` samples,
ranging from **1,615,028 to 1,732,770 cycles**, including funding-lock verification.
They are RPC estimates, not sustained performance or proving-cost measurements.
Fees, occupied storage, rebuild costs and prover economics still need the full
Experiment A accounting and deployment thresholds.

## Corrections to the previous blocked replay

The previous note reported SECP errors on a pre-existing 0.210.0 chain and
attributed them to a chain-wide node-version regression. **That general attribution
is withdrawn:** all signatures and this full mechanism suite pass on a fresh
0.210.0 chain. The exact cause in that earlier chain has not been established.

The repaired driver consumes the funding change output rather than the deployed
code output, preserves a separate fee balance, identifies the actual SECP dep
group by its members, signs the correct lock group, and uses `data1` for the Rust
ELFs. The linked ELF reproduced `MemOutOfBound` under VM v0 and ran under VM v1.
None of those observations alone proves which fault caused every historical
SECP error.

## Remaining completion criteria

Full Experiment A still requires protocol-enforced A2 obligations and A3 sealing,
authenticated processing/proof binding, saturation and carry-forward workloads,
consumed-but-unproven recovery, unplanned reorgs, quantitative resource accounting,
and realistic miner policies. Production also requires the execution, DA, validity,
bridge and operational paths listed in [PRODUCTION_READINESS.md](PRODUCTION_READINESS.md).
