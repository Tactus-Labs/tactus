# Two-node A3 and EVM network reorganization

Both **CKB 0.121.0 and 0.210.0** pass a controlled two-node partition and real P2P
fork-choice test. The original node replaces eight canonical blocks after learning
its peer's longer valid branch. Fresh processes recover the new A3 obligations and
EVM state, then a builder resumes mandatory processing. **No `truncate`, block
import or `submit_block` call is used.**

This is network-driven replacement in an intentionally induced laboratory fault.
It is not evidence of spontaneous public-network faults, sustained miner fairness,
validity settlement or production readiness. Full Experiment A, G2 and G6 remain
OPEN.

## Workload and observations

Two separately running nodes start with identical funded development genesis and
distinct network identities. The common branch deploys real immutable programs,
creates a four-lane mandatory A3 gate, admits four messages and advances eight
canonical batches. The messages contain signed Ethereum contract deployment and
storage-write transactions from the independent Geth fixture, plus two malformed
inputs. The two nodes synchronize this prefix through P2P.

The driver disables networking and verifies that both peer lists are empty. While
partitioned, the original node seals those four messages and executes their first
mandatory batch. The peer instead admits a signed storage-clear message to lane 0
from the common state, then generates enough valid blocks to have a longer branch.
The original node's tip must stay unchanged during this competing-branch work.

After P2P reconnection, both canonical tips and transaction pools must converge on
the peer's tip. The original node's orphaned seal snapshot is no longer live, and
the old observer's pinned block fails canonicality. Cold recovery finds five
messages, including the peer-only admission, an unsealed epoch-zero Schedule at
its eight-batch quota, and the common EVM state. Restoring the publisher's old
four-message queue would fail this check.

The builder seals the recovered five-message queue and processes it through the
normal eight-batch epoch. Deployment and storage write execute in the first
mandatory prefix; the added storage-clear transaction executes in the next. The
remaining batches complete the quota. Both nodes converge again, and independent
protocol and execution observers agree with the builder's final state.

| Measurement | CKB 0.121.0 | CKB 0.210.0 |
|---|---:|---:|
| Common canonical height | 60 | 60 |
| Original branch height before reconnect | 68 | 68 |
| Winning peer height at reconnect | 74 | 74 |
| Original canonical blocks replaced | 8 | 8 |
| CKB committed transaction events | 27 | 27 |
| Of those events, orphaned seal/batch transactions | 2 | 2 |
| Final canonical input batches | 16 | 16 |
| Recovered / subsequently published messages | 5 / 5 | 5 / 5 |
| Fresh protocol observer processes | 4 | 4 |
| Fresh EVM observer processes | 4 | 4 |

The three independently reconstructed Ethereum state roots match across versions:

| State | Root |
|---|---|
| Common prefix and restored state after reorg | `0xa7baf949c792b882fc5e72fd71c17b6268bff6c813001024a1407ba82b808155` |
| Orphan deployment/storage-write branch | `0x978dd80cce3e3f3f8e85e4ee4deb36d44a4140d20fa8ca71f3ddec1400027455` |
| Canonical replacement after storage clear | `0x37bf35b1ea8613496075eebbace0caa9d73d2c68428b9e97c8fac310156e5faa` |

The EVM observer reuses its durable journal directory across invocations, but
rescans canonical CKB publications and verifies cached prefixes before selecting a
branch. Its previously recovered longer orphan state cannot remain the current
state merely because it has more executed batches. The protocol observer has no
publisher snapshot or local journal input. The driver rebuilds its own EVM reference
from the newly canonical batch publications after reorganization; it does not
restore a saved execution object for this test.

## Isolation, synchronization and evidence boundaries

`replay-network` uses the launcher's isolated node directories, fixed development
keys and loopback interfaces. RPC and P2P ports are checked for conflicts before
startup. The second node uses the same chain specification but a separately
created runtime network identity. The launcher stops only its own two PIDs.

Network toggling and connection use the documented `set_network_active`,
`remove_node` and `add_node` APIs. The [pinned CKB RPC documentation](https://github.com/nervosnetwork/ckb/blob/v0.121.0/rpc/README.md#net-set_network_active)
defines these controls. Block creation uses the development `generate_block` API;
the common prefix and competing branch travel through ordinary P2P synchronization.
The permanent Dummy difficulty and equal compact targets are recorded. This avoids
claiming a public-network hash-power distribution from deterministic local mining.

Every synchronization boundary waits for both node tips and both pool tips, with
bounded waits that fail the suite on timeout. Transient relay rejection of a child
transaction before its parent has arrived is not counted as a protocol rejection;
only canonical commitments and final convergence decide success. The transaction
pool may re-admit orphan candidates, but the peer's new lane spend makes the old
seal stale, so the builder must reconstruct the correct new heads.

The original actor's ordinary fee-wallet bookkeeping is updated from the winning
admission and verified live; the separate builder's funding remained independent
of that admission. Cold recovery reconstructs protocol and EVM state, not a general
purpose wallet backup service. CKB consensus is trusted through the configured
full nodes. The original run used caller-supplied experimental genesis allocations. The
[allocation-bound rerun](GENESIS_ALLOCATION_REPORT.md) derives them from immutable
CKB publication; settlement verification of their state roots remains open.

## Reproduction and retained artifacts

```bash
TACTUS_DEVNET_SUITE=replay-network \
  CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-network TACTUS_CKB_VERSION=0.210.0 \
  CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
```

Default main RPC/P2P ports are 18714/18715; peer ports are 18716/18717. Override
`TACTUS_DEVNET_RPC_PORT`, `TACTUS_DEVNET_P2P_PORT`, `TACTUS_PEER_RPC_PORT` and
`TACTUS_PEER_P2P_PORT` together when running another isolated suite concurrently.
The launcher hashes the driver, both recovery binaries, all script programs,
source files and both node configurations.

[Archived evidence](evidence/network-reorg) contains raw transactions, peer lists,
fork headers, observer reports, node logs, configuration, execution genesis,
journal snapshots, manifests, summaries and checksums. Source hashes identify the
pre-commit worktree that was tested. CI now runs this suite on both supported CKB
versions; remote CI execution is not claimed for local unpushed changes.

The common A3 transaction helpers were extracted without changing protocol bytes.
Their original sealed suite was rerun on CKB 0.121.0: 210 committed events,
69 expected script rejections and 15 successful cold observers. Rustfmt, strict
workspace Clippy and all 89 workspace tests pass. Cargo.lock and all five script
ELFs remain unchanged.

## What remains open

This measures one controlled partition depth and reconnect schedule, four lanes,
one additional surviving message and two honest syncing nodes. Repeated or deeper
reorganizations, multiple competing peers, malicious miner policies, sustained
admission load, long-running observer operation and archival failures require
additional qualification. A1 and A2 have separate mechanism/recovery evidence;
this is not a claim of a complete three-arm network-fault comparison.

The new test closes the earlier gap between local `truncate` recovery and P2P
branch replacement for this workload. It does not close the proof-bound recovery
or censorship-resistance gates. No validity proof, CKB proof verification,
withdrawal authorization or release of pending obligations occurs here.
