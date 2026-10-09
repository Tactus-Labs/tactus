# Tactus O1 — DAG Acceleration and External Data Availability: A Research Note

**Status:** research note — a reasoned position, **not** implementation evidence and not a gate pass (G1–G9 remain OPEN per spec §13)
**Date:** 9 October 2026
**Baseline:** [TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md](TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md)
**Question addressed:** at which layers may directed-acyclic-graph (DAG) constructions accelerate Tactus O1, and how do such constructions coexist with the O2 external-DA domain?
**Amended:** 9 October 2026 after external review — §2's arithmetic claim weakened to an expectation; §1.2/§5.2 corrected (multi-microbatch manifests are **not** in the frozen baseline; the reservation question is decided by the L2 block model); §1.3's churn argument scoped to live-head constructions; §1.4 added (the prior question).

---

## 0. Summary of the position

1. **DAG scheduling inside the execution engine** carries no protocol consequence — the executor's output remains a deterministic total order — but it is not engineering-free, and host-side speed does not raise settled throughput whilst proving is the slower term (§1.1).
2. **DAG batch construction** is admissible, but what it costs to enable is decided by the **L2 block model**, not by DAG preference: under builder-internal linearisation there is nothing to reserve; under multi-EVM-block semantics the block-sequence commitment must be specified before genesis (§1.2, §1.4).
3. **DAG canonical ordering on CKB** is rejected for the baseline: the sufficient reason is the absence of demonstrated benefit against certain new consensus rules and verification complexity — not a claim that every DAG construction inherits A3′'s churn failure (§1.3).
4. **From prior art, borrow engines, not trust models.** MegaETH's execution engine belongs inside the permissionless builder role; Sonic's consensus DAG belongs to a thesis Tactus O1 exists to reject.
5. **External DA does not change the verdict.** Anchors are O(1) in transaction count, so ordering is *expected* to remain a minor cost term in the O2 domain — subject to measured anchor cadence and maximum provable batch size (§2); the expected O2 ceilings are DA throughput, proving cadence and recovery semantics. A canonical DAG would be *more* dangerous in O2, not less, because it multiplies ordering ambiguity precisely where recovery guarantees are weakest.
6. **Deployment timing resolves into interface headroom.** Reserve the manifest's multi-microbatch capability and the determinism harness on day 0; neither permitted tier ever requires a fork to activate (§5). StarkEx-style volition is the counter-case: day 0 reserves the already-specified domain boundary, never the dual-tree stitching (§5.5).

## 1. The three layers of "DAG-isation"

### 1.1 Execution-engine parallelism — permitted, no protocol consequence

The executor may schedule independent transactions in parallel using read-write-set conflict detection, provided the visible result is bit-identical to sequential execution under the canonical order. EVM constraints (per-account nonces, state conflicts) are serialised by the scheduler; deterministic replay is unaffected. This is an implementation tier, not an architecture decision.

The correct references are **Monad** (optimistic parallel execution with deferred execution) and **Block-STM** (Aptos) — the canonical treatments of VM-compatible parallel execution.

Protocol-free is not engineering-free. Optimistic parallel execution buys its speed with: dynamic read-write-set tracking; conflict detection with re-execution; multi-version state stores and their memory pressure; measurable degradation on conflict-heavy DeFi workloads; and non-trivial integration with zkVM witness generation. Nor does a host-side speedup by itself raise sustained *settled* throughput: if witness generation or proving is the slower term, accelerating builders merely widens an idle surplus. Scheduler priorities therefore come from segmented profiling of execution, witness generation and proving — before the scheduler is written, not after.

### 1.2 Batch-construction DAG — admissible; what it costs depends on the block model

*Corrected on review:* an earlier draft of this note claimed the frozen baseline already permits multi-microbatch manifests. It does not — `BatchManifest` (spec §3.2) commits to an ordered transaction sequence, and the word "microbatch" appears nowhere in the frozen specification; that language lived in pre-freeze drafts. What is actually at stake is a design fork with two, quite different, shapes:

