# Tactus

An independent, CKB-based EVM validity rollup: off-chain-first execution,
permissionless CKB-based canonical sequencing, validity-enforced settlement.

> **Protocol thesis.** Execute Ethereum transactions off-chain for performance;
> allow professional but non-privileged builders to assemble candidate batches;
> use CKB PoW and Cell transitions for canonical batch succession; settle only
> validity-proven EVM state transitions; preserve the data and witnesses required
> for the security domain's stated recovery guarantees.

## Status

- **Architecture baseline:** frozen at [v0.2.5](specs/TACTUS_ARCHITECTURE_SPEC_v0.2.5.md) (9 October 2026).
- **Evidence gates:** G1–G9 are all **OPEN**. Nothing here is an implemented
  protocol property, a measured performance result, or production authorisation.
- **Current deliverable:** [Experiment A](specs/EXPERIMENT_A_DESIGN.md) — the
  priority-admission comparison of A1 (atomic OrderingHead reference), A2
  (independent Priority Message Cells) and A3′ (sharded lane heads, with an
  epoch-sealed snapshot control arm) under identical adversarial CKB devnet
  conditions. G2 — censorship resistance — is the blocking gate.
- **Simulation tier: complete.** All three arms plus the sealed control run
  with deterministic seeds and pre-committed decision rules; reproduce with
  `cargo run --bin tactus-experiment-a` and read
  [`specs/EXPERIMENT_A_REPORT.md`](specs/EXPERIMENT_A_REPORT.md). Headline
  simulation-tier findings (not gate passes; devnet tier pending):
  - **A1** — fee priority rescues conflicts it can reach, never stale
    OutPoints: at a 3-block signing delay, DOA reaches 34% even at 10× fees
    → reference implementation only.
  - **A3′ live-head references** — collapse under adversarial churn
    (survival 0.01) and degrade as per-lane load grows, while the
    epoch-sealed control arm is churn-immune at bounded processing delay.
  - **A2** — admission is contention-free by construction; challenges that
    only exact a penalty leave messages unprocessed (`G2 not passed`),
    forced inclusion restores them at bounded delay.

## Repository layout

```text
specs/               # frozen architecture spec + experiment designs
crates/
  tactus-protocol/     # protocol primitives (illustrative companions to the spec)
  tactus-experiment-a/ # Experiment A harness: workload models, metrics, scenarios
```

Crates are added only when their protocol boundary is justified; the canonical
wire specification lives under `specs/` and is never defined implicitly by a
Rust struct.

## Relation to Myelin

Tactus is independent of [Myelin](https://github.com/Myelin-Labs/Myelin). Myelin
is a historical research influence and remains a separate CKB-isomorphic Cell
session runtime; it is not an ordering, DA or settlement authority here, and no
Myelin consensus or runtime dependency is required.

## Claim discipline

A design goal is not an implemented property. A local fixture passing is not a
public-network security result. A soft confirmation is not settled finality. A
cost estimate is not measured throughput.
