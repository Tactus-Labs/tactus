# A2 authenticated obligation experiment

10 October 2026. Both **CKB 0.121.0 and 0.210.0** pass the individual-obligation
boundary suite and reproduce the **failure of challenge-only forced inclusion**.
Full Experiment A is incomplete. G2 and the production gates remain OPEN.

The [v1 comparator](A2_OBLIGATION_V1.md) replaces ordinary untyped message cells
with unique identities, a type-bound protocol lock, consensus-mature challenge
transitions and immutable Included records. It is deliberately not advertised as
a complete forced inbox. The prior [A2 ordinary-cell omission baseline](EXPERIMENT_A_DEVNET_REPORT.md)
remains separate historical evidence.

## Findings on both node versions

| Control | Observed result |
|---|---|
| Admission without consuming or referencing anchor | Authentic message committed |
| Early 12-block relative-since challenge | Rejected by consensus as immature |
| Mature challenge paid by a different account | Committed without admitting user's signature |
| Builder ignores mature challenged message | Three subsequent empty batches committed |
| Builder processes four favorable messages | All four Included; challenged victim still live |
| Ordinary EVM payload inclusion without consuming message | Ethereum transaction succeeds; obligation stays live |
| Later priority inclusion of that same payload | Rejected execution outcome; state unchanged; Included record persists |
| Three rounds admitting 4 and processing 1 | Known workload backlog 3 → 6 → 9 |
| Independent actor drains known backlog in chunks ≤4 | Backlog 9 → 5 → 1 → 0; victim remains separately starved |
| Four 4096-byte messages in one inclusion | Accepted within the input/witness burden |
| Planned reorg removes victim's inclusion | Original challenged message restored; orphan record absent |
| Other actor includes restored message on replacement branch | Accepted once in canonical lineage; repeated Ethereum transaction still rejected |
| Final pending records | All 21 checked live; none asserted Proven |

The workload inventory is maintained by the test driver. It is **not** an
on-chain proof of the complete global pending set. Draining a finite known
workload demonstrates a permissionless processing path, not hostile-builder
liveness. The builder's valid omitted batches and favorable-subset processing
are concrete counterexamples to the challenge-only claim.

Included records bind the original bytes to a batch/block/slot and survive
continued anchor advancement. They do not establish that the EVM slot succeeded
or that a proof exists. The duplicate transaction has `InvalidTransaction`, no
included Ethereum transaction, and unchanged account state. Raw executed blocks
and outcome commitments are preserved in the evidence. There is no proof system
in this experiment and no capacity-release path for pending records.

## Rejection and resource evidence

Each version records **24 committed transaction events and 20 expected
rejections**. One committed inclusion is subsequently orphaned; event counts are
not final-canonical throughput. The 20 rejections cover:

- Minting an Included record, arbitrary identity, unknown/Proven stage, oversized
  payload or excessive admission fanout.
- An immature challenge, omitted since, payload mutation, burn, split, lock
  takeover and capacity reduction.
- Inclusion without the real anchor, swapped distinct payloads, excessive input
  count, wrong payload prefix, wrong recorded slot or excess witness bytes.
- Spending/reprocessing or burning an Included pending record.

The harness checks exact consensus errors or the expected priority program hash,
script location and numeric error. It permits the appropriate set of priority
locations when several independent type groups can reject the same transaction;
the node need not execute these groups in input order. Unrelated failures and
transport errors abort the suite. Mutations of transaction outputs are re-signed;
the oversized witness belongs to a permissionless group, so the funding signature
is still valid.

Both node versions returned identical cycle measurements for this workload.
They include all transaction scripts, including the SECP funding lock.

| Inclusion sample | Priority burden | Whole-transaction estimated cycles |
|---|---:|---:|
| One 3-byte message, three saturation samples | 567 bytes | 2,133,578–2,174,562 |
| Four distinct 3-byte messages | 1,893 bytes | 4,622,382 |
| Four 4096-byte messages | 18,265 bytes | 13,434,494 |

All 24 committed events have RPC cycle estimates, spanning 1,620,743–13,434,494.
The accepted burden ceiling is 20,480 bytes; it includes input descriptors,
input CellOutputs, message bytes and all witnesses. Full transaction bytes and
DA storage are additional. A 3-byte message reserves 335 CKB in this fixture;
a 4096-byte message reserves 4428 CKB, including the fixture's extra 1 CKB.
These are funded dummy-chain capacities, not an economic viability result. Each
transaction uses the laboratory's fixed 1 CKB fee.

Independent TypeIDs create distinct script groups: this experiment does not
claim that multiple messages share one execution of the verifier. The per-script
cycle check and conservative aggregate bound are specified in
[A2_OBLIGATION_V1.md](A2_OBLIGATION_V1.md). The gas bound is enforced by the local
pinned EVM executor; proving it on CKB remains a separate required boundary.

## Reproduction and provenance

```bash
TACTUS_DEVNET_SUITE=replay-priority CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-priority TACTUS_CKB_VERSION=0.210.0 CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

The isolated launcher uses separate funded accounts, deterministic blocks and a
loopback-only node. It stops only its own process. It writes raw evidence,
summary, source/binary/configuration hashes and logs under `artifacts/priority-*`.
The script ELF SHA-256 is
`a79d76aa2bc34f3ff77650ce2d45a6a5d8505f3efb25ff2045f20099937037a8`.

Portable evidence is in [evidence/a2-obligations](evidence/a2-obligations/), with
[0.121.0 summary](evidence/a2-obligations/ckb-0.121.0-summary.json),
[0.210.0 summary](evidence/a2-obligations/ckb-0.210.0-summary.json), gzip-compressed
full transaction/outcome records, manifests, node configurations, chain specs,
replay logs and [SHA256SUMS](evidence/a2-obligations/SHA256SUMS). Source hashes were
checked against the final implementation; the manifest's base commit predates
this milestone and its source inventory includes the then-untracked new files.

Local validation also passed formatting, Clippy with warnings denied and
**73 Rust tests**. Adding the priority package changes the Cargo.lock-bound EVM
rules domain: the independent Geth 1.17.8 comparison was rerun for all 9 scenarios /
14 blocks and its frozen vectors and raw archive refreshed. The older CKB-to-EVM
report retains its original evidence/domain. A fresh CKB 0.121.0 recovery regression
also passed under the new domain; its raw evidence and manifest are included with
the A2 archive under the `regression-` prefix. CI now includes this comparator on
both CKB versions; no remote CI run is claimed for these unpushed changes.

## Decision and remaining work

Reject standalone challenge markers as the forced-processing mechanism. Keep the
authenticated message and pending-record controls as experimental building blocks.
A2 still needs mandatory globally visible enforcement, proof-bound fulfillment,
complete resource/economic qualification and overload policy. A3 still needs
real authenticated sealing, mandatory snapshot switching and adversarial tests.
Unplanned network reorgs and public-miner scheduling remain unmeasured. See
[production readiness](PRODUCTION_READINESS.md) for the complete outstanding path.
