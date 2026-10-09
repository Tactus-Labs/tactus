# Tactus — Operational Posture: Centralisation, Buffers and Fast-UX Claims

**Status:** decision record — maintainer-accepted posture, 9 October 2026; not implementation evidence; no gate claims (G1–G9 remain OPEN)
**Baseline:** [TACTUS_ARCHITECTURE_SPEC_v0.2.6.md](TACTUS_ARCHITECTURE_SPEC_v0.2.6.md) §1.3, §2; [DAG_ACCELERATION_NOTE.md](DAG_ACCELERATION_NOTE.md) §9; [O2_ACTIVATION_POLICY.md](O2_ACTIVATION_POLICY.md)
**Position in one line:** *Centralised performance where convenient, permissionless authority where essential, and CKB-backed recoverability where security matters.*

---

## 0. The distinction that governs everything

Tactus requires off-chain services and must cache EVM blocks not yet anchored to CKB. None of that implies a centralised sequencer. The load-bearing distinction is between **operational centralisation** (who runs the services) and **consensus authority** (who can make canonical history). Off-chain services may be professional and even temporarily dominated by one operator; canonical ordering authority may not be monopolised by anyone — it rests with CKB PoW and permissionless, script-validated succession (spec §1.3).

A corollary for the builder role: a builder holds no exclusive interpretation of Tactus state. It orders transactions *within its candidate batch*; nothing it produces becomes canonical history until CKB accepts it.

## 1. Node roles

| Node | Who may run it | What it holds |
|---|---|---|
| RPC gateway | Official, third-party, community | mempool, request caches |
| Batch builder | Anyone | pending transactions, speculative blocks, execution state |
| Execution full node | Anyone | EVM state database, canonical history |
| DA publisher | The submitting builder or a service | data pending publication, until CKB confirms |
| Prover | Anyone with compute | execution witnesses, proving inputs |
| CKB node / miner | CKB network participants | CKB blocks and transaction data |

No role requires an officially issued sequencer identity; one codebase runs all of them by configuration.

## 2. The Temporary Execution Buffer is not external DA

A builder producing 100 ms blocks against ~10 s batch cycles necessarily buffers ~100 blocks. The buffer may be the builder's local memory and RocksDB, P2P propagation across execution nodes, or optional object storage for failure recovery — these are operational components. Two rules keep them harmless:

1. **Speculative until published** (DAG note §9): until CKB confirms the anchor *and* its reconstruction data, buffered blocks serve only off-chain execution and fast responses. Buffer loss may force rebuilding unpublished soft blocks; it can never impair the independent recoverability of any state for which O1 guarantees are claimed.
2. **The standing prohibition**: if only the builder holds the data while CKB receives hashes, treating those blocks as safely settled breaks O1's availability guarantee. OP Stack's unsafe/safe distinction is the operating precedent.

## 3. Four dimensions, honestly

| Dimension | Target | Residual risk |
|---|---|---|
| Canonical sequencing | Decentralised — CKB PoW decides | CKB miners' own ordering power and MEV |
| Fast block production | Multiple builders permitted | early dominance by one builder |
| Data availability | O1 on CKB | temporary operational services before soft-block publication |
| Proof & settlement | Permissionless provers + CKB verification | prover concentration or capacity shortfall |

Protocol-level decentralisation therefore coexists with operationally centralised reality: Day 0 may ship with an official high-speed builder as the primary. That is permissible **only if** others can in practice bypass it — construct and submit valid batches independently. If every wallet connects only to the official RPC, no transaction gossip exists, no independent builder operates, and the priority inbox cannot yet force inclusion, then the honest claim is *"the protocol permits permissionless participation"*, not *"censorship resistance achieved"*. The overall guarantee is constrained by the weakest privileged role, per spec §1.3.

## 4. The fast-UX trilemma — no free globally consistent 100 ms head

Permissionless based sequencing does **not** provide a network-uniform 100 ms soft block head. Three builders receiving different transactions each produce a valid 101st block in 100 ms; whichever candidate CKB later accepts canonicalises one history and orphans the rest. Users connected to the losing builder see reorged soft confirmations. The engineering routes:

| Fast-UX approach | Consequence |
|---|---|
| One dominant high-speed builder | simple UX; soft confirmations highly centralised |
| Competing independent builders | permissionless; soft blocks fork and roll back |
| Shared fast ordering / preconfirmation protocol | more uniform UX; new coordination, credit or economic-bond machinery — badly designed, it recreates the privileged sequencer Tactus exists to remove |

