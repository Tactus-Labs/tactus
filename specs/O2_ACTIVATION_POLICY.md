# Tactus O1 — O2 External Data Availability: Day-0 Boundary and Activation Policy

**Status:** decision record — maintainer-accepted posture, 9 October 2026; not implementation evidence; no gate claims (G1–G9 remain OPEN)
**Baseline:** [TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md](TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md) §10; [DAG_ACCELERATION_NOTE.md](DAG_ACCELERATION_NOTE.md) §8
**Position in one line:** *Day 0: O1 ZK rollup with CKB DA. O2-ready architecture, not O2-ready implementation.*

---

## 0. Why external DA is not a performance switch

DAG parallelism is a performance optimisation that can be deferred without changing specified execution results. External DA is different in kind: it changes **whether users can independently reconstruct state and prove withdrawal rights**. A validity proof guarantees state correctness, not data availability — a data provider may withhold exactly the data a user needs to construct an exit proof. External DA is therefore a *deferred data-security model*, never a feature toggle, and must never be introduced in a way that silently alters the guarantees of already-live assets.

## 1. Must / deferred / excluded at Day 0

| Work | Day 0 | Reason |
|---|---|---|
| Full CKB DA implementation | **Required** | the security basis of O1 |
| `da_policy_id` and rollup domain identity | **Required** | prevents cross-domain impersonation |
| Batch payload commitment (witness-committed envelope) | **Required** | proofs must bind to the exact published data |
| Independent data-reconstruction tests | **Required** | users must not depend on the original builder (Experiment C logic) |
| O1 DA cost and throughput benchmark | **Required** | decides the real necessity of O2 |
| External DA network integration | Deferred | does not affect O1 correctness |
| DA committee / DAS verification | Deferred | O2-specific trust model |
| O1 ↔ O2 asset migration | Deferred | both security domains must be validated first |
| Per-transaction volition | **Excluded** | the protocol supports no such mixed state model |

## 2. The domain boundary, from genesis

Each domain is a separate security and deployment boundary from day one:

```text
Tactus O1 (mainnet)                  Tactus Pulse (future O2 domain, separate genesis)
  rollup_id                          separate rollup/domain identity
  da_policy_id = CKB_DA              da_policy_id = EXTERNAL_DA
  ethereum_state_root                separate ethereum_state_root
  ordering_head                      separate ordering_head
  settlement_tip                     separate settlement_tip
  asset_vault                        separately accounted asset vault
```

The reference O2 domain carries the product name **Tactus Pulse** (named 9 October 2026); `O2` remains the technical designation inherited from the 10815 option matrix. Domain-labelling discipline applies from the first deployment: distinct chain ID; DA policy and recovery assumptions disclosed in chain-list metadata; `Tactus O1` denotes the O1 mainnet, never the O2 domain.

**Brand structure (decision, updated 9 October 2026): one mother brand, two networks, shared infrastructure** — the Arbitrum One/Nova model. Day 0 launches a single network, **Tactus O1** (`tactus-o1`), as the mainnet. **Tactus Pulse remains a dormant sub-brand** until the activation gate (§3) opens. The O1 name is explicit from Day 0 and remains in place if Pulse activates; this supersedes the provisional “Tactus” / “Tactus One” naming plan. The Pulse name is not trademark-cleared and O2 is not under development; both facts stand until separately decided.

| Unified across the brand | Kept separate across domains |
|---|---|
| Tactus umbrella brand and developer portal | chain ID and RPC network identity |
| Reth/revm execution stack | Ethereum state root |
| SDK, wallet integrations and base tooling | DA policy and security claims |
| proving infrastructure software | SettlementTip and custody accounting |
| ecosystem developer relations | withdrawal, recovery and fault-handling rules |

Liquidity does not inherit security by sharing a brand: assets cross between the mainnet and Pulse only by explicit proven transfer, and the validium DA risk is never silently passed to rollup users. Unified brand, explicit security boundary.

