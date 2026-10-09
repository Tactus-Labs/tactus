# A3 bounded sealed-epoch candidate, v1

**Implementation phase: pure transitions plus a mandatory CKB gate and real-node
experiments and independent canonical recovery.** The
[measured gate report](A3_SEALED_REPORT.md) and
[cold recovery report](SEALED_RECOVERY_REPORT.md) record their scope.
Production qualification, proof settlement, admission fairness and unplanned
network reorgs remain outstanding. The original untyped immutable-copy experiment
is separate historical evidence, not the authenticated construction specified here.

## Chosen schedule and boundaries

One mandatory Schedule accompanies a canonical anchor. There are 1–4 configured
lanes, each admitting at most 8 messages of 1–1024 bytes per epoch. An append adds
exactly one message and cannot be an empty head update. Each admitted message has
an implicit identity `(gate, lane_index, sequence)` and an authenticated payload.

A snapshot interleaves lane FIFO prefixes by depth, then increasing lane index.
Empty lanes are skipped. Every anchor batch must publish the next
`min(4, remaining)` messages as its first EVM block's exact input prefix. A batch
with several EVM blocks still consumes exactly one schedule slot; moving the duty
to a later block is forbidden. Rejected Ethereum input outcomes do not remove the
publication duty. Execution and proof settlement remain separate obligations.

An epoch permits exactly 8 successful batch transitions. The ninth must fail
until a seal consumes every configured current lane head, publishes their exact
immutable snapshot, and recreates those lane heads with empty active queues and
an incremented epoch. Queue clearing preserves the cumulative history root and
next sequence. Sealing early, excluding a lane, repeating a lane or selecting an
older epoch is invalid. Every full snapshot (at most 32 messages) is therefore
exhausted by the end of the 8-batch quota. The next epoch's gate pins the new
snapshot commitment; it cannot continue using the prior one.

At genesis there is no snapshot. The first 8 batches allow active admissions to
accumulate, then the first complete seal activates them. During later epochs,
new admissions affect only active heads; the current immutable snapshot and its
processing duty do not change.

These rules bound processing by **canonical batch progress**, not wall time.
A message accepted just after a seal becomes mandatory after at most 8 further
batch advances plus a seal, and is processed within at most 8 more advances.
This statement assumes the mandatory adapter is correct, the relevant canonical
history persists and transactions can be included on L1. It does not claim that
8 or 16 CKB blocks suffice. If the gate stops progressing, no elapsed-time
liveness guarantee follows from these pure transitions.

Admission has explicit backpressure: full lanes reject new messages until the
next seal. At most 32 positive appends can mutate all four active heads in an
epoch, so valid active-head churn is finite in that canonical epoch. This limits
one seal-invalidating workload but does not prove timely sealing under adversarial
miners, reorgs or repeated fee competition. Adversarial queue filling can still
block a new user's admission. The A1 contention and admission-economics questions
remain part of Experiment A; bounded capacity is not called censorship resistance.

## Canonical bytes

All integers are unsigned little endian. Encodings have no suffix or padding;
decoders validate bounds before allocating from supplied lengths.

A `Lane` is `TO1LAN01` (8 bytes), gate identity (32), index (1), epoch (8), next
sequence (8), queue-base history root (32), final history root (32), queue count
(1), then each payload as length (u16) and exact bytes. Its maximum length is
**8330 bytes**. The first sequence is `next_sequence - queue_count` with checked
subtraction. Replaying the queued payloads from the base root must yield the
recorded final root. Empty queues retain the cumulative root from earlier epochs.

The genesis history root is CKBHash of domain `tactus/o1/lane-genesis/v1`, gate
identity and lane byte. Appending is CKBHash of domain
`tactus/o1/lane-append/v1`, preceding root, sequence u64le, payload length u16le
and payload. A base at sequence zero must equal the lane genesis root. Roots at
later sequences require an authenticated transition history; internal consistency
of arbitrary serialized bytes does not prove their provenance.

A `Snapshot` is `TO1SEA01` (8), gate (32), sealed epoch (8), lane count (1), then
for each lane its serialized length (u32) and complete Lane bytes. Lanes appear
in exact index order from zero; all must match the gate and sealed epoch. Its
maximum length is **33,385 bytes**. The commitment is CKBHash of domain
`tactus/o1/sealed-snapshot/v1` followed by the complete encoding.

A `Schedule` has exactly **182 bytes**:

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 8 | `TO1SCH01` |
| 8 | 32 | Unique gate identity |
| 40 | 32 | Rollup identity |
| 72 | 32 | Complete anchor type script hash |
| 104 | 1 | Configured lane count |
| 105 | 8 | Current epoch |
| 113 | 1 | Batches used in the current epoch |
| 114 | 2 | Published-message cursor within snapshot |
| 116 | 32 | Snapshot commitment (zero only during epoch zero) |
| 148 | 2 | Snapshot message count |
| 150 | 32 | Sealed-policy hash |

The cursor must equal `min(batches_used * 4, snapshot_message_count)`. A nonzero
epoch requires a nonzero snapshot hash, whose snapshot epoch is exactly one less
than the schedule epoch. Gate, rollup and anchor identities are nonzero.

The policy hash uses `tactus/o1/sealed-policy/v1`, the semantic descriptor in
`sealed.rs`, and u64le values for lane count bound, messages per lane, payload
bytes, priority messages per batch, batches per epoch, maximum Lane bytes and
maximum Snapshot bytes. Policy changes require a different domain identity.

## CKB adapter contract and remaining qualification

