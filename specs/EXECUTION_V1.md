# Serial execution V1 — experimental Shanghai domain

Implemented in `tactus-o1-execution`; **not a production release or a G5 pass**.
This is the first executable transition function for the bounded batch-input V1
format. CKB publication alone does not verify this execution. Settlement must
still verify the state root derived from published genesis, the execution program,
outputs and a real proof.

## Pinned domain

Rust 1.92, revm 43.0.3, Alloy consensus/EIPs 2.1.1, Alloy primitives 1.6.0 and
Alloy trie 0.9.8 are used. Direct versions are exact and transitive versions are
locked. `rules_hash = CKBHash("tactus/o1/execution-rules/v1" || rules-v1.txt ||
Cargo.lock)` binds the literal checked-in files including newlines. Even an
unrelated lockfile change creates a different domain; this conservative boundary
is intentional until execution dependencies have an independently managed lock.
The executable guest/code identity will additionally be required by settlement.

Shanghai is selected explicitly together with its mainnet gas parameters; no
nonce, fee, balance, signature or sender-code checks are disabled. This is a
Shanghai **rollup execution profile**, not current Ethereum mainnet consensus.
EIP-4844, EIP-7702 and later forks are unsupported. Legacy unprotected signatures
are rejected. PREVRANDAO is zero and is **not randomness**. There is no issuance,
beacon withdrawal or deposit operation. Timestamps may repeat (as in batch V1);
an L1-bound timestamp rule remains outstanding. The A3 adapter enforces a
mandatory publication prefix; proof-bound execution remains separate.

The initial allocation has a [canonical immutable CKB publication](GENESIS_ALLOCATION_V1.md)
bound to the anchor type. Standalone execution accepts an explicit `Genesis` input
with sorted addresses,
nonce, balance, bytecode and storage. Empty accounts are rejected; zero-valued
storage is omitted. Total initial balance is bounded by `u64::MAX` wei (about
18.45 ETH) for this experimental profile. With conservation and no issuance this
makes overflowing u64 base fees unreachable: a nonempty block must be able to
pay for at least 21,000 gas, and an empty parent reduces the fee. This bound is
not suitable as a production asset supply policy and must be revisited together
with proof-bound genesis derivation and bridge minting before production.

Genesis header number/timestamp/gas-used are zero, gas limit is 1,000,000 and base
fee is 1 gwei. Its state root authenticates the allocation; its extra data binds
rollup identity, chain ID and execution rules. Genesis header hash must become an
authenticated settlement input. Recovery now derives allocation from the immutable
creation publication, or cross-checks a caller-supplied allocation against it.

## Complete, ordered slot semantics

The entire batch is validated before decoding owned blocks, including empty
blocks. The executor works on a clone and replaces its state only after every
block succeeds. Invalid envelope bytes or fatal engine failures abort the whole
call without partial execution. Memory allocation/panic/process failure is not
caught and must never be converted into a successful or rejected transaction.
This is memory atomicity, not durable storage.

Each nonempty byte input has exactly one ordered outcome. Checks have this
precedence:

1. Typed prefix 3 through 127: `UnsupportedType` (2).
2. Exact EIP-2718 decode and byte-for-byte canonical re-encoding: `Malformed` (1).
3. Unsupported decoded type: `UnsupportedType` (2).
4. Low-s secp256k1 recovery: `InvalidSignature` (3).
5. Exact protected chain ID: `WrongChain` (4).
6. Declared gas limit exceeds this block's remaining gas: `BlockGas` (5).
7. revm transaction validation, including nonce, intrinsic gas, upfront funds,
   fee cap and EOA code: `InvalidTransaction` (6).
8. EVM completion: `Success` (7), `Revert` (8), or `Halt` (9).

Dependency validation error text is diagnostic only and never consensus bytes.
Rejected slots consume no nonce, balance or gas; they do not enter the Ethereum
transaction/receipt tries. A fresh journal is used for each transaction, so an
invalid slot cannot poison the next one. Successful, reverted and halted EVM
transactions are included with dense Ethereum transaction indices. Revert and
halt charge gas and increment nonce; their receipts have status zero and no logs.
They roll back contract state according to Shanghai rules.

Per-block gas is accumulated from revm's post-refund transaction gas used. A
transaction must fit its declared gas limit in remaining block capacity before
execution. Base fee derives from actual parent gas using EIP-1559, elasticity 2
and denominator 8, with checked wide intermediates. The first block therefore
starts at 875,000,000 wei/gas after the empty genesis. Empty blocks advance this
schedule and header history too.

## Ethereum roots and rollup outcomes

Account/storage roots are Ethereum Keccak/RLP Merkle-Patricia roots, calculated
with Alloy trie. Zero slots, nonexistent/selfdestructed and empty accounts are
absent. Transactions use their signed EIP-2718 bytes in a dense ordered trie;
receipts use typed receipt bytes including status, cumulative gas, bloom and logs.
Headers use Ethereum RLP hashing, zero difficulty/mix hash/nonce, empty ommers and
withdrawals, and no post-Shanghai header fields. BLOCKHASH reads computed hashes
for the previous 256 blocks.

Rejected slots still need authentication. Each block's 32-byte extra data is
`CKBHash("tactus/o1/slot-outcomes/v1" || payload)`. Payload is:

- rollup ID (32), rules hash (32), chain ID (u64 LE), complete batch input commitment
  (32), block number (u64 LE), slot count (u32 LE);
- for each slot: Keccak(input) (32), status (u8), gas used (u64 LE), dense Ethereum
  index (u32 LE; `0xffffffff` for rejection), Keccak(return/output bytes) (32),
  created-address-present (u8), created address (20; zero when absent).

Thus batches with the same Ethereum state but different rejected bytes cannot
produce the same block hash. Full return data and creation addresses are returned
for clients; status codes and encoding are specified in `rules-v1.txt`.

## Verification and remaining work

`cargo test --locked -p tactus-o1-execution` covers real signed legacy, access-list
and EIP-1559 transfers, strict signature/domain checks, deployment, SSTORE and
zero-slot deletion/refunds, logs, REVERT/HALT, upfront/intrinsic/fee failures,
remaining block gas, SHA256/identity precompiles, block environment and parent
BLOCKHASH, empty blocks, deterministic replay and atomic rejection of every
truncated batch prefix. These are focused regressions, not a complete Ethereum
state-test corpus. A separate [Geth comparison](EXECUTION_DIFFERENTIAL_REPORT.md)
now verifies 9 scenarios and 14 blocks against independent Go execution.

An [append-only durable journal](EXECUTION_JOURNAL.md) now replays inputs on
restart and validates saved roots; storage-fault and cross-process restart tests
cover this local path.

[Canonical CKB input recovery](CKB_EVM_RECOVERY_REPORT.md) now feeds independent
EVM replay across planned branch replacement on both supported CKB versions.

Still required: persistent state checkpoints and efficient network-driven reorg
handling, broader differential conformance,
standard JSON-RPC/tool deployment, proved resource bounds, authenticated genesis
and priority semantics, zkVM guest/prover and CKB validity settlement. No execution
result currently authorizes withdrawals.

Primary references: [revm](https://github.com/bluealloy/revm),
[Alloy consensus](https://docs.rs/alloy-consensus/2.1.1/alloy_consensus/),
[Alloy trie](https://docs.rs/alloy-trie/0.9.8/alloy_trie/), and
[EIP-1559](https://eips.ethereum.org/EIPS/eip-1559).