The domains may share Reth/revm, the prover, RPC software and block-building tooling; they may **not** share an unrestricted global state root. Consequences: O1 users never silently inherit new data-availability risk from O2's introduction; moving assets between domains requires an explicit, user-visible cross-domain transfer under a declared security model — never a quiet relocation of execution data to external DA.

## 3. O2 Activation Gate — conditions, not dates

O2 engineering starts when **all four** hold:

1. O1 sustained settled throughput has been measured under realistic workloads.
2. The measured bottleneck is CKB DA — not proving, not EVM execution.
3. Target applications' throughput or cost requirements demonstrably exceed what O1 can economically support.
4. Verifiable solutions exist for external-DA availability proofs, historical retrieval, failure recovery and user exit.

One standing qualification: **data availability is not permanent data storage.** Even a DAS-capable network requires separately specified historical retrieval and archival. The gate's fourth condition includes that work.

*Escape valve:* if pre-launch benchmarks already show O1 cannot serve the first target applications, start O2 engineering PoC early and re-examine product scope — but never let an unvalidated O2 delay O1's secure launch.

## 4. Priorities (aligned with DAG note §8)

- **P0 — Day 0:** based sequencing, priority inclusion, serial EVM, CKB DA, ZK settlement, asset custody, independent exit.
- **P1 — O1 performance and cost:** batch compression, transaction-data encoding, prover pipeline, checkpoint cost, high-frequency EVM block support (block model per DAG note §1.4). Within O1 throughput work, the lever order is compression → verifiable state-diff DA → Fiber payment offloading (§6).
- **P2 — O2 external DA:** when the activation gate opens — independent validium domain development and its own security acceptance. Gates are claimed per domain; O1 passing any gate never transfers to O2.

**Day-0 positioning, restated:** *a genuinely based, validity-enforced EVM rollup secured by CKB, with independently reconstructable data.* Implement and verify that security model first; let G4 evidence decide whether O2 is ever added — never weaken a live domain's guarantees to buy throughput.

## 5. O1+ vs O2+ — orthogonality, convergence and the layered product (decision refinement, 9 October 2026)

**Orthogonality.** O2 addresses data throughput and publication cost; fast preconfirmation addresses short-horizon ordering determinacy. Neither substitutes for the other. The correct comparison is therefore **O1+** (CKB DA + based sequencing + ZK settlement + economically bonded preconfirmation) versus **O2+** (external DA + the same three plus the same preconfirmation layer).

**Convergence.** Both variants share CKB PoW canonical ordering, so O2 removes no reorg risk and grants no fast-state advantage — competing batches can displace either. O2 additionally owes cleanup semantics for data already published externally whose batch missed canonical inclusion. Corollary for budget: *if fast-state continuity is the binding product problem, the research spend belongs to preconfirmation and soft-head continuity (OPERATIONAL_POSTURE §8), not to switching DA.*

| Dimension | O1+ | O2+ |
|---|---|---|
| 100 ms soft confirmation | via preconfirmation service | identical — same dependency |
| Soft-confirmation rollback, CKB reorg | present | present — not reduced |
| Sustained TPS ceiling | CKB DA bandwidth (597,000 bytes/block, shared with all CKB activity) | materially higher DA ceiling |
| Per-transaction DA cost | CKB byte cost × compression | potentially lower |
| Independent reconstruction | from CKB-published data | external-DA and retention dependent |
| Operator disappearance | independent nodes recover from CKB | depends on external retrievability |
| Exit security | stronger base case | data-withholding risk (frozen-exit hazard) |
| Engineering complexity | lower | cross-system verification, recovery, fault handling |

O2 lifts only the DA term of the throughput bound `min(T_execution, T_DA, T_proving, T_ordering)`; the other three move not at all.

**Layered product, not competing chains.** Tactus O1 is the core rollup — the brand mainnet and long-term security baseline — for general EVM DeFi, custody, and recovery-sensitive applications. **Tactus Pulse** — the O2 high-throughput domain — is a later domain for order-flow-heavy applications that knowingly accept declared external-DA risk. The domains share software and proving infrastructure; they keep separate state roots, DA policies and asset boundaries; assets cross only by explicit proven transfer with a user-visible change of security model. Real-time DA availability never substitutes for archival and long-term retrieval.