`tactus-o1-sealed-script` enforces items 1–6 and the transaction/script resource
bounds in item 7. Real-node evidence covers the local scenarios of item 8; proved
execution, settlement, public miner scheduling and unplanned network reorgs are
still required. The pure functions alone do not establish these properties:

1. Create a unique genesis Schedule and complete lane configuration; reject
   replacement identities, extra/missing lanes and creation of later-epoch heads.
2. Bind every anchor spend to a co-spent Schedule through its protocol lock;
   preserving the anchor's lock must make omitting the gate impossible. Verify
   both named anchor type and rollup domain, and the actual BatchInput publication.
3. Protect Schedule and lane cells from burning, splitting, capacity theft and
   lock substitution. Permit independent fee payers; retain original obligations.
4. Authenticate every consumed lane by its unique configured type identity.
   Admission only permits `prior.append(one_payload)`. Seal permits exactly the
   successor returned by `Schedule::seal` and consumes all configured heads.
5. Separate seal from batch advance: a seal cannot also advance the anchor while
   bypassing the batch quota or prefix duty. Authenticate the operation witness.
6. Publish the exact snapshot in an immutable retained cell. Subsequent batch
   advances must read bytes whose commitment, gate, epoch and lane count match
   the Schedule. Arbitrary caller-made snapshots are insufficient.
7. Bound all transaction inputs, outputs, dependencies, witnesses, bytes and
   script cycles; prove the execution gas bound before settlement. Preserve the
   mandatory duty under overload and deterministic transaction rejection.
8. Exercise malicious switching, omitted/forged lanes, old snapshots, live-head
   churn, maximum queues, finite overload/drain, planned and unplanned reorgs,
   consumed-but-unproven recovery and independent builders on real CKB nodes.

Snapshots are retained; there is no garbage-collection or asset-release path in
this candidate. Publication cursors are not proven settlement cursors. A future
proof must authenticate the consumed identities, execution outcomes and canonical
batch lineage before any obligation or asset can be released.

## Host evidence

`cargo test --locked -p tactus-o1-protocol --test sealed` passes eight tests:
positive bounded admission and history; finite genesis quota and complete sealing;
FIFO round robin with omission/reordering rejection; first-block duty; full
32-message exhaustion and obsolete snapshot rejection; all 81 four-lane occupancy
patterns with 0/1/2 messages; strict codecs including the maximum 33,385-byte
snapshot; and counter-overflow/cursor-forgery rejection.

These tests check the pure transition contract and canonical representation.
They are not a substitute for the CKB adapter checklist or a G2 pass. Current
production blockers remain in [PRODUCTION_READINESS.md](PRODUCTION_READINESS.md).

## Script and transaction interface

The `data1` program has three roles, sharing the same code hash:

- Args `0 || type_script_hash[32]`: co-input lock. An input carrying that exact
  type must be present in the transaction. The anchor and Schedule share the
  Schedule-bound lock; each lane has its own lane-type-bound lock.
- Args `1 || gate_identity[32] || lane_index_u8`: unique lane type.
- Args `2 || gate_identity[32]`: Schedule type. Its identity is CKBHash of the
  first 44-byte input followed by the Schedule output's absolute u64le index.
- Empty args always reject, supplying the immutable snapshot lock.

Schedule creation requires a genesis anchor with its Schedule-bound lock and
all configured genesis lanes in the same transaction. Lane creation requires a
fresh co-created Schedule of the matching identity; a later Schedule spend
cannot authorize minting another head. Lane genesis reserves enough fixed
capacity for its maximum 8330-byte data. All Schedule/lane successors preserve
their type, protocol lock and capacity, with exactly one output per type group.

Ordinary append uses a lane input and its exact one-message successor. It has no
mutable Schedule dependency. The gate is therefore not invalidated by admission,
and ordinary Schedule advances do not invalidate an already signed lane append.

The Schedule input's `WitnessArgs.input_type` selects an operation:

- `[1]`: advance a batch. Both named anchor input/output must be present; the
  anchor's own input witness supplies the 4-byte DA output index. The Schedule
  validates the real batch and exact successor and enforces its mandatory prefix.
- `[0] || snapshot_output_index_u32le`: seal. All configured lane inputs and
  exact cleared successors must be present. An anchor input/output is forbidden
  in this operation, preventing a seal from hiding an unmetered batch advance.

For nonzero schedule epochs, a batch transaction references its immutable snapshot
as a cell dependency. The gate scans a bounded set of resolved dependencies,
checks the complete snapshot commitment, epoch, gate and lane configuration, and
requires the exact immutable program lock and no type. A caller-created snapshot
with different bytes cannot replace the pinned commitment. An identical retained
copy has the same authenticated contents; no particular indexer or publisher is
trusted. Oversized unrelated code dependencies are skipped before data allocation.

The adapter bounds transaction inputs to 8, outputs to 10, resolved cell
dependencies to 10 and witnesses to 12. Total witness length overhead, including
8 bytes per witness, is at most 4096 bytes and is checked before witness decoding.
Each accepted sealed-program script is capped at 20,000,000 cycles, with a
4096-cycle exit reserve. The end check bounds acceptance, not preemption of failed
work. Generic CKB transaction/block limits also apply; this is not an aggregate
20-million-cycle transaction claim. The existing BatchInput bounds and first EVM
block gas profile still apply; on-chain validity proof enforcement is pending.

Error codes: 1 args, 2 cardinality, 3 codec, 4 identity, 5 lock/capacity,
6 genesis, 7 missing co-input/creation authority, 8 configured lane missing,
9 snapshot, 10 anchor, 11 witness/operation, 12 resource limit, 13 cycle budget,
14 epoch/quota, 15 mandatory prefix, 16 transition mismatch.
