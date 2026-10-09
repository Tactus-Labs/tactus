# SeqCommit and the measured CKB admission designs

Reviewed 10 October 2026 for [Experiment A §9](EXPERIMENT_A_DESIGN.md#9-related-designs--kaspa-kip-21-lanes-and-seqcommit).
This comparison concerns authentication boundaries, not cross-chain performance.

KIP-21 specifies consensus-maintained lane tips, an active-lane sparse Merkle root
and selected-parent recursion under one header commitment. Its lane key is the
transaction subnetwork ID; its proving objective scales with relevant lane
activity. It also specifies inactivity purge and bootstrap/reorg requirements.
The reviewed document labels itself Active and describes a consensus-changing
hard fork; that label alone is not evidence of activation on a particular network.
[Pinned KIP-21 source](https://github.com/kaspanet/kips/blob/e4ae2332117b5cb68bd6188e065ef885b6d17939/kip-0021.md).

The Based Apps overview still describes construction work. The vProgs proposal
separately assigns validity proofs responsibility for state progression through
covenants and discusses independent provers and witness availability. Sequencing
and correct execution are different responsibilities in those sources too.
[Based Apps](https://docs.kaspa.org/programmability/based-apps),
[vProgs proposal](https://research.kas.pa/t/concrete-proposal-for-a-synchronously-composable-verifiable-programs-architecture/387).

| Boundary | Native lane commitment | Tactus O1 evidence |
|---|---|---|
| Authority for the lane sequence | Consensus updates the committed lane state | A1 and A3 state transitions authenticate only the Cell history enforced by their scripts. |
| Independent publication | Consensus lane classification supplies the sequence scope | A2 creates independent authenticated messages; the measured anchor does not enforce completeness of the outstanding set. |
| Reference freshness | Historical committed sequence anchors support lane proofs | Mutable A3 Cell dependencies become unresolvable after updates. Immutable snapshots survive active-lane churn. |
| Progress after admission | Sequencing authentication alone is not execution settlement | A3 binds mandatory prefixes to successful batch progression; proving and settled fulfillment remain absent. |
| Retention | Purge/bootstrap rules are part of the native design | Current CKB snapshot, data and pending-record Cells remain permanently retained, with explicit capacity cost. |

The CKB column is derived from the implemented
[anchor](../crates/tactus-o1-anchor-script/src/lib.rs),
[priority](../crates/tactus-o1-priority-script/src/lib.rs) and
[sealed gate](../crates/tactus-o1-sealed-script/src/lib.rs), plus
[admission](ADMISSION_CONTENTION_REPORT.md) and
[sustained-load](SUSTAINED_LOAD_REPORT.md) evidence. The following implications are
our engineering assessment, not claims made by Kaspa about CKB:

1. An indexer-provided list of independent Message Cells cannot become an
   authenticated complete priority set merely by being hashed. The verifier needs
   a rule proving that no eligible message was omitted.
2. A CKB script construction can authenticate a shared/sharded accumulator and
   its immutable snapshots, as the current gate does. Its producers still need
   admission and seal inclusion, and its proof must bind eventual fulfillment.
3. A construction proving a complete canonical CKB block range could, in
   principle, derive eligible independent ingress in canonical order. It would
   need authenticated headers, complete transaction commitments, bounded range
   progress, reorg rules and economical proving. That path is not implemented or
   benchmarked here; it cannot inherit KIP-21's lane-local proving cost by analogy.
4. A CKB-native lane commitment would instead require a separately specified,
   reviewed and activated consensus change. An application script deployment
   cannot assume that change has happened.

No native sequencing commitment by itself closes Tactus's admission fairness,
proof liveness, custody or exit gates. Keep this research comparison separate from
production protocol selection and retain the exact failed controls.

Source pin: KIP repository commit `e4ae2332117b5cb68bd6188e065ef885b6d17939`;
`kip-0021.md` SHA-256 `ce1606e74e20a9652a0c67ddd349d84b8b2ab8265df5a1d10b27d35f7a95cf26`.
The two overview/proposal URLs were read on the review date; they are not frozen
protocol dependencies.
