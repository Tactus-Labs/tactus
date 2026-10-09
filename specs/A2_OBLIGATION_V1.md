# A2 individual obligation comparator, v1

This experimental construction authenticates individual messages and their
inclusion records. **The challenge-only construction fails forced inclusion.**
It does not implement a global priority queue, proof settlement or a production
inbox. See [measured counterexample](A2_OBLIGATION_REPORT.md).

## Script identity and authorization

`tactus-o1-priority-script` is a `data1` CKB program used in two roles:

- Type args: byte `0` followed by a 32-byte unique identity.
- Lock args: byte `1` followed by that complete type script's CKB hash.

The protocol lock permits spending only when the same input has the named type.
The type requires exactly one successor with the identical type, lock and
capacity. Burning, splitting, replacing the lock or reducing locked capacity is
invalid. This is permissionless processing: the admitting account need not sign
processing or challenge transactions. Fee funding is a separate lock group.

A newly admitted identity is `CKBHash(first CellInput || output_index_u64le)`.
The 44-byte CellInput includes `since` and its previous OutPoint; the output index
is absolute, not the type-group index. New messages may only be `Admitted`.
Admission does not consume or reference an anchor. A claimed anchor hash/rollup
pair is checked against a real anchor transition when the message is included;
creating arbitrary-domain messages does not grant rights in another rollup.

## Canonical message bytes

All integers are unsigned little endian. No suffix or optional field is allowed.

| Offset | Bytes | Field |
|---:|---:|---|
| 0 | 8 | ASCII `TO1PRI01` |
| 8 | 1 | Stage: 0 Admitted, 1 Challenged, 2 Included |
| 9 | 32 | Unique message identity |
| 41 | 32 | Rollup identity |
| 73 | 32 | Complete anchor type script hash |
| 105 | 32 | Priority policy hash |
| 137 | 8 | Inclusion batch number |
| 145 | 32 | Inclusion batch commitment |
| 177 | 8 | Inclusion EVM block number |
| 185 | 2 | Inclusion slot in the first EVM block |
| 187 | 4 | Payload length |
| 191 | 1–4096 | Original input payload |

The first three identities must be nonzero. The policy must equal
`priority::policy_hash()`: CKB hash of the
`tactus/o1/priority-policy/v1` domain, the fixed semantic descriptor in
`priority.rs`, and its ten ordered resource limits encoded as u64le. The type
program's code hash also commits the enforcement implementation. This is separate
from the execution rules hash.

## State transitions

`Admitted → Challenged` preserves every field except the stage. The input's
consensus `since` must be exactly `0x800000000000000c`: a relative delay of 12
CKB blocks. CKB checks maturity from the admitted cell's creation. Zero since,
absolute since, another duration and early consumption are invalid. There is no
second challenge transition that could reset the clock.

`Admitted | Challenged → Included` consumes the real named anchor and creates
its unique successor. It loads that anchor's batch-data output through its
4-byte `WitnessArgs.input_type` index, validates the complete BatchInput v1 and
requires the exact successor anchor. The anchor script independently enforces
atomic immutable publication of the bytes.

All consumed priority cells from this program must name the same rollup and
anchor and must be within the processing bounds below. In absolute CKB input
order they form an exact payload prefix of the first EVM block. Every input
must have one successor record with unchanged payload/identity/domain/capacity
and the exact batch number, commitment, block number and slot. Mixing a challenge
successor with an inclusion transition is invalid. The prefix covers only the
messages actually consumed; it proves neither global completeness nor FIFO.

`Included` has **no spending transition** in this comparator. The pending record
keeps the original obligation and publication location alive and prevents reuse
of the same identity. There is no `Proven` encoding, withdrawal, settlement
cursor or capacity-release path. Malformed or repeated Ethereum transactions can
be Included as published input slots with deterministic rejected execution
outcomes. Included is not successful Ethereum execution or settled fulfillment.

## Resource limits and their scope

| Resource | Bound | Enforcement |
|---|---:|---|
| Priority inputs per inclusion | 4 | Priority type script |
| Priority outputs per admission | 4 | Priority type script |
| Individual payload | 4096 bytes | Canonical codec and bounded VM allocation |
| Priority input/witness burden | 20,480 bytes | Priority type script |
| Single priority script | 12,000,000 cycles | End-of-script cycle rejection, 4096-cycle exit reserve |
| All priority script groups in an inclusion | 96,000,000 cycles | At most 4 type and 4 lock groups, each bounded |
| First EVM block, including priority prefix | 1,000,000 gas | Pinned off-chain executor; not CKB-proved yet |
| Entire CKB transaction inputs / outputs / witnesses | 12 / 16 / 32 | Inclusion type path (witness limit also bounds allocation) |

The burden is the sum, for each priority input, of its 44-byte input descriptor,
serialized CellOutput and complete message bytes, plus `8 + witness_length` for
**every** transaction witness. Witness lengths are bounded before decoding the
anchor witness. The burden is not a whole-transaction byte or fee estimate: DA
outputs, other inputs and dependencies are separate, under their own protocol
and CKB limits. BatchInput v1 additionally limits the whole batch to 262,144 bytes.

The cycle check bounds **accepted** scripts; it does not preempt work at exactly
12 million cycles. CKB's transaction/block limits still apply and can be lower
than the conservative sum above. Successful maximum-payload measurements are in
the report; this is not a production performance qualification. The 4-input /
96-million aggregate describes inclusion, not arbitrary transactions challenging
several distinct cells at once.

Gas is currently enforced by the independent execution implementation. The CKB
message script verifies byte publication and prefix identity, not an EVM proof.
A future settlement proof must bind the actual rejected/successful outcomes and
prevent releasing obligations based only on these Included records.

## Why challenge alone fails

An anchor transition that never consumes a message does not execute its type
script. A standalone challenge cannot force unrelated ordinary anchor
transactions to read it. The real-node experiment advances the anchor after a
mature challenge and processes a favorable subset while the victim remains
Challenged. This disproves forced processing for this construction, not every
possible A2 design or CKB mechanism.

A complete design still needs a mandatory globally visible enforcement path,
authenticated proof/settlement binding, admission economics and a liveness
argument under overload and hostile scheduling. Introducing shared registration
or challenge state must account for the contention that it introduces. Larger
penalties or an indexer's preferred queue do not establish these properties.