- **Scheme A — builder-internal linearisation.** Builders may schedule, pre-execute and conflict-detect in parallel internally, then submit an ordinary linear transaction sequence. Nothing is reserved at genesis, and enabling parallel scheduling later changes no protocol object at all.
- **Scheme B — each microbatch is an EVM block.** Blocks carry their own `NUMBER`, `TIMESTAMP`, `BASEFEE`, gas limit, receipts root and hash — values that alter EVM execution results — so the microbatch boundary becomes consensus data, and the protocol must define a **Block Sequence Commitment** and per-block environment derivation rules.

Both schemes exploit parallel batch production; they are entirely different architecture decisions. Which reservation day 0 must make is decided by this fork (§5.2), and the fork itself is settled by the prior question of §1.4.

### 1.3 Canonical-ordering DAG — rejected for the baseline

Replacing the single linear `OrderingHead` succession with a DAG of parallel anchors would require a deterministic canonical-order rule over multiple live heads. Three objections:

- **It optimises a constraint expected to be minor.** See §2 — an expectation to be confirmed by measurement, not a proven bound.
- **It spends the property that pays for everything else.** Single-consumption linearisation is the entire reason a CKB script can verify canonical succession cheaply. A DAG order rule either migrates that complexity into scripts or falls back to an off-chain indexer — the negative-knowledge hazard recorded as W-11, which the specification prohibits.
- **Live-head variants multiply the churn surface.** Experiment A's simulation tier observed *live-head* references collapsing under adversarial churn (survival 0.01 under L3) whilst sealed, immutable references were churn-immune. This is evidence against mutable-live-head constructions specifically, not against every conceivable DAG ordering: a scheme of immutable nodes with deferred linearisation need not share that exact failure mode — though it must still solve unique canonical succession, contiguous state proofs, and reorganisation handling.

The sufficient reason for rejection is simpler than any of the three and does not require the churn result to generalise: **no construction has demonstrated benefit over the single `OrderingHead`, whilst any canonical DAG certainly adds consensus rules and verification complexity.** The burden of proof sits with the proposal, per §6.

### 1.4 The prior question — the L2 block model (anchor ↔ EVM-block cardinality)

Review of this note surfaced a gap that precedes every DAG decision: the frozen baseline requires "deterministic block context" (spec §7) but does not settle how many EVM execution blocks one CKB anchor may authenticate, nor how per-block environment values (`NUMBER`, `TIMESTAMP`, `BASEFEE`, gas limit, receipts root, `BLOCKHASH`) are derived. The cardinality determines the shape of the product:

- **One anchor, one block:** canonical L2 block cadence is coupled to the anchor cadence; fast blocks remain soft confirmations only.
- **One anchor, several strictly ordered blocks:** L2 block production separates from CKB anchor cadence — the foundation for a fast RPC experience under based settlement, and a determinant of proving cost and maximum batch size.

Four questions require their own specification before the serial EVM baseline is implemented (direction adopted and recorded in §9: multiple strictly ordered EVM blocks per anchor, no DAG consensus between them):

1. How many EVM blocks may one anchor commit to?
2. How is each block's number, timestamp, gas limit, base fee and hash derived?
3. Are blocks produced speculatively by builders and later acknowledged by CKB ordering, or fixed only after ordering?
4. May one validity proof cover multiple blocks, attesting every intermediate header?

This decision is independent of DAG scheduling and should be settled first; the DAG scheduler follows, never leads.

**Elevation (9 October 2026, on review).** The block model is promoted from this note to **Architecture Spec v0.2.6** as the P0 work package — these are consensus-critical execution semantics, not an optimisation. The frozen v0.2.5 objects do not yet carry the adopted decision:

| Gap in v0.2.5 | Required rule |
|---|---|
| `OrderingHead` records batch number, no EVM-block frontier | a new anchor continues from the previous anchor's last EVM block number |
| `BatchManifest` binds only a transaction-sequence root | bind ordered EVM block headers and block boundaries |
| 100 ms production | `TIMESTAMP` monotonicity and tolerance — second-granular EVM timestamps mean many fast blocks share one timestamp, moving TWAP, interest and block-number-dependent logic |
| multi-block gas | per-block `GASLIMIT`, `BASEFEE` and EIP-1559 behaviour |
| `BLOCKHASH` | deterministic query rules over canonical and speculative predecessors |
| proof coverage | a validity proof spans consecutive intervals, attesting every per-block header and state transition |

