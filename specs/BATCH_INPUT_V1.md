# BatchInput v1 — bounded input and atomic CKB publication

Status: implementation boundary for the O1 multi-block pipeline. This supersedes
neither the A1 experiment wire format nor the still-open Priority Inbox decision.
It does not establish execution totality, validity settlement or production readiness.

## Decisions

An anchor authenticates an ordered sequence of **execution inputs**, not claimed
post-execution state roots. Builders must not be able to stall settlement by
anchoring an invented output state, gas-used value or receipt root that no valid
execution can prove. The executor/prover will derive those outputs. Validation of
Ethereum envelopes, state-dependent rejection outcomes and the full proof statement
remain required before this format can be used by a production rollup.

A batch contains 1–16 ordered block inputs. Numbers are derived: the first is the
prior anchor's last block number plus one, and subsequent numbers increment by one.
Each block has a timestamp, fee recipient and an ordered list of raw transaction
inputs. Timestamps are nondecreasing, including across anchors; equal second-level
timestamps are allowed for speculative subsecond blocks. This is an explicit L2
rule. A consensus-authenticated bound to L1 time remains required; monotonicity
alone is not that bound.

Gas limit is fixed at 1,000,000 per block in this initial admission policy. This
is a development resource ceiling, not a measured production throughput choice.
BASEFEE is derived from actual parent execution according to EIP-1559, never a
builder-supplied field in BatchInput. Neither state roots nor output Ethereum
block hashes are asserted by the anchor input. BLOCKHASH, receipt construction,
base-fee execution and rejection receipts must be tested by the execution layer.

## Encoding

All integers are little endian, all byte lengths are exact, and unknown magic,
versions, trailing bytes, truncated values and over-limit counts are rejected.
Hashing uses Blake2b-256 with CKB's `ckb-default-hash` personalization and explicit
per-object domain tags. There are no implicit padding bytes.

BatchInput header, in order:

| Field | Bytes |
|---|---:|
| magic `TO1BAT01` | 8 |
| rollup_id | 32 |
| chain_id | 8 |
| batch_number | 8 |
| parent_batch_commitment | 32 |
| execution_rules_hash | 32 |
| da_policy_id | 32 |
| limits_hash | 32 |
| first_block_number | 8 |
| block_count | 2 |

Each block follows in order as `timestamp:u64`, `fee_recipient:[u8;20]`,
`transaction_count:u16`, then `length:u32 || raw_transaction[length]` for every
transaction. A transaction has 1–16,384 bytes. There are at most 256 transactions
per block, at most 1,024 per batch, and at most 262,144 bytes for the entire batch.
Empty blocks are allowed. Opaque bytes here are **not yet validated Ethereum
transactions**; accepted bytes must not be treated as proven execution.

`batch_commitment = H("tactus/o1/batch-input/v1" || entire_canonical_BatchInput)`.
The committed input includes domain, rules, limits, every boundary, every byte,
order, timestamp and fee recipient. No hash-only reconstruction claim is allowed.

## Anchor state and CKB transaction

AnchorState is exactly 200 bytes:
`"TO1ANC01" || rollup_id[32] || next_batch_number:u64 || last_batch_commitment[32]
|| last_block_number:u64 || last_timestamp:u64 || execution_rules_hash[32]
|| da_policy_id[32] || limits_hash[32] || chain_id:u64`.

Genesis identity uses the CKB Type ID seed (`first_input[44] || output_index:u64`).
Counters, prior commitment and timestamp start at zero. Genesis must use the
implemented inline-DA policy and exact admission limits hash. Chain ID is nonzero.
Execution rules identify a separately pinned, nonzero execution program; the
current devnet uses an explicit unimplemented-execution identifier.

A transition consumes one AnchorState and recreates exactly one with the same
capacity, lock and domain fields. The first group-input `WitnessArgs.input_type`
is exactly a four-byte **absolute output index**. That output contains the full
BatchInput bytes in `outputs_data`, has no type script, and uses the anchor type script itself as its lock, with **empty args**. Empty args
always fail in that program and make the output unspendable, retaining published bytes as live CKB data.
The on-chain script validates that output, decodes its bytes and derives the only
valid next AnchorState. Witness commitments without that output cannot advance.
The DA output cannot alias the state output.

This immutable retained-data policy pays permanent CKB capacity and is deliberately
simple. Production affordability/archival policy remains open. No off-chain cache
is counted as O1 publication evidence. Checkpoint authenticity currently consists of
the verified CKB anchor transition; no arbitrary cell with a plausible hash is an
authenticated checkpoint.

## Limits and open boundaries

The new anchor has a distinct wire magic and deployed code hash from the A1
OrderingHead experiment; the A1 bare-commitment path cannot advance this anchor.
The implementation establishes byte publication, domain binding, bounded parsing
and contiguous block/batch succession only. It does not enforce the Priority Inbox,
verify EVM execution, verify proofs, authorize a bridge or enforce L1 timestamp
bounds. These omissions keep W-12, G2, G3, G5, G6 and G7 open.

Source for the base-fee dependency: [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559).
