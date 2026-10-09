# Changelog

## Unreleased

- Add isolated A1/A2/A3 CKB mechanism experiments on 0.121.0 and 0.210.0,
  with raw evidence, independent actor keys, fee/delay races, omission and
  dependency-churn controls, planned reorg and canonical head recovery.
- Implement seed-bound unique OrderingHead genesis, forbid split/burn/lock
  takeover/capacity drain, reject counter wrap and add a type-bound permissionless
  lock. Use VM v1 (`data1`); validate the linked ELF on actual CKB.
- Repair deployment inputs, fee funding, group-aware signing, SECP dependency
  discovery, Molecule empty vectors and occupied-capacity accounting. Bound RPC
  waits, isolate devnet writes, and require the exact expected rejection reason.
- Add a pinned two-version CI matrix and production-readiness/fixture-boundary
  documentation. Full Experiment A and all production gates remain OPEN.
- Fresh 0.210.0 devnet signatures pass; retract the earlier broad attribution of
  signature failures to that node version. The old chain's root cause is unresolved.

- Rename the O1 project and repository to `tactus-o1` (display name: Tactus O1),
  including crate names, source imports, build artifacts, CI commands and spec links.
  Tactus Pulse and Tactus Beat retain their separate names. Existing devnet hash
  inputs remain unchanged so recorded experiment identities stay reproducible.
  This supersedes the earlier provisional mainnet naming decision below.

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
- Decision record `specs/O2_ACTIVATION_POLICY.md`: Day 0 = O1 with CKB DA,
  O2-ready architecture not implementation; must/deferred/excluded table;
  genesis domain-boundary structures (separate roots, ordering heads,
  settlement tips, vaults — shared software, never a shared unrestricted
  state root); four-condition O2 Activation Gate (measured O1 throughput, DA
  as the binding bottleneck, demonstrated application demand, verifiable
  availability/retrieval/recovery/exit); DA ≠ permanent storage; P0/P1/P2
  aligned with the DAG note §8 phases.
- Block-pipeline decision (DAG note §9): high-frequency speculative blocks,
  low-frequency CKB anchors, asynchronous validity settlement; commitment ≠
  DA (hash-only publication prohibited as an O1 claim); atomic anchor–
  publication rule (same-tx data or verifiable shard completion); draft
  TactusO1Batch format (multi-block range commitments); production latency ≠
  finality — no preconfirmation authority; adopted Day-0 parameter table.
- Decision record `specs/OPERATIONAL_POSTURE.md`: operational centralisation ≠
  consensus authority; node-role table (no sequencer licence); Temporary
  Execution Buffer ≠ external DA (unsafe/safe precedent); four honest
  centralisation dimensions; fast-UX trilemma — no network-uniform 100 ms soft
  head under permissionless competition, fast local speculative blocks adopted;
  minimum Day-0 test deployment + four software principles; devnet/testnet/
  mainnet claims ladder.
- O2_ACTIVATION_POLICY §5 — O1+ vs O2+ refinement: orthogonality (DA
  throughput vs preconfirmation ordering determinacy — neither substitutes the
  other) and convergence (both meet at the CKB OrderingHead; O2 reduces no
  reorg or soft-rollback risk and adds missed-inclusion cleanup); O1+/O2+
  comparison table; O2 lifts only the DA term of the throughput bound; layered
  product (O1 core rollup mainnet + O2 high-throughput domain, explicit proven
  transfers); staging refined to Day 0 → Fast DeFi upgrade → O2 activation;
  verdict — mainnet never converted to validium for trading flow.
- O2 domain naming: the reference O2 deployment is named **Tactus Pulse**;
  `O2` remains the technical designation (10815 option matrix); unqualified
  "Tactus O1" always denotes the O1 mainnet; domain-labelling discipline
  (distinct chain ID, disclosed DA policy and recovery assumptions) recorded
  in O2_ACTIVATION_POLICY §2.
- Brand structure recorded (O2_ACTIVATION_POLICY §2): one mother brand, two
  networks, shared infrastructure (Arbitrum One/Nova model); Day 0 launches a
  single network "Tactus O1" — Pulse stays a dormant sub-brand until the
  activation gate opens, after which the mainnet may adopt "Tactus One";
  unify/separate table (brand, stack, SDK, proving software, devrel unified;
  chain ID, state roots, DA policies, settlement/custody, exit rules separate);
  liquidity never inherits security by sharing a brand.