Completion requires normative test vectors. Until the block-model content lands in a spec revision, "multi-block per anchor" remains a direction, not a specification (v0.2.6 currently carries only editorial changes).

## 2. The arithmetic that decides the question

An anchor is **O(1) in transaction count**: it carries commitments, not transaction bodies. Suppose a domain sustaining 5,000 transactions per second with one anchor per CKB block (~10 seconds): each batch covers ~50,000 transactions, yet the on-chain anchor remains a fixed-size commitment. From this, an earlier draft concluded that the ordering frontier "remains non-binding well into the tens of thousands of TPS". That conclusion was stronger than the argument: sustainable settled throughput is bounded by

> TPS_settled ≤ min( T_DA, T_execution, T_proving, f_anchor · N_batch_max )

and neither factor of the last term is established. `N_batch_max` is finite — bounded by the provable resource envelope (gas limits, witness construction, batch-format rules, spec §3.3) — and `f_anchor`, the sustainable anchor cadence under RFC 0020's proposal window (w_close = 2, w_far = 10), is a quantity Experiment A's devnet tier exists to measure, not to assume.

The honest statement, and the one this note now makes: **the ordering frontier is expected to remain relatively inexpensive, but sustainable anchor cadence and maximum provable batch size must be measured before ordering can be excluded as a throughput bottleneck.** Until those measurements exist, treating ordering as settled would be exactly the kind of unearned G4 conclusion the specification's claim discipline prohibits.

Optimising an unmeasured term by dismantling the mechanism that verifies canonicity remains a poor trade — but it is now stated as a priced expectation, not a proof.

## 3. Prior art, sorted by borrowability

### 3.1 Monad / Block-STM — borrow wholeheartedly

Engine-tier parallel execution; no trust-model entanglement. Relevant to the speculative head and the canonical executor alike.

### 3.2 MegaETH — borrow the engine, not the trust model

MegaETH demonstrates the value of a hyper-optimised sequencer: in-memory execution, ~10 ms streaming blocks, sub-second soft confirmations. Its trust model — a designated sequencer — is precisely what the based kernel forbids. The correct transplantation is to install that engine inside the **professional builder** role: permissionless, competitive, bypassable. Millisecond experience is retained; the "no conductor" thesis is not surrendered.

### 3.3 Sonic — protocol-irrelevant

Sonic's DAG lives at the consensus layer (an event DAG stabilising into aBFT finality): it replaces one consensus authority with another. Tactus O1's thesis is the removal of the independent L2 consensus authority altogether; there is nothing here to borrow beyond general engine hygiene.

### 3.4 Kaspa Based Apps / vProgs — study the lanes, not the DAG

Kaspa's verifiable-programs direction proposes sovereign per-program state, shared L1 sequencing, and an account-scoped computation DAG with proof stitching. Maturity as of 9 October 2026: the Toccata programmable-UTXO layer is live; Based Apps are in development; full cross-program synchronous composition remains a future direction — a research proposal, not a production-verified scheme.

- **Worth borrowing:** L1-native **application lanes and authenticated SeqCommit (KIP-21)** — a consensus-level authenticated entry point for an application's own transaction sequence, directly relevant to the priority-inclusion problem of Experiment A (see its §9); and account-scoped dependency scheduling with local proving, consistent with the engine tier of §1.1.
- **Not changed by it:** DA bandwidth — data compression and computation sharding are not availability capacity, and Kaspa's based apps likewise publish user operations as L1 lane transactions; external-DA withholding and exit risk — state isolation does not restore withheld data, and Kaspa's own research records the pruning/withholding dispute as open; soft-confirmation irreversibility — 10 blocks/s is still PoW, with its own reorg and confirmation policy; and Tactus O1's product identity — reorganising Tactus O1 into many sovereign vProgs would forfeit the standard-rollup property that ordinary contracts deploy into one synchronously composable EVM environment.
- **Verdict:** an R&D reference, not a dependency. The architecture stands; the lanes are the research thread.

## 4. Coexistence with the O2 external-DA domain

### 4.1 Layer × domain interaction

