# Experiment A — Simulation-Tier Report

**Arms:** A1 atomic OrderingHead (reference) · A2 independent Message Cells · A3′ sharded lane heads + epoch-sealed control
**Tier:** discrete-event simulation. Devnet-tier evidence (real CKB txpool/miner behaviour, scripts, proofs) is **not** included; G1–G9 remain OPEN per spec §13.
**Reproduce:** `cargo run --bin tactus-o1-experiment-a` (deterministic seeds).

## A1 — fee-ratio × signing-delay (design §7.1)

| delay | fee× | succ% | p50 | p95 | DOA% | bch/blk | decision |
|---|---|---|---|---|---|---|---|
| 1 | ×1 | 71.8 | 7 | 19 | 7.1 | 0.17 | AdvanceToProductionReview |
| 1 | ×2 | 97.1 | 6 | 14 | 10.9 | 0.04 | AdvanceToProductionReview |
| 1 | ×10 | 97.1 | 6 | 14 | 10.9 | 0.04 | AdvanceToProductionReview |
| 2 | ×1 | 73.1 | 9 | 21 | 13.6 | 0.17 | AdvanceToProductionReview |
| 2 | ×2 | 97.1 | 7 | 15 | 19.1 | 0.05 | AdvanceToProductionReview |
| 2 | ×10 | 97.1 | 7 | 15 | 19.1 | 0.05 | AdvanceToProductionReview |
| 3 | ×1 | 75.7 | 11 | 21 | 24.1 | 0.17 | AdvanceToProductionReview |
| 3 | ×2 | 98.7 | 8 | 14 | 34.4 | 0.06 | KeepA1AsReferenceOnly |
| 3 | ×10 | 98.7 | 8 | 14 | 34.4 | 0.06 | KeepA1AsReferenceOnly |

**A1 verdict (worst observed, delay 3):** DOA 34.4% → `KeepA1AsReferenceOnly` — fee priority rescues conflicts it can reach, never stale OutPoints.

## A3′ — sharded lanes, churn and the sealed control (design §7.3, §14.6)

| scenario | K | survival | inval% | bch/blk | admDOA% | proc p95 | decision |
|---|---|---|---|---|---|---|---|
| no-churn | 8 | 0.98 | 2 | 0.247 | NaN | NaN | AdvanceToProductionReview |
| L1 per-lane ×4 (users ∝ K) | 8 | 0.12 | 88 | 0.037 | 8.9 | 77 | RejectLiveHeadDependencyStrategy |
| L1 per-lane ×1 (K=2) | 2 | 0.60 | 40 | 0.168 | 8.7 | 10 | AdvanceToProductionReview |
| L2 aggregate fixed (K=8) | 8 | 0.27 | 73 | 0.083 | 6.1 | 35 | RejectLiveHeadDependencyStrategy |
| L3 adversary 0.8, live refs | 8 | 0.01 | 99 | 0.002 | NaN | NaN | RejectLiveHeadDependencyStrategy |
| L3 adversary 0.8, sealed | 8 | 1.00 | 0 | 0.250 | NaN | NaN | ConditionalEnforcementPrimitiveUnimplemented |

**A3′ verdict:** live-head references collapse under adversarial churn (L3) and degrade with per-lane load (L1), while the aggregate-fixed regime (L2) stays comparable — matching the analytic model. The sealed control arm is churn-immune; its p95 processing delay was **not measured** in this run (the processing-delay path was never exercised), so the switching-policy limit is untested, not satisfied. Sealing cadence, sealing authority, post-seal mandatory latency and builder evasion of the next snapshot remain open. No reading of this table puts G2 near passing.

## A2 — independent Message Cells (design §7.2)

| builder | challenge | adm succ | viol% | proc p50 | proc p95 | forced frac | decision |
|---|---|---|---|---|---|---|---|
| honest FIFO | forces | 100.0 | 0.0 | 0 | 1 | n/a | ConditionalEnforcementPrimitiveUnimplemented |
| lazy, forced inclusion | forces | 100.0 | 98.6 | 21 | 21 | 1.00 | ConditionalEnforcementPrimitiveUnimplemented |
| lazy, penalty only | penalty | 100.0 | 98.6 | NaN | NaN | 0.00 | G2NotPassed |

**A2 verdict:** admission is contention-free by construction (100% at every configuration). Penalties without forced inclusion leave messages unprocessed (`G2NotPassed`). The forced-inclusion row passes **only by simulation assumption** — `challenge_forces_processing` presupposes the enforcement primitive (how a legal CKB challenge makes a refusing builder process) that no CKB lock/type script yet implements — so it is recorded as `ConditionalEnforcementPrimitiveUnimplemented`; and 98.6% of deadlines were violated before enforcement caught up, so even the simulated success is eventual, not within deadline. The devnet tier must implement, not assume, the enforcement path.

