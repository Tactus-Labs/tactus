# Devnet OrderingHead encoding and trust boundary

Status: executable mechanism fixture, 10 October 2026. This is **not** the
production BatchManifest, proof statement or Priority Inbox wire protocol.

## State and identity

The ordering type uses `hash_type=data1` (CKB-VM v1) and 32-byte script args.
The args equal the head's rollup ID. Genesis derives that ID with CKB-personalized
Blake2b-256 over `first_input[44] || absolute_output_index_u64_le[8]`, following
the [CKB Type ID construction](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0022-transaction-structure/0022-transaction-structure.md#type-id).
That consumed seed cannot create the same identity a second time. A deployment
must also pin the ordering code hash; an identity under another code hash is not
this deployment.

Head data is exactly 188 bytes. Integers are little endian.

| Offset | Bytes | Field |
|---|---:|---|
| 0 | 32 | rollup_id |
| 32 | 4 | protocol_version |
| 36 | 8 | next_batch_number |
| 44 | 32 | batch_accumulator_root |
| 76 | 32 | inbox_root |
| 108 | 8 | inbox_tail |
| 116 | 8 | processed_inbox_cursor |
| 124 | 32 | execution_rules_hash |
| 156 | 32 | da_policy_id |

Genesis requires version 1, zero counters, zero accumulator and zero inbox root.
Execution-rules and DA-policy hashes are immutable identifiers, not evidence that
execution or data availability has been implemented. The fixture deliberately
uses hashes of strings identifying those missing implementations.

There is exactly one output per type group, and zero (creation) or one
(succession) input. Splits, merges and destruction fail. Succession preserves
capacity and lock hash; separate actor funding pays transaction fees. Counter
increments are checked and never wrap. The cursor cannot exceed the tail or
regress during append; invalid prior cursor states cannot be propagated.

## Permissionless lock and witnesses

The separate head lock takes the exact type-script hash as its args. Every input
in its lock group must carry that type script. CKB executes the bound type script
when the cell is spent; removing its type does not bypass the required successor.
The laboratory's two actors use distinct SECP keys and pay their own fees.

Transition input 0 is the permissionless head, and input 1 is the actor's fee
cell. `WitnessArgs.input_type` on input 0 carries exactly one 32-byte commitment.
The signature on input 1 covers only that actor's lock-group witnesses, according
to the system SECP signing scheme. It does not incorrectly include the head's
lock-group witness.

The type selects ENQUEUE when `tail_after == checked(tail_before + 1)`; all other
states must satisfy APPEND_BATCH. Hash chaining uses CKB-personalized Blake2b-256
on the 64-byte concatenation of the prior root and supplied commitment.

## Limits that block production

A message commitment has no authenticated EVM payload or admission envelope in
this fixture. A batch commitment has no validated manifest, CKB DA publication,
authenticated consumed prefix or validity proof. A cursor increment is therefore
**not proof of actual processing**. Accepting a transition here must never be
used to release bridge funds or label a transaction proven.

The A2 cells in the mechanism experiment are ordinary independent cells. They
are an omission baseline, not an implemented Priority Message/Obligation Cell
protocol. The A3 snapshot controls are immutable copies with no enforced
provenance, epoch transition or mandatory-inclusion deadline. Their immunity to
live-cell churn proves none of those absent rules.

Script code cells are locked by the head-lock program with empty args, which
always fails when spent. Their availability therefore does not depend on a
retained deployment key. Such immutability makes occupied capacity permanent in
these disposable fixtures; this is not a production storage/garbage-collection
policy.

The [CKB-VM runtime specification](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0003-ckb-vm/0003-ckb-vm.md)
is separate from Rust host tests. The linked ELF is executed on a real CKB devnet
in CI, including both successful transitions and rejection cases. The compiler,
node, code hashes and configuration are recorded with each experiment.
