# Experiment A acceptance audit

This matrix preserves the full requirements of [architecture §14](TACTUS_O1_ARCHITECTURE_SPEC_v0.2.6.md#14-experiment-a--competing-priority-inbox-designs)
and the [experiment design](EXPERIMENT_A_DESIGN.md). A reproduced failure can finish
an experiment control while contradicting a production gate. G2 remains OPEN;
A1 and live-dependency A3 remain rejected production strategies, and standalone A2
challenge markers do not force processing.

| Requirement | Current authoritative evidence | What remains before the full requirement passes |
|---|---|---|
| §14.1–14.2, fee conflict versus stale identity | [Matched admission](ADMISSION_CONTENTION_REPORT.md): five authenticated arms, 120 races/version, independent fee funding, delays 0/1/3/6, fees 1/2/10, node-confirmed sizes and exact stale/pool outcomes | [Current CKB byte-fee control](ADMISSION_FEE_DENSITY_REPORT.md) adds 120 races per absolute/serialized-byte policy with unchanged classifications. Cycle-weighted pricing, different miner/txpool policies, repeated ordinary wallet traffic, proposer identity changes and inclusion assumptions need qualification. |
| §14.1, malformed/duplicate/valid messages | [A2](A2_OBLIGATION_REPORT.md), [A3](A3_SEALED_REPORT.md), [execution](EXECUTION_DIFFERENTIAL_REPORT.md) and [network](NETWORK_REORG_REPORT.md) retained signed inputs and state roots | Complete production envelope, proof-bound total processing and mixed sustained useful EVM workload remain absent. |
| §14.3, individual A2 authenticity and mature challenge | Typed message/lock transitions, exact prefix inclusion, pending records and script-specific negative controls in [A2 evidence](A2_OBLIGATION_REPORT.md) | Processing-consumption proof, all liability economics, proposer changes and actual forced processing are not implemented. |
| §14.3, carry-forward starvation | Real omission/challenge counterexample records `forced_inclusion: FAILED`; nominal batch progress does not imply obligation fulfillment | A selected production mechanism must prevent indefinite deferral under explicit admission/miner assumptions. Rejecting the failed A2 construction does not automatically qualify A3 admission. |
| §14.3, consumed but unproven | Immutable pending records persist; duplicate ordinary/priority input receives deterministic rejection | [Actual A3 fulfillment](SEALED_PROOF_SETTLEMENT_REPORT.md) now proves four duties (including one deterministic rejection), with canonical rollback and same-proof recovery. A2-specific fulfillment, another independent prover/operator, expiry and safe asset exit remain unqualified. |
| §14.3–14.4, sustained backlog and stability recovery | [Sustained A3 load](SUSTAINED_LOAD_REPORT.md) measures offchain backlog separately from admitted obligations, repeated seals, matched L1/L2/L3 arrivals and stopped-arrival drain | Queue-level production pricing/load shedding, A2 sustained overdue subsets, admission deadline policy and longer workload qualification remain open. A full active queue is not a successful admission. |
| §14.4, delay distributions and deadline violations | Per-message heights/batch numbers; p50/p95/p99 independently derived for complete finite workloads including a separately identified drain | No wall-clock guarantee or production admission deadline exists. A finite canonical-batch bound is conditional on canonical progress and is not validity settlement. |
| §14.4, economics | Exact raw transaction capacities, fees, node-confirmed bytes, estimated cycles, retained data/snapshot capacity and abandoned-candidate work | Serialized-byte fee matching is measured in the [current admission control](ADMISSION_FEE_DENSITY_REPORT.md). Production affordability thresholds, resource-weighted and ordinary-wallet fee competition, proving cost, funded liability/bonds and long-lived storage policy remain unqualified. |
| §14.6, dual throughput and construction intervals | [Sustained comparison](SUSTAINED_LOAD_REPORT.md): K=1/2/4, L1/L2/L3, one/three four-block quanta, 32 windows, live-dependency diagnostic versus mandatory sealed control | Deterministic offered schedules are not a Poisson survival estimate; independent arrivals, signed-wallet competition, dominant/public miner policies and network propagation still need qualification. |
| §14.6, sealed switching and retained references | On-chain all-lane seal, immutable snapshots, mandatory FIFO prefixes, full queues, snapshot-switch attacks and cold canonical recovery | [Repeated seal contention](SEAL_CONTENTION_REPORT.md) now measures the finite valid-append budget and rejects further full-queue churn across two epochs. Eventual miner inclusion, admission fairness and reorg-restored budgets remain conditional; favorable serialization of seals in the sustained-load suite is not the supporting evidence. |
| §14.1/§14.4, reorg/checkpoint behavior | [Cold recovery](SEALED_RECOVERY_REPORT.md), [two-node partition](NETWORK_REORG_REPORT.md) and [chain-derived genesis](GENESIS_ALLOCATION_REPORT.md) | [Canonical A3 proof inputs, settled duties and same-proof P2P recovery](SEALED_PROOF_SETTLEMENT_REPORT.md) now pass, alongside [abrupt primary restart](OBLIGATION_RESTART_REPORT.md). Repeated/unplanned faults, durable production checkpoints and archival retrieval remain incomplete. |
| §14.5, positive production selection | Evidence rejects unsafe constructions and supports only conditional A3 publication after successful admission and sealing | A safe, recoverable, affordable, force-progressing selection has not been demonstrated. Revise the inclusion protocol where admission/sealing can still starve. |

The [SeqCommit comparison](SEQCOMMIT_COMPARISON.md) addresses the related-design
requirement with primary sources and a pinned KIP revision. It identifies which
completeness properties come from native consensus and which the current CKB
scripts still have to establish; it imports no unmeasured performance or liveness
claim.

A full production path also requires Experiments B/C and G1–G9, including production qualification beyond the now measured execution proofs
verified on CKB, custody conservation, independent exit evidence,
ordinary Ethereum developer tooling, operational fault qualification and enforced
upgrade boundaries. The status in [production readiness](PRODUCTION_READINESS.md)
is authoritative; none of those requirements is removed by this measurement work.