## Analytic churn sensitivity (design §3)

| lanes | L1 e^(-KλΔt) | L2 e^(-ΛΔt) |
|---|---|---|
| 1 | 0.4724 | 0.0000 |
| 2 | 0.2231 | 0.0000 |
| 4 | 0.0498 | 0.0000 |
| 8 | 0.0025 | 0.0000 |
| 16 | 0.0000 | 0.0000 |

L3 has no analytic form by construction; the event-level L3 arm above replaces it with measurements. Candidate registry: `A1AtomicHead` / `A2MessageCells` / `A3ShardedLanes` / sealed control.

---
_Decisions referenced: `KeepA1AsReferenceOnly` · `G2NotPassed` · `RejectLiveHeadDependencyStrategy` · `RejectSnapshotSwitchingPolicy` · `AdvanceToProductionReview` · `ConditionalEnforcementPrimitiveUnimplemented`._

## Starvation — what it is, why it happens, what the simulation found

**Definition.** Starvation is the failure of a legitimate participant to make progress through the priority path *while every safety property continues to hold*. It is a liveness failure, not a safety failure: nothing invalid is ever accepted — the valid thing simply never lands. The simulation isolated one distinct form per arm, each with a distinct cause.

**Form 1 — admission starvation (A1).** A wallet user reads head `H`, signs across a delay of *d* blocks, and broadcasts; meanwhile a professional builder has already consumed `H` and advanced to `H′`. The user's transaction is dead on arrival — invalid from the moment it enters the mempool, at any fee, because fee priority arbitrates only among simultaneously *valid* competing spends and never resurrects an already-invalid one. The cause is a structural asymmetry: builders are resident, re-submit instantly and pipeline successors, whilst wallet users traverse a signing round trip. The two-variable design separates these cleanly — fee ×2 and ×10 produce identical results at every delay, whereas increasing the signing delay from 1 to 3 blocks drives DOA from 7.1% to 34.4%. Retries eventually rescue headline success (98.7% at delay 3), but a third of all attempts die on arrival, which trips the worst-case rule: `KeepA1AsReferenceOnly`. **Cause: staleness, not price.**

**Form 2 — progression starvation (A3′, live-head variant).** Here the starved party is not the user's admission but canonical batch progression itself. Because a cell dependency must resolve to a live cell, every lane-head update invalidates each prepared anchor referencing the previous head — and exploiting this requires no control over any builder, only continuous valid enqueues. Under the L3 adversary, anchor survival collapses to 0.01 with 99% invalidation and canonical progression falls to 0.002 batches per block (`RejectLiveHeadDependencyStrategy`). The cause is the coupling of anchor validity to mutable live state that anyone may move; the sealed control arm removes exactly that coupling by referencing immutable sealed snapshots and is churn-immune — survival 1.00, progression 0.250 per block — though its processing-delay gate went unexercised and stays conditional. **Cause: live-state coupling; cure: immutable references.**

**Form 3 — processing starvation (A2).** Admission succeeds by construction (100% at every configuration) — and on its own feeds nobody. With a penalty-only challenge, 98.6% of deadlines are violated and **zero** per cent of messages are force-processed: admitted messages starve of processing while the guilty party pays for the privilege of ignoring them (`G2NotPassed`). The cause is retrospective liability without an enforcement path. The forced-inclusion challenge restores processing at a bounded delay (p50 = p95 = 21 blocks, forced fraction 1.00) — *in simulation, by assumption*: the enforcement primitive is unimplemented, so the row is recorded as `ConditionalEnforcementPrimitiveUnimplemented`. **Cause: liability without enforcement; the cure is an implemented primitive, not an assumed one.**

**The common lesson.** In every arm the safety machinery behaved exactly as specified — single-consumption linearisation never accepted a conflicting successor — and starvation occurred anyway. Safety is structural; liveness is an adversary-dependent property that must be purchased separately: A1 needs an admission path that does not race a resident professional counterparty; A3′ needs sealed, immutable references; A2 needs enforcement that exists in CKB scripts, not only in the simulator. No fee market fixes any of the three, because each failure mode is invalidity rather than priority — which is precisely why design §5 forbids substituting any average-throughput figure for G2.

## Interpretation boundaries

- Simulation models the protocol state machines, RFC 0020 proposal-window timing, fee-density conflict resolution and depth-1 reorgs only.
- Per design §5, no average-throughput figure substitutes for G2; decisions above are inputs to the devnet tier, not gate passes.
- Next tier: identical arms against a CKB devnet under a dominant-builder driver (design §8).

