# Immutable genesis allocation, version 1

Current input anchors bind a canonical initial Ethereum allocation in their type
identity and publish its bytes atomically at creation. This closes allocation
ambiguity for the configured deployment: two observers cannot legitimately choose
different initial account states for the same current anchor. It does not prove
execution, back initial balances with bridge assets or close G1/G3/G5/G6/G7.

## Canonical allocation bytes

`TO1GEN01` is eight ASCII bytes followed by `account_count:u32le`. Each account is:

| Field | Encoding |
|---|---|
| Address | 20 raw bytes |
| Balance | 32-byte unsigned big endian |
| Nonce | u64 little endian |
| Code length | u32 little endian |
| Code | Exact `code_length` bytes |
| Storage count | u32 little endian |
| Storage entries | Repeated 32-byte big-endian key, 32-byte big-endian value |

Addresses and each account's storage keys are strictly increasing as raw bytes.
Duplicates and descending order are invalid. Zero storage values are omitted;
encoded zero values are rejected. An account with zero balance, zero nonce and
empty code is rejected, including one carrying storage alone. No padding, suffix,
unknown version or truncated field is accepted. Deployment rollup identity,
network and chain ID are bound separately, avoiding a circular Type-ID dependency
on the allocation publication transaction's own hash.

Executable ceilings are 262,144 total bytes, 1,024 accounts, 24,576 code bytes per
account and 4,096 storage slots per account. Count/length bounds are checked before
allocating. The on-chain validator scans the encoded bytes without constructing
owned account/code/storage collections. The host decoder first validates and then
constructs bounded collections.

Balances retain a 256-bit representation, but this execution profile still limits
**total supply to `u64::MAX` wei**. Overflow or high balance bits are rejected.
This experimental limit is unsuitable for production supply. Changing it requires
an updated execution profile and checked base-fee/issuance policy; the binding
introduced here does not silently relax that limit.

The commitment is:

`CKBHash("tactus/o1/genesis-allocation/v1" || complete_canonical_allocation_bytes)`.

The [independent vector](test-vectors/genesis-v1.txt) contains two lines: encoded
bytes and their commitment. Python `hashlib.blake2b`, digest size 32 and
personalization `ckb-default-hash`, generated the hash independently of the Rust
codec. Six host tests cover this vector, all truncations, suffix/magic changes,
ordering, duplicate keys, zero storage, empty accounts, supply overflow, hostile
counts and an exact 262,144-byte allocation.

## Anchor deployment identity and CKB enforcement

The current anchor type has `data1` hash type and **64 argument bytes**:

`rollup_id[32] || genesis_allocation_commitment[32]`.

The rollup ID remains CKBHash of the first 44-byte transaction input followed by
the anchor's absolute output index as u64le. The 200-byte `TO1ANC01` AnchorState
and `TO1BAT01` batch wire format remain unchanged. Their immutable domain fields
bind chain ID, execution rules and data/admission policies; the complete anchor
type additionally binds allocation. A3's Schedule and A2 messages already bind
the complete anchor type hash, so they inherit this additional identity boundary.

Creation requires a matching allocation output among the first 16 absolute
outputs. It must have no type script and must use the same anchor program with
empty arguments as its lock. That lock always rejects spending. The anchor script
checks bounded canonical allocation bytes and their exact commitment before
accepting genesis. Missing, mismatched, mutable or malformed publication fails
with code 16; an oversized allocation fails with code 13. Empty or legacy 32-byte
anchor args fail with code 1. Canonical transitions still require one connected
successor with the identical type, lock and capacity.

This is a new deployment profile/program hash. Existing allocation-unbound
32-byte-argument deployments are not reinterpreted or silently migrated by the
current recovery APIs. Historical experiment evidence retains its original script
hashes. A1's bare OrderingHead reference still uses its own separate program and
32-byte type helper; it is not a fallback for input-publication anchors.

Allocation data remains locked indefinitely in this initial policy. Its permanent
capacity cost, migration/governance rules and any future garbage collection need
explicit production treatment.

## Independent recovery

Recovery locates the selected anchor's canonical creation transaction, checks the
immutable allocation output and commitment, and derives the initial account state
from those bytes. The caller can additionally provide a local Genesis JSON; its
canonical allocation must match the chain publication before any recovery journal
is accepted or written. Zero-valued JSON storage entries normalize to omission,
so equivalent local and chain-derived representations share one journal identity.

To recover without a publisher-provided allocation file:

```bash
TACTUS_CKB_RPC_ADDR=127.0.0.1:8114 \
  cargo run --locked --bin recover-execution -- \
  --chain TRUSTED_CKB_GENESIS_HASH ANCHOR_TYPE_SCRIPT_HEX RECOVERY_DIRECTORY
```

The expected CKB genesis hash and complete deployment type are trusted
configuration. Wrong chain identity fails before chain-derived genesis adoption.
Rules hash, chain ID and rollup identity come from the authenticated anchor and
must match the supported executor. Canonicality and journal checks remain those
of [execution recovery](CKB_EVM_RECOVERY_REPORT.md). Local saved JSON never replaces
a fresh canonical-chain check.

The returned report names the allocation commitment and derived genesis header
hash. A future validity proof must bind both the published allocation and its
correct state-root derivation. This implementation supplies canonical recoverable
inputs; it does not substitute host replay for CKB proof verification.