| Layer | O1 (CKB DA) | O2 (external DA) |
|---|---|---|
| Engine parallelism (§1.1) | Identical; orthogonal to DA | Identical |
| Microbatch DAG → manifest (§1.2) | Manifest data published to CKB under a witness-committed envelope | Identical construction; published bytes travel to the external DA under the `DataPublicationEnvelope`; ordering identity unchanged |
| Canonical DAG (§1.3) | Rejected | Rejected with emphasis (§4.3) |
| Canonical succession | One linear `OrderingHead` per domain | One linear `OrderingHead` per domain — `da_policy_id` is genesis-bound |

### 4.2 Does cheap DA make ordering binding? No.

Lifting the data ceiling is the very circumstance in which one might expect anchor cadence to become the constraint. It does not, for the reason of §2: batches grow, anchors do not. In O2 the binding ceilings are (a) sustained external-DA throughput, (b) sustained proving throughput (the settled-TPS metric), and (c) reconstruction and exit semantics under the domain's declared recovery assumptions. None of these is relieved by DAG-ising canonical ordering.

### 4.3 Why a canonical DAG is worse in O2, not merely unhelpful

1. **Recovery surface.** O2's weak flank is recovery under DA failure — the S0→S1→S2→S3 cascade argument of spec §10.2. A canonical DAG enlarges the space of "which prefix is canonical" precisely where completeness proofs are hardest and where every ordering ambiguity must be resolved without an indexer.
2. **Churn compounding.** Every additional live canonical head multiplies the live references a dependent transaction must carry; dependency-invalidation exposure grows with the referenced update rate, compounding the churn measured in Experiment A.
3. **Double payment at the settlement boundary.** A validity proof binds to a contiguous interval over an authenticated accumulator (spec §6). A canonical DAG therefore requires a deterministic linearisation rule anyway — defined, authenticated and verified — so the DAG collapses back into a line at settlement, having paid the complexity twice.

### 4.4 Adoption ladder

1. Engine in-memory/parallel execution (implementation tier; priority set by the profiling discipline of §1.1).
2. O2 domain deployment where sustained throughput justifies its declared assumptions.
3. Multi-pipeline batch construction (semantics per §1.2/§1.4).

The ordering of the first two rungs is demand- and G4-driven, not a fixed development sequence. The canonical ordering line is not on the ladder, and a StarkEx-style volition stitching the two domains is deliberately absent from it as well (§5.5). (Deployment timing and fork exposure: §5.)

## 5. Deployment timing — day-0 capability, not day-0 activation

The question "should DAG-isation ship on day 0, or arrive with a later hard fork?" partly dissolves under inspection: **neither permitted tier requires a fork, because neither touches consensus-critical state.** The genuine day-0 decision is not activation but **interface headroom** — and for a validity rollup, "hard fork" properly denotes the G8 upgrade machinery (new execution rules, new verification keys, timelocks, exit-preserving windows), which makes consensus-critical surface something to budget, not to improvise.

### 5.1 Engine parallelism — no fork ever; gate on determinism, not on a release schedule

Engine-tier parallelism is invisible to the protocol: the executor's output must remain bit-identical to sequential execution under the canonical order, so the scheduler may be exchanged at any time, like a database engine beneath a node. Nothing about it is day-0-critical — sequential execution comfortably serves O1's DA-bounded throughput, and premature parallelism purchases nondeterminism risk against unneeded speed. What MUST exist on day 0 is its guardrail: the determinism contract and the differential test harness (spec §7), which convert engine DAG-isation into ordinary engineering, schedulable whenever devnet and proving workloads justify it.

### 5.2 Batch-construction DAG — reserve what the block model requires, not a blanket schema

*Corrected on review:* the earlier claim that the frozen specification already defines a multi-microbatch manifest capability was wrong (§1.2). The reservation question is decided by the block-model fork, not by DAG preference:

- **Scheme A (builder-internal linearisation):** nothing to reserve. Builders submit ordinary linear sequences; parallel scheduling is invisible to the protocol and may be adopted at any time.
- **Scheme B (each microbatch an EVM block):** the consensus-critical objects are a **Block Sequence Commitment** and the per-block environment-derivation rules of §1.4. These must be specified *before genesis*: afterwards they can only change through `execution_rules_hash`, a new proving programme and verification keys, and the full G8 machinery.