**Adopted choice:** fast *local* execution, explicitly labelled speculative, with short-lived soft forks accepted and CKB as the sole canonical decider. Day 0 does not chase 100 ms globally consistent preconfirmation. This is more honest to the architecture than pretending all nodes share one 100 ms chain.

## 5. Minimum Day-0 test deployment

- **1 primary builder** — official, carrying the main high-speed execution and batching load;
- **≥ 1 independent builder** — validating permissionless entry, independent batching and failover;
- **multiple full nodes** — independently replaying and verifying EVM state from CKB DA;
- **≥ 1 backup prover** — validating that proof generation does not depend on the official operator.

This is a minimum *test* shape, not production redundancy and not a security audit. Four software principles govern the codebase:

1. **Anyone can start a Tactus node** — the same binary runs builder, follower, RPC or prover by configuration; no official sequencer licence exists.
2. **Formal state is fully reconstructable** — a fresh node recovers canonical EVM history from genesis plus CKB-published O1 data, never from an official database snapshot.
3. **Temporary caches may distribute but never rule** — cache loss touches only unanchored soft blocks, never settled-state reconstruction.
4. **Third-party builders must be able to participate in practice** — transaction propagation, data access, fees, the CKB proposal window and the priority inbox must not form a de facto official monopoly. Principle 4 is measured by Experiment A, above all G2: forced inclusion under a hostile primary builder.

## 6. Claims ladder by stage

| Stage | Operation | Honestly claimable |
|---|---|---|
| Devnet | Official primary builder + independent test builder | experimental validation of permissionless proposing |
| Public testnet | Multiple builders, P2P transaction propagation, independent provers | ordering liveness and node recovery under public conditions |
| Mainnet | No dependence on a designated builder for ordering or forced inclusion | censorship resistance and validity settlement under measured assumptions |

There is no need to eliminate professional builders — Ethereum runs highly specialised executors, builders, provers and RPC services. What must never exist is an operator with an unbypassable protocol privilege. The 100 ms-versus-permissionlessness tension is real and permanent; the adopted trade — fast local soft blocks, short forks, CKB deciding — is the one consistent with a single-consensus based rollup.

## 7. Dispute adjudication and the no-committee verdict (decision, 9 October 2026)

Three powers, three mechanisms — and one gap that is ours to close:

| Dispute | Adjudicated by | Mechanism |
|---|---|---|
| Conflicting candidate histories (builder A vs B) | CKB PoW | conflicting `OrderingHead` spends yield exactly one canonical successor |
| Which EVM state root is correct | CKB ZK verifier | only proofs matching the canonical batch history and prior state root settle |
| Forged transaction data | CKB scripts + DA commitments | batches failing commitment verification are rejected |
| A builder ignoring a user's transaction | Priority Inbox | **unresolved — Experiment A; G2 is blocking** |
| Contradictory soft confirmations | no final coordinator | CKB eventually selects one history; early soft confirmations may roll back |

Framing: *CKB provides final arbitration; Tactus provides deterministic execution rules; builders provide only candidate results.* No L2 committee votes on canonical history. The complete argument runs one step further than "unnecessary": a committee is a solution to a problem Tactus does not have — conflicting histories are already adjudicated by CKB single-consumption — whilst being a non-solution to the problems it does have. Forced inclusion requires enforceable protocol rules (Experiment A); settlement liveness on an unprovable batch requires admission totality (spec W-12). A voting body would reintroduce the trust anchor the architecture removed while fixing neither.

**Preconfirmation services: allowed economically, never statutorily.** At the protocol layer, builders compete (Scheme A). Operationally, a professional preconfirmation service (Scheme B) may offer uniform fast soft confirmations, provided it holds no statutory ordering authority, users and other builders can bypass it, and — if its promises are to be punishable — it operates under explicit commitment signatures, bonds, breach conditions and on-chain verifiable liability. A failed bonded preconfirmation compensates the user; it never rewrites canonical history. **Money settles broken promises; consensus never does.** This mechanism is separate from, and strictly optional to, the censorship-resistance work; the two are built independently, and neither is a committee.

The confirmation-naming discipline follows: 100 ms soft confirmation (a builder executed or promised — may be revoked), CKB canonical ordering (batch in the current canonical history — still reorgable), ZK proven settlement (verified on CKB — subject to L1 confirmation depth). "100 ms finality" is not among the claimable strings.

