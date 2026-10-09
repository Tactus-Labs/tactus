# Changelog

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
