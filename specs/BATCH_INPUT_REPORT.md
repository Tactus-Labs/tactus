# Batch input and atomic data publication — VM evidence

**Date:** 10 October 2026. **Versions:** CKB 0.121.0 and 0.210.0.
**Result:** the input-publication boundary passes the same controls on both.
**Production status:** not ready; W-12 and all end-to-end gates remain OPEN.

## What changed

The new anchor authenticates full ordered block inputs instead of accepting a bare
32-byte commitment. Every accepted successor has its entire canonical input in an
immutable output of the same CKB transaction. A distinct script and state magic
prevent the old A1 experiment's loose witness convention from bypassing this rule.
The fixed limits and exact wire encoding are in [BATCH_INPUT_V1.md](BATCH_INPUT_V1.md).

A shared no-std parser is run both on the host and in CKB-VM. It streams over length
fields without allocating from untrusted counts. The script checks data length
before allocating the full payload. Genesis uniquely derives the domain from its
consumed seed; ordinary transitions derive the only acceptable successor state
from the published bytes and preserve the head lock and capacity.

## Measured controls

Each node run recorded six commit events and 24 expected rejections. One branch
was deliberately orphaned. These are event counts, not throughput measurements.

| Control | Observed outcome |
|---|---|
| Independent actor publishes two blocks | Complete input and next anchor commit atomically |
| Hash-only/missing witness, missing DA, DA pointer to state output | Rejected by the anchor script |
| Spendable DA output | Rejected; a private operator cannot later remove the retained data |
| Split/burn anchor, invented block frontier | Rejected |
| Wrong rollup, chain, rules, limits or DA policy | Rejected |
| Wrong predecessor, batch gap or block gap | Rejected |
| Unknown version, truncated or trailing bytes | Rejected |
| Excess block/transaction size or timestamp regression | Rejected |
| Exactly 262,144 payload bytes | Committed |
| 262,145 payload bytes | Rejected before loading the oversized payload |
| Spend a previously published DA cell | Rejected by the immutable lock |
| Recover after truncate and replacement | Exactly the canonical inputs recovered, no orphan input retained |

The `rejected_invalid_transitions` field in the raw result counts the initial 22
controls. The two additional rejections are the maximum-byte overrun and the
attempt to spend published data, for 24 total.

The final chain contains three batch inputs covering four block inputs. Recovery
reconstructs **262,636 bytes** from canonical CKB block transaction outputs, without
an indexer or operator snapshot. It revalidates each input's domain, commitment,
predecessor and next state, checks the final state is live, and rejects a changing
tip rather than mixing branches.

Six RPC cycle estimates range from **1,620,743 to 9,097,180**, including funding
signature verification. The maximum-size payload is included. These are measured
script-cost estimates, not proof cost, sustained performance or production capacity
pricing. Permanent data-cell capacity and the current byte/gas ceilings require
production affordability review.

## Evidence and reproduction

- [CKB 0.121.0 summary](evidence/batch-input/ckb-0.121.0-summary.json)
- [CKB 0.210.0 summary](evidence/batch-input/ckb-0.210.0-summary.json)
- `evidence/batch-input/` contains compressed complete transaction/error records,
  source/compiler/code manifests, node configurations, chain specs and SHA256SUMS.
- `test-vectors/batch-v1/` contains independently generated fixed wire vectors;
  `scripts/generate-batch-vectors.py` is the Python struct/hashlib oracle.

```bash
TACTUS_DEVNET_SUITE=replay-batch CKB_BIN=/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-batch TACTUS_CKB_VERSION=0.210.0 CKB_BIN=/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

CI runs this suite on both versions and checks the independent wire vectors. Local
validation passes 42 Rust tests, formatting and Clippy; remote CI execution is not
claimed for unpushed work.

## Remaining production requirements

These fixtures contain bounded **opaque transaction bytes**, including deliberately
non-Ethereum test bytes. Their acceptance demonstrates the input/publication
boundary, not Ethereum validity or total execution. The executor must implement
and prove deterministic envelope validation, state-dependent rejection and execution
outcomes; otherwise an admitted malformed input could still block settlement.

This anchor has no authenticated Priority Inbox processing rule, no proof verifier,
no bridge authorization and no L1 clock bound. BASEFEE and BLOCKHASH derivation,
Ethereum block/receipt output commitments, actual state reconstruction and all
A2/A3 enforcement obligations remain required. Input recovery cannot substitute for
EVM state recovery, and immutable bytes cannot substitute for a validity proof.