- Kaspa assessment recorded: DAG note §3.4 (Based Apps / vProgs — study the
  lanes, not the DAG: KIP-21 SeqCommit and account-scoped dependencies worth
  research; DA capacity, withholding risk, soft-finality and product identity
  unchanged; verdict = R&D reference, not a dependency) and
  EXPERIMENT_A_DESIGN §9 (SeqCommit as a comparison point for A1/A2/A3′
  outcomes; first-source links).
- Fiber posture recorded (O2_ACTIVATION_POLICY §6): division of labour —
  Fiber scales payments, Tactus O1 scales EVM, CKB settles; transport is not a
  DA claim (data→Fiber + hash→CKB leaves reconstruction and withdrawal proofs
  unsolved; adding replication/availability/archival makes it O2 behind the
  gate); O1 throughput lever order — compression → verifiable state-diff DA
  → Fiber offloading, with the byte-rate/bytes-per-tx constraint; combined
  stack (Tactus O1 + Fiber + CKB) a direction, not a Day-0 dependency.
- Stablecoin loop recorded (O2_ACTIVATION_POLICY §6): CKB xUDT as canonical
  asset identity with vault-bridged ERC-20 mapping (same name ≠ same asset);
  Route A base path (proven withdrawal → xUDT → channel → micropayments;
  funding_udt_type_script interfaces exist) vs Route B liquidity gateway
  (commercial; collateral/atomicity/refund rules required; not trust-free by
  shared hashlock); binding obstacle = channel liquidity, not TPS; traffic
  division (micropayments → Fiber, DeFi → Tactus O1, sweeps → Fiber→CKB→Tactus O1);
  priority validation = one full loop with mapping, settlement, liquidity and
  exit safety each demonstrated.
- DA provider policy recorded (O2_ACTIVATION_POLICY §7), including a
  correction of record: Myelin-derived DA software does not make a deployment
  O4 (O4 = optimistic settlement + external DA, a correctness-model pattern);
  the related-operator principle governs sequencing/settlement authority and
  O1's DA, while O2's DA slot is a declared trust domain judged by the §3
  gate plus fault-domain diversity — not by team identity. Provider-neutral
  DataAvailability interface (conceptual); adapters compete (external networks
  or a Myelin-derived adapter — evidence plumbing separable from its court,
  no execution advantage); precise O2 claims stated (proven root ≠
  exitability; withholding denies exit witnesses; archival/DAS/challenges
  reduce, never eliminate); research priority: evaluate existing DA networks
  before any Myelin adapter decision.
- Godwoken precedent recorded (DAG note §9): PR #776's ProduceSubmitConfirm
  state machine (Local→Submitted→Confirmed, local/submitted limits as
  backpressure, P2P unanchored-block propagation, rollback-to-confirmed) is
  the first reference implementation for the Tactus O1 block pipeline — the
  production/CKB-submission decoupling is Godwoken 2022 work on the same L1,
  not a Tactus O1 novelty; Tactus O1's ladder extends it with Canonical (CKB-ordered)
  and Proven (ZK-verified), and its deltas are permissionless builders,
  validity settlement, multi-block-per-anchor and priority inclusion. Godwoken
  finality lesson noted (faster blocks broke block-count challenge windows and
  number/timestamp-dependent contract semantics — reinforcing §1.4's Day-0
  obligation); cadence ≠ throughput (no multiples claimed from 100 ms vs 8 s);
  253 bytes/tx ERC-20 recorded as a historical estimate reference point
  (O2_ACTIVATION_POLICY §6).
- UX posture recorded (OPERATIONAL_POSTURE §9): four pillars (fast EVM
  interaction, proof-driven withdrawal, near-native Ethereum tooling,
  independent recovery/exit — "not needing to believe the team will keep
  running"); two honest risks pre-recorded (permissionless ordering may be
  less stable in fast confirmation than Godwoken's privileged producer —
  mitigated by Fast-Head Continuity + bonded preconfirmation; "cheaper than
  Godwoken" unclaimable before G4); receipt-honesty RPC rule (no speculative
  receipt presented as final; Pending/Soft/Canonical/Proven explicit);
  Day-0 UX acceptance table including the network-sunset row (self-service
  exit, never contact-the-team); priority order with gas ranked last;
  product goal: fast enough to trade, reliable enough to build on,
  independently recoverable without its original operators.