If Scheme B is adopted — the direction §1.4 recommends — day 0 carries the block-sequence semantics and builders emit single-block manifests until multiple pipelines mature; activating multi-block manifests later then requires no fork, because the on-chain object shape never changes. Support, however, means **implemented and tested** — ordering, block-header commitments, execution environment and proof binding — never a reserved field alone (§8). The affordable mistake, if there must be one, is activating late; the unaffordable one is specifying the block model after genesis. The self-restraint stands: reserve exactly what the block-model specification states and nothing more.

### 5.3 Canonical DAG — no timing question exists

§1.3 stands: not on day 0, and not by any later fork, unless the reopen conditions of §6 are met by measurement rather than preference.

### 5.4 Summary

| Tier | Day 0 | Later | Fork required |
|---|---|---|---|
| Engine parallelism | determinism contract + differential harness | parallel scheduler, whenever workloads justify | never |
| Microbatch DAG (Scheme B) | block-sequence semantics + env derivation specified (§1.4); emit single-block manifests | emit multi-block manifests | never |
| Canonical DAG | — | — | not applicable; rejected (§1.3) |
| StarkEx-style volition | domain-separation discipline only (already specified) | stitch two validated domains, after its own isolation-and-transfer specification | G8-class upgrade by design; never a genesis retrofit (§5.5) |

### 5.5 StarkEx-style volition — the counter-case: neither capability nor activation on day 0

The reserve-on-day-0 rule of §5.2 (as corrected) turns on the block model; StarkEx-style volition — per-mode dual state trees with explicit transfers between a rollup tree and a validium tree — fails even the corrected test on every axis, and the specification already records the verdict: "A future StarkEx-style Volition mechanism needs its own state-isolation and transfer specification; this is not part of this specification" (spec §10.2).

| Test from §5.2 | Microbatch DAG | StarkEx-style volition |
|---|---|---|
| Specified in the baseline? | No — absent from the frozen baseline (§1.2); specifiable pre-genesis as block-model content (§1.4) | No — deferred pending its own specification |
| Reservation cost at genesis | Negligible — N = 1 and N > 1 are the same on-chain object | Real — dual trees alter state commitments, proof public inputs, vault scripts and exit rules from day one |
| Cost of omitting the reservation | A format change, hence the full G8 machinery | Absent — the chosen O2 architecture (separate security domains, spec §10.2) means the O1 domain is never retrofitted; volition, if ever, is a third construct stitching two already-validated domains |
| Prerequisite | Multiple builders or pipelines | O1 validated, O2 validated under its declared assumptions, then a cross-domain transfer specification |

What day 0 *does* reserve for a future volition is the **domain boundary**, and v0.2.5 already reserves it: genesis-bound `da_policy_id` per domain, independent state roots and `withdrawal_root`, and conservation-checked explicit proven transfers. Reserve the boundary, not the stitching — the boundary is specified and cheap, whilst the stitching is unvalidated, and pre-heating it would bake S0→S1→S2→S3-class recovery subtleties into genesis.

Precedent supports the ordering: StarkEx deployments (dYdX, ImmutableX) composed modes per running application rather than launching with volition, and zkSync's announced zkPorter remained unrealised for years. Volition's correct appearance is after both domains stand — as a G8-class upgrade carrying its own adversarial testing, not as a genesis configuration.

## 6. What would reopen this question

This position is falsifiable. Reconsider §1.3 if any of the following is demonstrated:

- **Measured anchor cadence becomes binding** at realistic batch sizes under actual CKB block intervals and proposal-window behaviour (contradicting the O(1) arithmetic of §2).
- **A verifiable canonical-order rule for a DAG of cells** is constructed that requires neither an indexer nor heavier script verification than single-consumption succession — and survives an adversarial test programme comparable to Experiment A, including dependency churn.
- **CKB protocol changes** materially alter anchor economics.

## 7. Interpretation boundaries

This note reasons from the frozen v0.2.5 baseline and the simulation tier of Experiment A. It contains no measured devnet evidence, makes no performance claim, and passes no gate. Where it disagrees with future measurements, the measurements win.

## 8. Decision record — accepted posture (9 October 2026)

Following external review and maintainer decision, the accepted posture is **correctness-first**: Day 0 closes the loop on based sequencing, ZK settlement, EVM compatibility and permissionless exit; DAG in every form is deferred or rejected.

