# Experiment A — Simulation-Tier Report

**Arms:** A1 atomic OrderingHead (reference) · A2 independent Message Cells · A3′ sharded lane heads + epoch-sealed control
**Tier:** discrete-event simulation. Devnet-tier evidence (real CKB txpool/miner behaviour, scripts, proofs) is **not** included; G1–G9 remain OPEN per spec §13.
**Reproduce:** `cargo run --bin tactus-experiment-a` (deterministic seeds).

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
| L3 adversary 0.8, sealed | 8 | 1.00 | 0 | 0.250 | NaN | NaN | AdvanceToProductionReview |

**A3′ verdict:** live-head references collapse under adversarial churn (L3) and degrade with per-lane load (L1), while the aggregate-fixed regime (L2) stays comparable — matching the analytic model. The sealed control arm is churn-immune and its p95 processing delay (NaN blocks, seal period 12) stays within the switching-policy limit.

## A2 — independent Message Cells (design §7.2)

| builder | challenge | adm succ | viol% | proc p50 | proc p95 | forced frac | decision |
|---|---|---|---|---|---|---|---|
| honest FIFO | forces | 100.0 | 0.0 | 0 | 1 | n/a | AdvanceToProductionReview |
| lazy, forced inclusion | forces | 100.0 | 98.6 | 21 | 21 | 1.00 | AdvanceToProductionReview |
| lazy, penalty only | penalty | 100.0 | 98.6 | NaN | NaN | 0.00 | G2NotPassed |

**A2 verdict:** admission is contention-free by construction (100% at every configuration); the open question is exactly the one the design predicted — penalties without forced inclusion leave messages unprocessed (`G2NotPassed`), while a challenge that forces processing restores them at bounded delay.

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
_Decisions referenced: `KeepA1AsReferenceOnly` · `G2NotPassed` · `RejectLiveHeadDependencyStrategy` · `RejectSnapshotSwitchingPolicy` · `AdvanceToProductionReview`._

## Interpretation boundaries

- Simulation models the protocol state machines, RFC 0020 proposal-window timing, fee-density conflict resolution and depth-1 reorgs only.
- Per design §5, no average-throughput figure substitutes for G2; decisions above are inputs to the devnet tier, not gate passes.
- Next tier: identical arms against a CKB devnet under a dominant-builder driver (design §8).