## 8. Fast DeFi — soft-state continuity and preconfirmation economics (research direction, opened 9 October 2026)

**Three goals, honestly separated:**

| Goal | Status | Belongs to |
|---|---|---|
| 100 ms EVM execution | engineering target | execution engine |
| 100 ms network-consistent ordering | not yet built | soft sequencing / preconfirmation |
| 100 ms irreversible settlement | not providable by CKB PoW (probabilistic NC-Max finality) | L1 finality — out of scope |

High-frequency DeFi does not require 100 ms L1 finality; it requires a stable, shared fast execution history. Two rollback sources stress that history: L1 reorgs of anchored batches (rarer) and pre-anchor builder competition (the everyday case) — the second is the more frequent test of based-sequencing UX.

**Routes:**

| Route | Trading UX | Guarantee | Effect on Tactus |
|---|---|---|---|
| A. Pure based | fast execution; soft history replaceable | CKB final arbitration | current architecture |
| B. Based + bonded preconfirmation | fast shared soft history; breach compensable | economic promise, not L1 finality | adds fast-confirmation services and bond rules |
| C. Independent fast BFT sequencing | fast consistency, deterministic finality | separate validator set | **changes the core trust model** |

Route B is the priority research direction, with one CKB-specific caveat that forbids over-claiming: **PoW has no predictable next proposer**, so even an honest builder can fail to honour a preconfirmation through no fault of its own — miner choice or L1 reorg. Bonds therefore compensate; they never make a promise irreversible (mev-commit is the Ethereum precedent). The claimable string is *economically protected soft confirmation*, never "fast finality". Route C (Hyperliquid/HyperBFT class) is a different product thesis: choosing it forfeits the no-independent-L2-consensus claim.

**Adopted shape — two confirmation lanes, not two consensus layers:**

- *Fast lane (~100 ms target):* order submission, execution, soft blocks, optional bonded preconfirmation from competitive providers.
- *Settlement lane (CKB cadence):* canonical ordering, O1 DA publication, validity proofs, custody.