**Staging refinement (default order; demand may reorder, §3's gate stands):**

1. **Day 0 — O1 security closed loop:** high-speed soft execution permitted; complex bonded preconfirmation *not* required.
2. **Fast DeFi upgrade:** credible soft-head continuity, signed preconfirmation and breach liability — Taiko's based-preconfirmation economics as a reference, adapted to CKB PoW's absent next-proposer knowledge (OPERATIONAL_POSTURE §8).
3. **O2 activation:** only on real G4 evidence that CKB DA is the binding bottleneck, with its own recovery and security acceptance.

**Verdict.** O1+ is the better infrastructure mainline; O2+ is the better high-throughput scaling route; and the hardest fast-trading finality problem is *the same problem for both*. The mainnet is never converted to a validium to chase trading flow — the flow gets its own domain.

## 6. Fiber — payments rail, not a DA substitute (decision, 9 October 2026)

**Division of labour.** *Fiber scales payments; Tactus O1 scales general-purpose EVM execution; CKB provides canonical ordering, data availability and settlement.* Fiber offloads high-frequency CKB/xUDT payments into channels, removing traffic that would otherwise land on O1's DA budget — and changes none of the ceilings: the 597,000-byte block limit stands, shared EVM state (an AMM swap's pools, ticks and fee growth) still publishes through O1, and PoW reorgs, priority-inbox contention and proving cost are untouched.

**Transport is not a DA claim.** Using Fiber nodes to propagate batch data is researchable; equating that with rollup DA is not. Fiber's protocol guarantees payment-channel state between participants — nodes need not retain others' channel histories, and its security model is keep-your-latest-state plus watchtowers. A pipeline of *batch data → Fiber, hashes → CKB* leaves new nodes unable to reconstruct EVM state and users unable to build withdrawal proofs. Bolting on replication, availability certification, archival and recovery turns it into a new external-DA protocol — at which point the deployment **is O2**, subject to the §3 gate, no longer O1. Fiber may carry bytes; it may never carry O1's DA security claim.

**O1 throughput levers, in order of benefit-to-intrusiveness:**

1. **Batch data compression** — shrink reconstructable bytes per L2 transaction (Godwoken's 2022 proposal estimated 253 bytes per ERC-20 L2 transaction — a historical estimate, never a measurement; Tactus O1's compression targets are set against its own encodings under G4).
2. **Verifiable state-diff publication** — reduce required data where applicable, with recovery demonstrated: a state root alone is not DA; which state, history and proof material remain recoverable must be specified.
3. **Fiber payment offloading** — reduce the payments that enter Tactus O1 EVM at all.

The underlying constraint: `TPS_O1 ≲ (L1 DA byte-rate available to Tactus O1) / (average reconstructable bytes per L2 transaction)`, further bounded by proving and ordering.

**Combined stack — a direction, not a Day-0 dependency.** Tactus O1 for EVM DeFi, AMM, lending and complex settlement; Fiber for instant payments, routing and micropayments; CKB for PoW security, custody and canonical settlement. Caveats recorded: Fiber balances and Tactus O1 EVM balances do not merge naturally — cross-system custody, settlement and liquidity mechanisms are prerequisites; feeding a Fiber payment atomically into a Tactus O1 contract requires a new cross-system protocol, not two SDKs joined. Day 0 takes no Fiber dependency; future asset-interop interfaces are designed, not assumed.

**The stablecoin loop — the combined stack's first concrete use case (mid-term roadmap item; not a Day-0 or consensus dependency).** The prerequisite is *asset identity*: the CKB xUDT is the canonical form, and Tactus O1 holds a fully collateralised, vault-bridged ERC-20 mapping of it. A shared name and symbol do not make two assets the same — issuance rules, asset identity and the ERC-20 mapping must each be verified. Two routes then exist for spending Tactus O1-held stablecoins:

- **Route A — the base path, independently executable:** proven withdrawal to the xUDT → channel funding → repeated micropayments, with no per-payment footprint on Tactus O1, the prover or CKB block production. Fiber's interfaces already accept the asset (`open_channel` takes `funding_udt_type_script`; invoices name `udt_type_script`). The first hop pays the cross-layer settlement latency.
- **Route B — the UX path, commercial and to be designed:** a Tactus O1–Fiber liquidity gateway pays the merchant from pre-positioned channel liquidity and collects the user's Tactus O1 funds per protocol. This requires collateral, atomicity, refund and risk-bearing rules; a shared hashlock does not make it trust-free. Long-term posture: B for experience, A always available as the independent base path.

The binding obstacle is **liquidity, not TPS**: an asset being protocol-supported is not the network holding directional channel liquidity for it — public mainnet nodes have lacked stablecoin channel liquidity even where testnet tutorials worked. Requirements: L1 representation, a verified bridge, node UDT support, channel liquidity, wallet/gateway UX. Traffic divides accordingly — retail payments, AI-agent per-call billing and per-second content ride Fiber; AMM, lending and clearing ride Tactus O1; merchant sweeps travel Fiber → CKB → Tactus O1. Micropayments never become EVM state transitions, and ecosystem payment volume decouples from O1's DA ceiling.

**Priority validation — one loop:** Tactus O1 ERC-20 → CKB xUDT → Fiber micropayment → CKB xUDT → Tactus O1 ERC-20, with asset mapping, cross-layer settlement, channel liquidity and exit safety each demonstrated. If the loop holds, this is the CKB-flavoured product combination: composable finance on Tactus O1, high-frequency payments on Fiber, one common asset-security base on CKB.

## 7. DA provider policy — neutral interface, competing adapters (decision, 9 October 2026)

**Correction of record.** Two earlier statements are corrected here. First: using Myelin-derived DA software does not, by itself, make a deployment O4 — O4 names the *optimistic-settlement + external-DA* pattern (a correctness-model failure), and O2 remains validity-settled regardless of whose software carries the bytes. Second: the "related operator pollutes the based claim" principle applies to canonical sequencing and settlement authority, and to any DA arrangement for O1 — where DA is CKB and no external trust domain is permitted. O2's DA slot is, by definition, a declared external trust domain; the criterion there is not team identity but the §3 gate — verifiable availability evidence, recovery and exit — with fault domains diversified so that no single party, least of all one correlated with the rollup operator, controls the DA set. The remedy for correlation risk is diversified control and independently verifiable evidence, not the banning of a team's software.

**Provider-neutral DA interface (conceptual, not an implementation):**

```text
trait DataAvailability {
    publish(blob) -> BlobCommitment
    verify_evidence(commitment, evidence) -> Result<()>
    retrieve(commitment) -> Result<blob>
}
```

Real implementations add asynchronous request handling, fault proofs, independent verification, lifecycle and attestation rules. Adapters compete: an external DA network (Celestia-class) may implement one; a **Myelin-derived adapter** may implement another — Myelin's DA evidence plumbing (blob commitments, provider receipts, retrieval probes, policies, certificates) is engineering separable from its session consensus and court, and confers no execution-layer advantage. Adoption is decided solely by the §3 requirements.

**Precise O2 security claims — stated exactly, correcting an over-clean formulation.** A proven state root is not exitability:

1. Wrong states cannot settle — conditional on proof-system and binding correctness.
2. Data withholding can still halt transactions, proof generation and exits.
3. Exit proofs require authenticated witnesses drawn from available data; withholding denies exactly those witnesses (spec §11 already refuses to assume universal recoverability).
4. Independent archival, DAS, challenges and penalties reduce — never eliminate — the residual risk; data having been available once is not a promise of permanent retrieval.

**Research priority:** do not build a new DA network in order to reuse Myelin. Evaluate existing networks' security and cost first (§3's measurements); a Myelin adapter is justified only if it demonstrably outperforms them against the gate. Myelin is thereby neither a required dependency nor a banned contributor: it is a competitor on the DA adapter market, judged by the same evidence as everyone else.