| Construction | Day 0 | Rationale |
|---|---|---|
| Execution DAG (parallel EVM) | **Not implemented** | Serial revm establishes the correctness baseline; parallel scheduling joins at Phase 1 only if profiling shows execution binding, and must match the serial reference bit-for-bit |
| Microbatch DAG (parallel batching) | **Not implemented** | Builder-internal optimisation; its scheduler complexity is not carried in the first version |
| Canonical-ordering DAG | **Rejected** | Linear CKB `OrderingHead` succession is retained (§1.3) |

**The one boundary to settle — and test — before Day 0:** the multi-EVM-block-per-anchor model. Accepted direction: one CKB anchor commits to multiple consecutive EVM blocks, each retaining its own number, timestamp, gas limit and state-root semantics; CKB governs the canonicity of the whole batch interval; provers attest the contiguous execution history asynchronously. Two disciplines attach: supporting N blocks means implementing and testing their ordering, block-header commitments, execution environment and proof binding — a schema field alone is not support; and multi-block semantics do not require DAG, since serial execution realises them fully — they reserve room for fast L2 block cadence and later acceleration without coupling it to anchor cadence.

**Phased order of work:**

- **Day 0 — correctness-first rollup:** serial revm, linear CKB ordering, validity proofs, priority inbox, CKB DA, basic asset bridge and exits; independent differential execution tests; an explicit EVM block model.
- **Phase 1 — execution and batching optimisation:** execution caches, state access, witness generation, batch compression; Block-STM/Monad-style scheduling only if execution is the measured bottleneck, results identical to the serial reference.
- **Phase 2 — high-throughput extensions:** by measured throughput and cost bottlenecks — O2 external DA (subject to the activation gate of `O2_ACTIVATION_POLICY.md`), further parallel proving, microbatch DAG.

**Fork exposure (confirmed):** off-chain DAG optimisations are ordinary software upgrades whenever their outputs satisfy the existing protocol; changes to block-organisation semantics, consensus encodings or proof statements are protocol upgrades. The artefacts worth freezing early are therefore the **canonical EVM block semantics, the batch commitment format and the proof binding rules** — never a DAG scheduler.

## 9. Block pipeline — adopted architecture (decision, 9 October 2026)

**Model.** *High-frequency speculative blocks, low-frequency CKB anchors, asynchronous validity settlement.* EVM blocks are produced off-chain at a configurable cadence (initially testing the 100 ms–1 s range); runs of consecutive blocks aggregate into one canonical anchor; validity proofs attest whole contiguous intervals. Block-production cadence and anchor cadence are independent parameters — illustratively, a ~10 s work cycle of ~100 blocks, one anchor, one later settlement (illustrative only; CKB imposes no fixed confirmation rhythm). OP Stack's batcher is the operating precedent for batched, compressed L1 publication rather than per-block posting; MegaETH's two-tier granularity — ~10 ms signed mini-blocks beneath ~1 s tooling-compatible EVM blocks — is a live production precedent for the multi-block model of §1.4.

**CKB-native precedent — Godwoken v1.7 and PR #776 (added on review).** The decoupling above is *not* a Tactus O1 novelty: Godwoken solved it on this same L1 in 2022. Its early architecture coupled production to CKB submission (~30–40 s L2 blocks); the July 2022 proposal split producing, syncing and submitting into independent pipelines, and [PR #776](https://github.com/godwokenrises/godwoken/pull/776) implemented a `ProduceSubmitConfirm` state machine — `Local → Submitted → Confirmed`, with `local_limit`/`submitted_limit` backpressure, P2P propagation of unanchored blocks to read nodes, and rollback to the confirmed state on submission failure or CKB inconsistency — reaching ~8 s average testnet block times by v1.7-rc (a testnet observation; neither finality nor a sustained-TPS measurement). Three dispositions follow. **Borrow:** the state machine, P2P local blocks, rollback and backpressure are the reference implementation for the Tactus O1 block pipeline — read and test rather than reinvent; Tactus O1 extends the ladder with `Canonical` (CKB-ordered) and `Proven` (ZK-verified). **Scope the novelty:** Tactus O1's contributions are the permissionless builder set, validity settlement, multi-block-per-anchor and priority inclusion — not asynchronous production itself. **Heed the finality lesson:** when Godwoken's block time shrank, block-count-based challenge windows ceased to match wall time, and `block.number`/`timestamp`-dependent contract semantics (TWAP, interest accrual) shifted — a live demonstration of why §1.4's block model must be fixed at Day 0, *before* any acceleration. And cadence is not throughput: 100 ms soft blocks over Godwoken's 8 s claims no multiple; sustained settled TPS remains unmeasured until G4.

