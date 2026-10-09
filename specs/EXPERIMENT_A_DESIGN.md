# Experiment A — Priority Admission Comparison

**Status:** experiment design, pre-implementation
**Baseline:** [TACTUS_ARCHITECTURE_SPEC_v0.2.5.md](TACTUS_ARCHITECTURE_SPEC_v0.2.5.md) §14 (as amended by changes 20–24)
**Blocking gate:** **G2 — censorship resistance.** No average-TPS figure may substitute for it.

## 1. Purpose

Determine which priority-admission construction can, under identical CKB devnet
and adversarial conditions, simultaneously provide safety, a permissionless
route to *actual* processing, and acceptable cost. The experiment must be able
to reproduce each candidate's known failure mode:

- **A1 — atomic OrderingHead:** admission starvation (dead-on-arrival `ENQUEUE`
  under a dominant builder).
- **A2 — independent Priority Message Cells:** obligation failure (admission
  succeeds; forced processing does not follow).
- **A3′ — sharded lane heads:** dependency churn (verifiable freshness, but
  batch anchors invalidated by lane-head updates).

## 2. Arms

| Arm | Role |
|---|---|
| A1 atomic OrderingHead | correctness reference / oracle |
| A2 independent Message Cells | competitor |
| A3′ sharded lane heads | competitor |
| A3′ epoch-sealed snapshots | control arm (spec §5.4) |

## 3. Workload assumptions and load models

The survival sensitivity model `P_survival ≈ e^(−Δt·Σλᵢ)` from spec §5.4 is a
simplification. **Two load regimes must be distinguished and both reported:**

| Model | Assumption | Consequence |
|---|---|---|
| **L1 — fixed per-lane rate** | each lane updates at `λ`; lane count `K` grows | `P_survival ≈ e^(−KλΔt)` — more lanes *do* worsen invalidation |
| **L2 — fixed aggregate rate** | total rate `Λ = Σλᵢ` fixed, spread over more lanes | `P_survival ≈ e^(−ΛΔt)` — K-neutral in the simple model; more lanes still cost more `cell_deps`, verification and construction complexity |
| **L3 — adversarially concentrated** | attacker *chooses* update timing and lane targeting | independence assumption breaks; no analytic form — must be measured |

Conclusions about "more lanes ⇒ worse liveness" are valid **only under L1**.
L3 is mandatory regardless: a dependency-churn attacker does not obey Poisson
arrival, and the analytic models are sensitivity aids, not measured CKB
behaviour. CKB txpool and miner policies (fee-density ordering, proposal-window
handling) must be recorded per run; RFC 0022's live-`cell_deps` consensus rule
guarantees no particular pending transaction is ever packaged promptly.

## 4. Result categories (all three, per run)

- **Safety:** any duplicate consumption, mis-ordered processing, unauthenticated
  carry-forward, or unauthenticated execution obligation.
- **Liveness:** can a dominant builder or a malicious enqueuer block legitimate
  user admission, block actual processing of a priority message, or stall
  canonical batch progression?
- **Economics:** fees, cycles, rebuild costs and storage borne by users,
  builders and CKB nodes while the above guarantees hold.

## 5. Decision rules (pre-committed)

| Outcome | Decision |
|---|---|
| A1 safe, but wallets sustain DOA | A1 stays reference implementation only; not production admission |
| A2 admission succeeds, forced processing unproven | **G2 not passed** |
| A3′ freshness correct, churn blocks batches | reject current live-head reference strategy |
| Sealed snapshots reduce invalidation but introduce unbounded mandatory-inclusion delay | reject current snapshot switching policy |
| A candidate is safe, recoverable, force-progressing, affordable | advance to production protocol review |

## 6. Measurements

Spec §14.4 in full, including the churn additions: `lane_head_churn_rate_per_lane`,
`batch_dependency_invalidation_rate`, `candidate_anchor_survival_rate`,
`abandoned_anchor_rebuild_cost`. Dual-throughput requirement (§14.6):
**priority admission throughput** and **canonical batch progression throughput**
must be measured together; improving one while materially degrading the other
is not a success.

## 7. Test groups

1. **Fee-ratio vs DOA** (§14.2): same-live-head fee competition and
   stale-OutPoint races as independent variables.
2. **A2 adversarial cases** (§14.3): backlog saturation, carry-forward
   starvation (decisive), consumed-but-unproven.
3. **Cross-lane dependency churn** (§14.6): lane count × load model (L1/L2/L3) ×
   construction-to-commit interval, with the sealed-snapshot control arm.

## 8. Infrastructure requirements

Identical CKB devnet for all arms; dominant-builder driver; configurable wallet
signing delay; planned and unplanned reorganisation injection; checkpoint
reference tracking. A run is reproducible only if the devnet configuration,
txpool/miner policy snapshot, seeds and workload scripts are pinned.
