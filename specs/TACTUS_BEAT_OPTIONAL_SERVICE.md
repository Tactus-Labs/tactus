# Tactus Beat — Optional Fast-Head Service

**Status:** standalone research extract · **optional acceleration service, not mainline** ·
demoted from the priority research direction on 9 October 2026 after design review ·
deployment gated behind G2 · never a component of the Tactus security model.

> This document supersedes the "Route B is the priority research direction" wording in
> `OPERATIONAL_POSTURE.md` §8. Route B survives as **one optional commercial service shape**,
> evaluated as arm D3 of Experiment D. The fast-convergence research priority moves to the
> leaderless Open-DAG direction (Experiment D, arms D0–D2; see `DAG_ACCELERATION_NOTE.md`).

## 0. Position

Tactus Beat is a **Bonded Soft-Head Leasing** service: a periodically elected provider signs
ordered soft frames (~100 ms target) that multiple builders may adopt as a shared speculative
prefix; every builder retains the unconditional right to bypass the provider and submit
canonical batches to CKB directly.

The impossibility boundary it accepts:

> Under CKB PoW — no predictable next proposer, no miner obligation to honour off-chain
> promises — one cannot simultaneously guarantee free builder competition, a network-uniform
> 100 ms soft state, and inclusion of that soft state in canonical history.

Beat therefore offers a *shared, economically accountable speculative head* — never a second
consensus, never finality. Forcing a different trust model here would be subsidising it
(native Kaspa-class speed cannot be bought with bonds); that comparison is recorded as a
known and accepted weakness.

## 1. Protocol sketch

- **Lease election:** open registration (key, CKB bond, endpoints) → sealed registry snapshot →
  deterministic selection seeded by a deep CKB block hash; lease periods are measured in CKB
  blocks, not per frame. Election grants **service rights only, never canonical authority**.
- **SoftFrame** (logical structure, not a final consensus encoding): binds `rollup_id`,
  `lease_id`, `ckb_origin` (reorg binding), `evm_block_number`, `parent_soft_hash`, header and
  body commitments, `execution_rules_hash`, provider signature.
- **Hard constraint — the based boundary:** `APPEND_BATCH` validation never requires the
  provider's signature. Requiring it would re-create licensed sequencing.

**Invariants.**

1. *No canonical privilege* — the provider's signature is never an admission condition.
2. *No implicit finality* — soft frames bind only execution and economics; CKB PoW orders, ZK
   settlement proves.
3. *Independent escape* — refusing or disappearing never removes the independent CKB Priority
   Inbox path (whose forced-inclusion strength is G2's open question).

## 2. Accountability: three different things

> **Slashing makes equivocation costly. Insurance makes failed promises compensable. Neither
> makes a soft block canonical.**

- **Equivocation slashing** (`ProviderBondLock`): two validly signed frames at the same
  `(lease_id, ckb_origin, evm_block_number)` with different certified hashes → bond slashed,
  whistleblower reward. A genuine CKB reorg (changed `ckb_origin`) is *not* equivocation.
- **Canonical-inclusion insurance** — prepaid, per-session `SessionInsuranceCell`
  (`B_locked ≥ Σ B_i`, one max payout per session). Provider cannot control miners, so a losing
  competing anchor is an insured risk, not provable malice. **v0 covers prefix-conflict only**
  (objectively script-verifiable). Deadline-based inclusion insurance requires proving a
  negative (non-inclusion before a deadline) and is explicitly **not claimed**.
- **Claim granularity:** claims require a transaction-membership witness against the insured
  frame's `txs_root` — user-level coverage, not bare prefix-level.

## 3. What Beat does not provide

No guaranteed 100 ms network-wide sync; no guarantee soft history enters canonical history; no
PoW-reorg immunity; no forced inclusion (that remains G2); bonds compensate — they never make a
promise irreversible ("economically protected soft confirmation" is the strongest claimable
string).

## 4. Recorded gaps (design review, 9 October 2026)

1. **Data withholding is outside the failure model.** Signing frames without releasing data is
   not equivocation and is not on-chain provable (negative fact). Remedies are lease-level
   only; Experiment D includes flapping scenarios.
2. **Convergence-by-orderflow:** if most RPC and builders follow one provider, it acquires de
   facto sequencing influence without protocol privilege. Mitigation is competing providers
   plus published coverage/orderflow metrics — measurement, not prohibition.
3. **Election grinding** is bounded only by lease profit vs bond; lease length and bond size
   are experiment parameters.
4. **Builder adoption incentives are structural:** ordering autonomy is MEV revenue; sharing a
   prefix is a concession. Whether adoption forms at all is an empirical question.

## 5. Gate, experiment mapping, kill criteria

- **Deployment gate: G2 (forced inclusion) must pass first.** A valuable provider seat raises
  the stakes of exactly the censorship vector G2 exists to close; insurance compensates, it
  does not repair censorship.
- Experiment D arms (`EXPERIMENT_D`): **D0** pure based (protocol baseline) · **D1** P2P
  dissemination + anti-entropy · **D2** leaderless Open-DAG convergence · **D3** single
  preferred provider — the Beat-shaped arm, doubling as the centralised upper bound.
- **Kill criteria:** if D2 matches D3's user experience without a privileged seat, Beat is not
  built. If neither materially beats D0 on agreement/rollback at honest latency, no fast-head
  layer ships at all.

## 6. Pluggability contract (shared with Open-DAG)

Optional acceleration layers — Beat or Open-DAG — may touch only the dissemination and
speculative-execution planes and MUST NOT: alter `APPEND_BATCH` validation, add signature or
membership admission conditions to canonical sequencing, make any soft prefix mandatory, or
weaken the independent exit path. A Tactus node running pure-based (D0) interoperates fully
with every layer above. Removing the layer must cost UX, never safety.