**Commitment is not data availability.** An anchor may carry fixed-size commitments — batch root, block range, data commitment, execution rules — yet hashes alone tell no third party what executed. O1 therefore also publishes sufficient reconstruction data on CKB (compressed transaction sequences, or state differences under demonstrated reconstruction guarantees), possibly merged, compressed or sharded — and never one CKB transaction per EVM block:

| Publication | Fast blocks | Independent recovery | Model |
|---|---|---|---|
| Anchor + full reconstruction data on CKB | supported | from CKB history | **O1 rollup** |
| Anchor + external-DA data | supported | external-DA dependent | O2 validium (deferred; activation gate) |
| Anchor hashes only; data in a builder's private store | supported | none | **not O1 — prohibited** |

High-frequency block production does not by itself motivate external DA; what motivates O2 is the volume of reconstructable data per unit time against CKB's budget (597,000 bytes per block, RFC 0020).

**Atomic anchor–publication rule.** A canonical anchor and its DA publication MUST verify atomically: either the reconstruction data is published within the same CKB transaction whose script the anchor validates, or publication proceeds under an explicit, verifiable shard-completion condition — the anchor is not valid until the full data set is provably on CKB. And as ever (spec §10.1), CKB's raw transaction hash excludes witnesses: the script must verify an explicit batch data commitment, never a raw hash.

**Speculative until published.** Off-chain propagation — P2P gossip, block buffers, object stores, prover witness caches — is operational infrastructure, not protocol-level DA (operational posture: `OPERATIONAL_POSTURE.md`). Until CKB confirms sufficient reconstruction data, produced blocks are speculative. Losing an off-chain cache may force rebuilding unpublished blocks; it can never invalidate the independent recoverability of any state for which O1 guarantees have been claimed.

**Production is not finality.** Two builders may produce competing 101st blocks and soft-confirm them within milliseconds; only CKB ordering makes one canonical, and only an accepted proof settles it. Demanding irreversible canonical ordering at production latency would require preconfirmation trust or economic fast-finality — reintroducing precisely the privileged ordering authority Tactus O1 exists to remove. The four confirmation levels of spec §2 stand.

**Day-0 batch format (draft, not implementation):**

```text
TactusO1Batch {
    parent_batch_commitment: Hash32,
    first_evm_block: u64,
    last_evm_block: u64,
    ordered_block_headers_root: Hash32,
    reconstruction_data_root: Hash32,
    execution_rules_hash: Hash32,
    da_policy_id: Hash32,
}
```

The obligations of §1.4 remain open: per-block `NUMBER`, `TIMESTAMP`, `BASEFEE`, `BLOCKHASH`, state-root and receipts derivation rules — none of which may be assumed to inherit full Ethereum consensus semantics merely by slicing transactions across many blocks. A batch whose reconstruction data exceeds a single CKB block's budget MUST shard under a verifiable completion condition; no anchor may presume its batch fits one CKB transaction.

**Adopted Day-0 parameters:** EVM block time configurable (100 ms–1 s speculative range); anchors aggregate multiple consecutive EVM blocks; transaction ingress via off-chain RPC/P2P; temporary L2 storage operational only; canonical ordering via the CKB OrderingHead; DA via CKB authenticated publication; settlement asynchronous ZK; external DA deferred per the O2 activation gate; priority inclusion per Experiment A.

**Positioning sentence, completed:** *Produce EVM blocks rapidly off-chain, aggregate their commitments into canonical CKB anchors, and publish sufficient reconstruction data to CKB under the same authenticated batch history.* The binding constraints on high-frequency blocks are EVM block semantics, DA publication bandwidth and prover cadence — never how often CKB accepts a hash.