Fast-lane state derives from L1 origins at a chosen CKB confirmation depth, so single-confirmation L1 events do not churn high-frequency state; deposits and cross-layer messages pay a latency cost entering the fast environment (OP Stack's unsafe/safe/finalized separation is the precedent). Buffering reduces, never eliminates, either rollback source.

**Research questions — Fast-Head Continuity & Preconfirmation Economics:**

1. How does a professional builder offer a unified fast soft history without exclusive canonical authority?
2. How are conflicting preconfirmations detected and resolved?
3. Under CKB reorg or competing batches, how many confirmed transactions are replayed, revoked or re-executed?
4. Which risks do bonds and compensation cover — and which are exogenous PoW risk no bond should underwrite?
5. Which operations may consume soft state, and which must await CKB settlement?

The priority inbox remains independent: refusing a fast service never costs a user canonical inclusion.

**Refinement (9 October 2026, after MegaETH/Sonic review) — the liability dichotomy answers RQ4.** A preconfirmer's breach splits into two kinds with different provability:

- **Equivocation** — the same preconfirmer signs conflicting commitments for the same history slot. Objectively provable from the signatures themselves; bondable and slashable.
- **Non-inclusion** — a signed history is not adopted by CKB. Not attributable: it may equally reflect honest competition, miner choice, or a PoW reorg, none within the preconfirmer's control. Bonding it would bleed honest builders' collateral for L1 outcomes they cannot govern.

Slashing rules that confuse the two are the primary design hazard of a bonded preconfirmation market. MegaETH supplies the borrowable mechanics: an on-chain signer registry with rotation history (any RPC can verify a mini-block's provenance); the **mini-block ≠ EVM block** two-tier — a live production precedent for the block model of DAG note §1.4/§9 (~10 ms signed mini-blocks without full headers beneath ~1 s tooling-compatible EVM blocks); and realtime API surfaces (subscriptions; `eth_callAfter`-style nonce-gated simulation), adopted only under §9's receipt-honesty labelling. Sonic supplies a boundary, not a module: its aBFT finality is its own consensus, and its exits to Ethereum still traverse gateway confirmations — *consensus speed is not settlement speed* — and Route C remains rejected.

**Committee boundary (explicit).** A signer registry is not a committee so long as no closed set's consent is *necessary* for canonical ordering or settlement validity — the test Godwoken's PoA and Tendermint fail and a preconfirmation market passes, because spec §4.2 already forbids any signature from purchasing the right to propose. Registry semantics are therefore fixed: **bond-to-enter or verification-only** — a roster of currently bonded preconfirmers or a key-rotation record that any RPC may consult; never a licence, never administered as a choke point. The transplant deliberately drops MegaETH's single-sequencer exclusivity (§8's providers are plural and competitive); the residual committee risk is de facto dominance, governed by the claims discipline of §3 rather than by pretending the market cannot concentrate.

**The product question, restated as risk tiers.** Whether users may build on CKB-revocable state is not binary: swap-class flows accept labelled soft-state risk (§9: rollback stated); high-value operations — CLOB positions, leverage, real-time liquidation — require one of canonical-gated execution, bonded preconfirmation with compensation (never irreversibility, per the no-next-proposer caveat), or eventually the Pulse domain for order-flow-heavy applications. This refines RQ5 without closing it.

**Product fork (OPEN — maintainer decision):** a general EVM DeFi L2, or a chain with Hyperliquid-grade ordering determinism as a hard requirement? The two lanes keep the first open while researching the second; Route C is a thesis change, not an upgrade. Recorded recommendation: begin as the general L2 with Route B research — the ecosystem's demonstrated demand is AMM/lending/stablecoin class; B is additive and reversible; and C, if ever needed, is more honestly a separate product than a Tactus upgrade.

## 9. User-experience posture — what Tactus O1 must feel like (decision, 9 October 2026)

**Four pillars.** Faster EVM interaction (not merely shorter block time); proof-driven withdrawal replacing challenge-period waiting (days-class optimistic windows become proof-generation plus L1-confirmation latency — with the honest caveat that a congested prover queue can still delay); near-native Ethereum wallet and developer experience (Godwoken v1 did support direct Ethereum RPC after dropping its Web3-provider plugin — its friction was semantic, not absence; Tactus's target is deeper: standard addresses, receipts and tooling under pinned Reth/revm plus the differential suite); and independent recovery and exit after operator disappearance. The fourth is the generational difference: Godwoken's sunset left remaining withdrawals dependent on contacting its maintainers — *the superior UX is not needing to believe the team will keep running.*

**Honest risks, recorded before they bite.**

1. *Soft-confirmation consistency.* Godwoken's privileged producer supplied a relatively uniform fast soft history; permissionless builders compete, and CKB's eventual choice may re-execute, delay or discard a soft-confirmed transaction (Taiko's based-preconfirmation research records the same fork-and-rollback surface). Decentralised ordering may therefore be **less stable in fast confirmation than its predecessor** — the trade of §4 and §8. Mitigations: Fast-Head Continuity research, optional bonded preconfirmation; never disguise the risk.
2. *Gas honesty.* Godwoken was already cheap (2022 estimates: ~1 cent per ERC-20 transfer). Tactus adds proving, DA publication, anchoring and verification cost; "cheaper than Godwoken" is not claimable before G4 — compression, proving amortisation and efficient L1 contracts must earn it.

**Receipt honesty (RPC design requirement).** `eth_getTransactionReceipt` must never present a speculative receipt as an irreversible one; wallets and frontends expose **Pending / Soft / Canonical / Proven** explicitly, matching spec §2's four levels. Shortening perceived latency by faking finality converts speed into betrayed trust.

**Day-0 UX acceptance rows.**

| User action | Product target |
|---|---|
| Connect wallet | ordinary Ethereum wallet; no special provider |
| Send ERC-20 | fast feedback; no CKB wait |
| Swap | 100 ms–1 s soft execution target, rollback risk stated |
| Query a transaction | Pending / Soft / Canonical / Proven shown |
| High-value withdrawal | proof-backed; no optimistic challenge period |
| Builder outage | switch gateway/builder; inclusion never depends on the original operator |
| Asset history | independently reconstructible from CKB DA |
| Network sunset | verifiable self-service exit — never "contact the team" |

**Priority order (ranked above extreme TPS):** 1 Fast-Head Continuity; 2 fast proof-backed withdrawal; 3 full Ethereum tooling compatibility; 4 independent recovery and exit; 5 low, predictable gas — subsidised fees never masquerade as protocol efficiency.

**Product goal, restated:** *Fast enough to trade, reliable enough to build on, and independently recoverable without its original operators.*
