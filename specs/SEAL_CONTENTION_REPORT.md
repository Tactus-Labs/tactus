# Fresh seal candidates under repeated enqueue contention

The authenticated A3 gate now has real-node evidence for repeated invalidation of
freshly rebuilt seal candidates. On both CKB 0.121.0 and 0.210.0, every tested epoch
ends with a complete authenticated snapshot and classification of every admitted
message after finite legal enqueue churn. This qualifies one important switching
boundary; it does not establish user admission fairness or a miner inclusion
promise. G2 and production readiness remain OPEN.

## Measured construction

Each case has a fresh allocation-bound deployment and independent funding for the
enqueuer and sealer. After eight warm-up batches make the first seal legal, the
sealer constructs and signs a valid seal. A real VM cycle estimate must succeed
before any attack. The enqueuer then commits a valid append that consumes one or
more of the seal's lane inputs. Broadcasting the previously signed seal must fail
with the exact CKB resolution error `-301`; its Schedule and funding inputs remain
live. The sealer rebuilds against the new canonical lane states and the attack
repeats. This is deliberate stale-before-broadcast contention, not simultaneous
fee replacement or a hostile miner implementation.

The one-lane-per-rebuild attacker rotates its next target across all configured
lanes. The all-lanes attacker appends once to every lane in a single authenticated
transaction. Payloads are distinct 64-byte malformed envelopes carrying epoch,
lane and position; all must receive deterministic EVM `Malformed` outcomes after
mandatory publication. They are not useful Ethereum transaction throughput.

When all queues become full, three further attacks are attempted against **every
lane**: a ninth append, an unchanged-head transition and an epoch reset without
Schedule authorization. These must fail with script errors 3, 12 and 7. The rebuilt
seal then commits, every active queue resets through that authorized seal, and
eight canonical batches drain the complete frozen set. The entire attack repeats
in a second consecutive epoch, preserving each lane's cumulative history.

## Results per version

Both versions have the same results. Counts below are per epoch, with two epochs
per row:

| Lanes | Enqueue strategy | Legal messages | Invalidated fresh seals | Extra invalid churn rejections | Final seal delay, CKB blocks | Maximum processing delay, canonical batches |
|---:|---|---:|---:|---:|---:|---:|
| 1 | One lane per rebuild | 8 | 8 | 3 | 4 | 2 |
| 1 | All lanes per rebuild | 8 | 8 | 3 | 4 | 2 |
| 2 | One lane per rebuild | 16 | 16 | 6 | 4 | 4 |
| 2 | All lanes per rebuild | 16 | 8 | 6 | 4 | 4 |
| 4 | One lane per rebuild | 32 | 32 | 12 | 4 | 8 |
| 4 | All lanes per rebuild | 32 | 8 | 12 | 4 | 8 |

Each run records **324 committed transaction events**, **244 expected rejections**,
**160 invalidated fresh seals**, **84 rejected attempts at further full-queue
churn**, **12 successful seals** and **224 distinct admitted/processed messages**.
No offered message in this construction is dropped or processed twice. Every
snapshot contains exactly the authenticated input lanes; processing order is
round-robin across their FIFO queues. Reported four-block seal inclusion is the
observed Dummy-node proposal schedule, never a public-chain deadline.

## Conditional bound and its limits

On one canonical history within an epoch, let `R` be the sum of unused active
queue slots. An independent legal lane append strictly decreases `R`; no-op and
unauthorized reset transitions cannot replenish it. Initially `R ≤ 8K`, where
`K ≤ 4`. Consequently, at most `8K` individually committed legal appends can
invalidate successive seal candidates before active enqueue churn exhausts.
An atomic append to several lanes spends several of those slots at once. The
maximum-count rotating attack and the grouped attack both attain their respective
measured bounds.

This is a protocol argument about a finite canonical mutation budget, supported
by the retained attacks. It is conditional on a builder reconstructing current
state, having funding and witnesses, and eventual CKB inclusion once conflicting
valid appends stop. It does not bound wall time, CPU/network denial of service or
reorgs that restore earlier states. An authorized competing seal advances the
protocol; it is not a successful continuation of enqueue-only churn.

Most importantly, queue exhaustion stabilizes seal inputs **by refusing further
admission**. An attacker can still win admission slots ahead of a particular user.
The [matched admission evidence](ADMISSION_CONTENTION_REPORT.md) demonstrates that
separate weakness; neither this mutation bound nor a successful stopped-arrival
drain resolves it. Continuous overload qualification remains a separate measurement: chain-external
waiting must remain distinct from admitted obligations.

## Reproduction and evidence

```bash
TACTUS_DEVNET_SUITE=replay-seal-contention CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
python3 scripts/analyze-seal-contention.py artifacts/seal-contention-RUN/evidence.json artifacts/seal-contention-RUN/analysis.json
```

Set `TACTUS_CKB_VERSION=0.210.0` and its binary for the compatibility run.
[Retained evidence](evidence/seal-contention) includes raw transactions, summaries,
node configurations, manifests, candidate estimates, replay logs, independent
analyses and checksums. Both local runs used the retained temporary launcher while
the long load matrices were in flight. Its runtime is identical to the registered
launcher; the only extra operation records its own file hash in the manifest.
The final registration adds the suite name to the allowed list and CI. No remote
CI result is claimed for these unpushed changes.

The independent Python analyzer checks actual stale input overlap, continued
independent funding, append bytes, every snapshot's complete input set, authorized
queue reset and exact mandatory payload order. It is an evidence consistency
checker, not an independent consensus or validity verifier. CKB programs and the
execution rules are unchanged by these experiments. Validity settlement, pending
record fulfillment and exit rights remain outside these host replay results.
