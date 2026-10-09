# CKB input publication to independently recovered EVM state

**Measured on CKB 0.121.0 and 0.210.0; G5/G6 and production readiness remain OPEN.**
The `replay-evm` suite now publishes real signed Ethereum deployment/storage and
transfer inputs through the immutable-data anchor, then reconstructs their EVM
state in separate processes using canonical CKB blocks and a pinned genesis.
No publisher state snapshot, indexer or unpublished witness cache is supplied.

Both versions passed the same scenario and produced the same final state:

| Measurement | Both CKB versions |
|---|---|
| CKB committed transaction events | 5, including one later orphaned |
| Final canonical input batches / EVM blocks | 2 / 3 |
| Included Ethereum transactions / rejected input slots | 3 / 2 |
| Canonical input bytes recovered | 1,033 |
| Independent recovery process invocations | 8: 7 successful, 1 expected wrong-chain rejection |
| CKB cycle estimates | 5 samples, 1,620,743–1,786,994 |
| Final Ethereum state root | `0xa46880a2c6bfef65be5465910ac75d23525eca0275bf6bbe5fc6d5076a32bb2c` |
| Final rollup block hash | `0xe03de33b651914cb524d442a239def6164329c9387d147b0014ae9537c40ecbc` |

Cycle estimates measure **CKB admission scripts**, not EVM execution or proving
cost. The two rejected input slots are malformed bytes and a duplicate Ethereum
nonce, handled by deterministic execution outcomes; they are not rejected CKB
transactions. No proof or settlement transaction is present.

## What the experiment actually exercises

1. Deploy immutable code, create a unique anchor with the pinned execution rules,
   and independently recover the genesis EVM state.
2. An independent CKB fee payer publishes a two-block batch: malformed input,
   contract deployment, duplicate-nonce input, storage write/log, then an empty
   block. A separate process scans CKB, executes, persists and agrees with the
   publisher's header and state root.
3. Mine an ordinary extra CKB block. A snapshot pinned to the earlier height
   remains valid: tip growth must not be confused with a reorg.
4. Publish and execute a branch that clears the contract's storage. Recover it in
   another process, then deliberately truncate the isolated devnet to its parent.
   The orphan snapshot is rejected and execution rolls back to the prior state.
5. A different CKB fee payer publishes a replacement branch containing a signed
   value transfer. Contract storage remains present. The replacement state root
   differs from the orphan root, and a brand-new observer directory reconstructs
   exactly the same header/state as the publisher. Restarting the older observer
   also agrees.
6. Alter the local CKB network binding and require recovery to reject it.

The orphan state root is
`0x234493f46ed8fc6139c1865452251c29dc2a3680c795627c56a3bf675ded6da5`;
the rolled-back state root is
`0xd5334dddeea4ed8b6fdb0f7c5396d39692b700b962a7ee4e0ee3f6c75dcbb8b5`.
The final root in the table is distinct from both.

## Canonical snapshot and journal contract

Recovery pins a CKB block height/hash, scans its ancestors and validates the
complete anchor succession and immutable input publications. It then checks that
`get_block_hash(pinned_height)` still matches. A newer descendant tip is allowed.
The caller supplies the exact type script and trusted execution genesis; the
recovered genesis anchor must match its identity, chain ID and execution rules.
External script bytes are strictly checked for canonical Molecule offsets,
lengths and hash type before use.

The local recovery directory pins CKB genesis hash and exact type-script bytes,
and holds an exclusive identity-journal lock throughout branch updates. Each
final batch commitment has a separate append-only execution journal. Existing
records are replayed and checked against the recovered canonical prefix before
missing inputs are appended. Old branch journals are retained, never reused as
canonical merely because they contain more blocks.

Canonicality is checked before replay, before publishing `current.json`, and
again before returning success. A reorg during replay leaves the prior pointer
alone. A reorg during pointer publication returns an error; the saved checkpoint
can be stale and **must not be served without a fresh canonical check**. The
public recovery function always scans CKB again; it never trusts `current.json`.
The checkpoint records its pinned CKB height/hash and explicitly says
`settled: false`. No snapshot can guarantee it will remain canonical afterward.

Three focused recovery tests additionally inject a replay-time reorg, a
publication-time reorg, and wrong chain/type/genesis bindings. Strict script
parsing tests every truncated prefix and malformed offsets. The pre-existing
batch-input suite was rerun after extracting shared publication helpers: its
maximum-size publication, 24 negative controls and input-only reorg recovery
still pass on CKB 0.121.0.

## Reproduction and artifacts

```bash
TACTUS_DEVNET_SUITE=replay-evm CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-evm TACTUS_CKB_VERSION=0.210.0 CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

For an existing configured chain (read-only RPC; writes only local journals):

```bash
TACTUS_CKB_RPC_ADDR=127.0.0.1:8114 cargo run --locked --bin recover-execution -- execution-genesis.json TYPE_SCRIPT_MOLECULE_HEX RECOVERY_DIRECTORY
```

The parent of the recovery directory must exist. Failed RPC, changed pinned
block, wrong genesis or journal corruption produces a nonzero exit status.

Portable [0.121.0 summary](evidence/ckb-evm-recovery/ckb-0.121.0-summary.json) and
[0.210.0 summary](evidence/ckb-evm-recovery/ckb-0.210.0-summary.json) link the raw
transaction evidence hashes. The same directory contains compressed complete
reports, CKB configurations, execution genesis, final checkpoints, source/binary
manifests and [SHA256SUMS](evidence/ckb-evm-recovery/SHA256SUMS). The execution-rule
domain changed with the added workspace dependency, so the independent Geth
vectors and their raw evidence were regenerated and verified again.
CI now includes this suite in both CKB versions; remote CI has not run on these
unpushed local changes.

## Remaining production work

The CKB node is the consensus trust boundary; this is not a CKB light client.
Genesis allocation is caller-pinned, not yet authenticated by a settlement cell.
The reorg is a deliberate isolated-node truncate, not an unplanned network fork.
Recovery rescans chain history and replays from genesis for a new final-commitment
journal; incremental checkpoints and efficient long-running synchronization are
still required. There is no continuously running observer or archival-retention
qualification yet. The complete G6 requirement also needs a separate prover to
reconstruct proving inputs and settle a batch. Actual validity proofs, CKB
verification, priority enforcement and safe exits remain unimplemented.
