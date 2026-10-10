# Native CKB bridge contract and remaining settlement integration

Status: **contract semantics implemented and independently exercised; no
L2 deposit credit or CKB release**. A [subsequent real-node experiment](NATIVE_VAULT_REPORT.md)
now authenticates funded CKB deposit records. G7 remains OPEN. This advances the native-CKB portion
of architecture §11; the explicitly defined xUDT domain remains required work.

## Implemented contract

`contracts/bridge/NativeCKB.sol` is an immutable ERC-20-style native-CKB ledger
with eight decimals: one token unit represents one shannon in the intended
backing protocol. It has no owner, upgrade hook, external call, arbitrary mint,
or release method. Compilation uses SHA-256-pinned stable solc 0.8.30, Shanghai,
optimizer 200 runs, with metadata omitted. The committed ABI, creation/runtime
bytecode, method identifiers and storage layout are reproducible using
`scripts/build-bridge-contract.py`. A missing compiler is downloaded from the
hash-pinned official solc-bin artifact; the installed development compiler is
not used. Creation bytecode is 3,187 bytes; the runtime template is 2,969 bytes.
Runtime DOMAIN patches differ by actual deployment identity.

The transfer/approval interface follows [ERC-20](https://eips.ethereum.org/EIPS/eip-20),
including successful zero transfers and Transfer events. Transfers to zero or
the token contract are rejected. `approve` overwrites allowance; infinite
allowance is preserved by `transferFrom`. A failed transfer reverts an allowance
reduction atomically. This contract supplies neither permit nor administrative
allowance adjustment.

`creditDeposit(bytes32,address,uint64)` accepts only the zero caller, a nonzero
unique deposit ID, nonzero amount and a usable recipient. It permanently marks
the ID, increments cumulative credited units and total supply, and credits the
recipient. Outstanding supply cannot exceed u64::MAX. The constructor/deployer
receives no mint privilege. A new execution profile must reserve the zero
address (including prohibiting genesis code there) and make this call only from
CKB-authenticated, exactly-once deposit processing. **The current v1 executor
has no such hook.** Its ordinary signed transactions cannot use this entry point.
The direct VM calls in unit tests deliberately exercise contract logic; they do
not establish deposit authenticity or permissionless L1 processing.

`withdraw(uint64,bytes32)` burns only the caller's tokens and stores a permanent
commitment to the exact amount and CKB recipient lock hash. Positive amount,
nonzero recipient and sufficient balance are required. An incrementing u64 ID
starts at one and cannot wrap. Later burns never overwrite previous claims.
ERC-20 allowance does not authorize burning another user's tokens. No recipient
callback or offchain signature is required to create the commitment.

## Exact proof interface

The deployment DOMAIN is Keccak-256 of this fixed-width concatenation:

```
"TO1BRDG1" [8]
bridgeDomain [32]
EVM chain ID [32, big endian]
contract address [20]
```

The future `bridgeDomain` must commit to the CKB genesis, rollup and unique vault
identity. Use a singleton/type identity independent of the final bridge lock
arguments to avoid a circular code-hash dependency. Merely accepting an arbitrary
constructor domain or another deployment's runtime does not authenticate a vault.

A withdrawal value is Keccak-256 of:

```
"TO1EXIT1" [8]
DOMAIN [32]
withdrawal ID [8, big endian]
amount in shannons [8, big endian]
CKB recipient lock-script hash [32]
EVM withdrawing account [20]
```

Storage slots are pinned: totalSupply 0, cumulativeDeposited 1,
cumulativeWithdrawn 2, balanceOf 3, allowance 4, creditedDeposits 5,
withdrawalCount 6, withdrawals 7. A claim's storage key is
`keccak256(uint256(id) || uint256(7))`, both padded to 32 bytes. The storage proof
then follows Ethereum's secure-trie hashing of that key. See the pinned compiler's
[storage layout rules](https://docs.soliditylang.org/en/v0.8.30/internals/layout_in_storage.html).

CKB release must verify both the account code identity and this nonzero slot
against a canonical initialized SettlementTip, reproduce the full commitment,
and atomically update a vault-specific replay-protection state. The existing
[CKB state-proof verifier](SETTLED_STATE_PROOF_REPORT.md) supplies MPT verification,
but a read certificate alone supplies none of those release rules. A proof of a
balance is insufficient to authorize release: the asset must already have been
burned and the permanent claim must not have been consumed before.

## Measured evidence

- Six Rust tests execute compiled bytecode: deployment/domain separation, ordinary
  mint rejection, permanent burns and malformed/overlarge withdrawals, ERC-20
  allowance rollback/self/zero transfers, direct system-credit duplicate/limit
  controls, a 128-operation independent balance/burn model, and withdrawal-ID
  overflow atomicity. Several properties share a test function.
- Twenty-seven actual signed EIP-1559 calls across two cases run through the
  unchanged v1 Executor and independently through Geth 1.17.8 Shanghai. State,
  transaction and receipt roots, bloom, status and gas agree in every block.
  Fourteen calls revert as intended. The user-operation fixture seeds 1,000 token
  units in genesis; it **does not claim those units came from L1**. Four burns
  exhaust this fixture's supply without erasing old claims. The second case
  deploys actual creation bytecode and tests the constructor and mint boundary.
- A separate Geth VM test deploys the actual constructor, invokes 20 calls with
  explicit callers, and rejects nine controls. One zero-caller credit is followed
  by transfer and two burns; deposited = burned = 1,000, supply = 0. Geth independently
  reconstructs both packed commitments and their storage slots. This direct-call
  test is not a signed zero-sender transaction and not a new consensus rule.
- The archive reconciler checks all 27 independent transitions, fourteen signed
  reverts, final fixture accounting and the direct-call scope. Seven forged
  evidence controls fail. CI recompiles the contract, reruns current Rust and Go
  tests, regenerates the differential fixture and compares it with retained Geth
  results. Archive integrity alone does not establish current code behavior.

[Retained evidence](evidence/native-ckb-contract/source-manifest.json) contains
source/tool hashes, candidate execution and independently generated Geth outputs.
Raw run: `artifacts/native-ckb-contract`. Root Cargo.lock and the existing
execution-rules descriptor are unchanged. No bridge bytecode is installed into
an existing rollup or used by a production vault.

## Required work before custody

1. Define authenticated CKB deposit cells, their unique IDs, actual deposited
   amounts versus reserved capacity, inclusion/consumption rules and reorg
   behavior. Reject duplicate/forged/refundable-after-credit inputs at the
   publication and settlement boundaries, not just inside this ledger.
2. Introduce an explicit new execution/proof domain and canonical deposit encoding.
   Bind every system credit to authenticated L1 evidence. The proof statement and
   CKB scripts must agree on the exact deposit set and amounts. Observer replay,
   DA reconstruction and cold recovery must enforce the same interpretation.
3. Implement the unique CKB vault, replay-resistant claim consumption, immutable
   code/domain/asset binding, exact recipient payment, capacity floors and external
   fee funding. Its accounting must reconcile pending deposits, circulating tokens,
   unclaimed burns and released assets. Gas balances are separate from wrapped CKB.
4. Generate and accept the real updated execution proof; recover nonzero burn
   witnesses independently; execute actual CKB releases, duplicate/forged/redirected
   release controls and reorg recovery with the original operator stopped.
5. Add the specified xUDT domain, upgrade/old-rule exit constraints and independent
   review. Tokens held in arbitrary EVM contracts are withdrawable only when their
   contract permits an appropriate call; universal escape is not assumed.
