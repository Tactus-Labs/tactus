# Tactus O1

Project and repository name: `tactus-o1`.

An independent, CKB-based EVM validity rollup: off-chain-first execution,
permissionless CKB-based canonical sequencing, validity-enforced settlement.

> **Protocol thesis.** Execute Ethereum transactions off-chain for performance;
> allow professional but non-privileged builders to assemble candidate batches;
> use CKB PoW and Cell transitions for canonical batch succession; settle only
> validity-proven EVM state transitions; preserve the data and witnesses required
> for the security domain's stated recovery guarantees.

## Status

- **Architecture baseline:** frozen at [v0.2.5](specs/TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md) (9 October 2026).
- **Evidence gates:** G1–G9 are all **OPEN**. Nothing here is an implemented
  protocol property, a measured performance result, or production authorisation.
- **Current deliverable:** [Experiment A](specs/EXPERIMENT_A_DESIGN.md) — the
  priority-admission comparison of A1 (atomic OrderingHead reference), A2
  (independent Priority Message Cells) and A3′ (sharded lane heads, with an
  epoch-sealed snapshot control arm) under identical adversarial CKB devnet
  conditions. G2 — censorship resistance — is the blocking gate.
- **Simulation tier: complete.** All three arms plus the sealed control run
  with deterministic seeds and pre-committed decision rules; reproduce with
  `cargo run --bin tactus-o1-experiment-a` and read
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
- **Day-0 posture (decision, 9 October 2026):** correctness-first — serial revm,
  linear CKB ordering, validity proofs, priority inbox, CKB DA, basic bridge and
  exits. No execution DAG, no microbatch DAG, no canonical DAG. The protocol
  boundary to settle *and test* before Day 0: the multi-EVM-block-per-anchor
  model, batch commitment format and proof binding rules — see the
  [DAG note §8 decision record](specs/DAG_ACCELERATION_NOTE.md).
- **O2 posture (decision, 9 October 2026):** O2-ready architecture, not
  O2-ready implementation — Day 0 is O1 with CKB DA; external DA is a deferred
  *data-security model*, never a performance switch, and opens only through
  the four-condition [activation gate](specs/O2_ACTIVATION_POLICY.md).
  Layered product: O1 core rollup as mainnet and security baseline; the O2
  domain is named **Tactus Pulse** — O2 and preconfirmation solve orthogonal
  problems and converge at the CKB ordering layer; "Tactus O1"
  always means the O1 mainnet.
- **Block pipeline (decision, 9 October 2026):** high-frequency speculative
  blocks, low-frequency CKB anchors, asynchronous validity settlement.
  Commitment ≠ data availability: anchors are valid only with atomically
  published reconstruction data — hashes alone are prohibited as an O1 claim
  ([DAG note §9](specs/DAG_ACCELERATION_NOTE.md)).
- **Operational posture (decision, 9 October 2026):** operational centralisation
  and consensus authority are separate claims — services may be dominated by one
  operator, canonical ordering may not; Temporary Execution Buffer ≠ external
  DA; no globally consistent 100 ms soft head under permissionless builders
  (fast local speculative blocks adopted); minimum Day-0 deployment and the
  stage-by-stage claims ladder in [OPERATIONAL_POSTURE.md](specs/OPERATIONAL_POSTURE.md).
- **Fast DeFi posture (research direction, 9 October 2026):** two confirmation
  lanes — a ~100 ms fast lane (execution, soft blocks, optional **bonded**
  preconfirmation) over the CKB-cadence settlement lane; preconfirmation buys
  compensation, never irreversibility ("economically protected soft
  confirmation", not fast finality); the general-L2 vs Hyperliquid-grade fork
  is recorded OPEN in [OPERATIONAL_POSTURE.md §8](specs/OPERATIONAL_POSTURE.md).
- **Fiber posture (decision, 9 October 2026):** payments rail, not a DA
  substitute — Fiber may offload payment traffic and carry bytes, but never
  O1's DA security claim (replication + availability + archival machinery
  would make it O2, behind the gate). Combined-stack direction: Tactus O1 (EVM
  DeFi) + Fiber (payments) + CKB (settlement); Day 0 takes no dependency —
  interop interfaces designed, not assumed.
- **Work plan (9 October 2026, post-review):** architecture stays; resources
  shift from designing features to proving the two hardest properties —
  **G2** (forced inclusion under a hostile builder) and **W-12** (no accepted
  canonical batch may permanently stall settlement; the batch-admission
  envelope is protocol-enforced, never builder self-restraint). **P0** —
  EVM Block Model v0.2.6 (multi-block semantics with test vectors; gap table
  in the DAG note §1.4) · Experiment A devnet (implement, don't assume,
  forced processing) · Experiment B (end-to-end proof) · Experiment C
  (operator-independent recovery). **P1** — Fast-Head Continuity (soft-reorg
  UX under two independent builders) · CellScript integration (first joint
  prototype targets SettlementTip + Vault/Withdrawal, not the Priority
  Inbox). **P2** — Pulse/O2, parallel execution, Fiber, on measured need.
  If G2 proves unsatisfiable under existing CKB Script capabilities, the
  security claims are re-examined honestly — up to and including a minimal
  CKB consensus extension — never masked by soft TPS or larger penalties.
- **Simulation-tier honesty note:** A2's forced-inclusion row and the sealed
  A3′ arm are recorded as `ConditionalEnforcementPrimitiveUnimplemented` —
  they pass only under simulator assumptions; no G2 credit is taken.
- **Research notes:** [DAG acceleration and external DA](specs/DAG_ACCELERATION_NOTE.md)
  — execution-engine parallelism carries no protocol consequence but real
  engineering cost; batch-construction DAG depends on the **L2 block model**
  (how many EVM blocks one CKB anchor authenticates — the note's prior
  question, §1.4); canonical-ordering DAG is rejected; coexistence with the
  O2 external-DA domain examined; falsifiable reopen conditions stated.

## Repository layout

```text
specs/               # frozen architecture spec + experiment designs
crates/
  tactus-o1-protocol/     # protocol primitives (illustrative companions to the spec)
  tactus-o1-experiment-a/ # Experiment A harness: workload models, metrics, scenarios
```

Crates are added only when their protocol boundary is justified; the canonical
wire specification lives under `specs/` and is never defined implicitly by a
Rust struct.


## Claim discipline

A design goal is not an implemented property. A local fixture passing is not a
public-network security result. A soft confirmation is not settled finality. A
cost estimate is not measured throughput.