- MegaETH/Sonic review recorded (OPERATIONAL_POSTURE §8 refinement): the
  liability dichotomy answers RQ4 — Equivocation (conflicting signed
  commitments for the same slot; provable, slashable) vs Non-inclusion (not
  adopted by CKB; not attributable — competition, miner choice, PoW reorg;
  never bondable, or honest builders bleed collateral for L1 outcomes they
  cannot govern); MegaETH borrowables (signer registry with rotation,
  mini-block ≠ EVM block two-tier as a live precedent for the §1.4 block
  model, realtime APIs under receipt-honesty labelling); Sonic boundary
  (aBFT finality is its own consensus, exits still traverse gateway
  confirmations — consensus speed ≠ settlement speed; Route C stays
  rejected); product question restated as risk tiers (swap-class labelled
  soft risk; CLOB/leverage/liquidation need canonical gating, bonded
  compensation or the Pulse domain).
- Post-review corrections (simulation honesty): A2's `challenge_forces_processing`
  is an *assumption* of the enforcement primitive no CKB script implements —
  all A2 rows and the sealed A3′ arm (whose processing-delay gate was never
  exercised; NaN silently passed limits) now decide
  `ConditionalEnforcementPrimitiveUnimplemented`, never
  `AdvanceToProductionReview` at simulation tier; "eventually processed" is
  not "processed within deadline" (98.6% violations recorded); starvation
  synthesis folded into the report generator (reproducible). No G2 credit
  taken.
- Block model elevated (DAG note §1.4): promoted to Architecture Spec v0.2.6
  as the P0 work package with the six-row gap table (EVM-block frontier in
  OrderingHead, ordered block-header binding, TIMESTAMP monotonicity under
  shared second timestamps, per-block gas/EIP-1559, deterministic BLOCKHASH,
  multi-block proof coverage) — consensus-critical semantics requiring
  normative test vectors, not an optimisation; stale §5.5 claim
  ("spec §3 permits multi-microbatch manifests") corrected.
- Work plan recorded (README): G2 and W-12 co-equal top research priorities;
  P0 = Block Model v0.2.6 + Experiments A-devnet/B/C; P1 = Fast-Head
  Continuity + CellScript (first prototype: SettlementTip + Vault/Withdrawal);
  P2 = Pulse/parallel/Fiber on measured need; honest re-examination clause if
  G2 is unsatisfiable under existing CKB Script capabilities.
- Committee boundary made explicit (OPERATIONAL_POSTURE §8): a signer
  registry is not a committee while no closed set's consent is necessary for
  canonical ordering or settlement (the test PoA/Tendermint fail, a
  preconfirmation market passes; spec §4.2 forbids signatures purchasing
  proposal rights); registry semantics fixed as bond-to-enter or
  verification-only — never a licence or choke point; MegaETH's
  single-sequencer exclusivity deliberately dropped (plural competitive
  providers); residual risk is de facto dominance, governed by §3 claims
  discipline.
- OPERATIONAL_POSTURE §7: dispute-adjudication map (five dispute types);
  the complete no-committee argument (a committee solves a problem Tactus O1 does
  not have and neither of the ones it does); preconfirmation services allowed
  economically, never statutorily — bonded promises compensate, never rewrite
  canonical history; confirmation-naming discipline ("100 ms finality" not
  claimable).
- OPERATIONAL_POSTURE §8 — Fast DeFi research direction: three performance
  goals separated (execution / network-consistent ordering / L1 finality);
  two rollback sources (L1 reorg vs pre-anchor competition); routes A/B/C with
  the PoW-no-next-proposer caveat — bonded preconfirmation compensates, never
  makes irreversible ("economically protected soft confirmation"); two
  confirmation lanes, not two consensus layers, with confirmation-depth L1
  origins; five research questions; product fork (general L2 vs
  Hyperliquid-grade determinism) recorded OPEN with recorded recommendation:
  general L2 + Route B research.
- Experiment A report: added a starvation synthesis section — definition,
  three observed forms (admission / progression / processing starvation) with
  causes, and the safety-versus-liveness lesson; linked from README.

- OPERATIONAL_POSTURE §8 second refinement: the certificate-gating test (does
  the OrderingHead accept a valid batch without a committee certificate —
  soft-confirmations-only vs protocol-level ordering veto); shared mempools
  and hash-ordering conventions cannot yield a network-unique soft head;
  staged coordination plan P0 permissionless → P1 open preconfirmation market
  (measure fulfilment rate, soft-reorg frequency, divergence, compensation
  cost, switching time) → P2 threshold/BFT only on demonstrated need, never a
  precondition for OrderingHead advancement; builder vs sequencing-coordinator
  role separation (PBS-flavoured); red line recorded — G2 is never solved via
  a fast committee (a committee can collectively refuse a user); research
  question: an economically credible shared fast head under CKB PoW without
  exclusive canonical sequencing rights.

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
