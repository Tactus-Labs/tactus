# Changelog

## 0.1.2 — 2026-10-09

- Research note: DAG acceleration and external data availability
  (`specs/DAG_ACCELERATION_NOTE.md`) — three-tier decomposition of DAG-isation
  (engine parallelism permitted, batch-construction DAG linearised into
  manifests already specified, canonical-ordering DAG rejected on O(1)-anchor
  arithmetic), prior-art borrowability (Monad/Block-STM, MegaETH engine-not-
  trust-model, Sonic protocol-irrelevant), coexistence with the O2 domain, and
  deployment timing — day-0 interface headroom (manifest multi-microbatch
  schema, determinism harness); both permitted tiers activate without a fork;
  StarkEx-style volition recorded as the counter-case — day 0 reserves the
  domain boundary, never the dual-tree stitching (§5.5).
- DAG note amended after external review: §2's "non-binding ordering" weakened
  to a measured-expectation (TPS_settled ≤ min(T_DA, T_execution, T_proving,
  f_anchor·N_batch_max)); corrected the false premise that multi-microbatch
  manifests are in the frozen baseline (they are not — the reservation question
  is decided by the L2 block model); scoped the churn argument to live-head
  constructions; added §1.4 — the prior question of anchor↔EVM-block
  cardinality (recommended: multiple strictly ordered EVM blocks per anchor,
  to be specified before genesis).
- DAG note §8 decision record: Day 0 is correctness-first (serial revm, linear
  ordering, validity proofs, priority inbox, CKB DA, basic bridge/exits); no
  execution DAG, no microbatch DAG, canonical DAG rejected; multi-block-per-
  anchor semantics must be implemented and tested pre-Day-0, never schema-only;
  phased plan Day 0 / Phase 1 (optimisation) / Phase 2 (throughput extensions);
  freeze early: EVM block semantics, batch commitment format, proof binding
  rules.
- Experiment A report: added a starvation synthesis section — definition,
  three observed forms (admission / progression / processing starvation) with
  causes, and the safety-versus-liveness lesson; linked from README.

## 0.1.1 — 2026-10-09

- Experiment A simulation tier completed: A3′ sharded lane-head arm with
  read-dependency churn (L1/L2/L3) and the epoch-sealed control arm; A2
  independent Message Cell arm with honest/lazy builders and
  forced-inclusion versus penalty-only challenge semantics.
- Pre-committed decision rules (design §5) with explicit thresholds.
- Runner emits `specs/EXPERIMENT_A_REPORT.md` (deterministic seeds).
- CI: fmt + clippy -D warnings + tests + harness smoke.

## 0.1.0 — 2026-10-09

- Project inception. Architecture specification v0.2.5 frozen; Experiment A
  design; A1 atomic-OrderingHead admission simulation (fee-ratio vs DOA).
