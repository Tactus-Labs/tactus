# Genesis publication and recovery evidence

The current anchor deployment binds its initial account allocation in its type
arguments and atomically publishes canonical immutable allocation bytes. A fresh
execution observer can now recover genesis from CKB without the operator's
allocation file. A supplied different allocation is rejected before journal
adoption. [Exact bytes, limits and deployment rules](GENESIS_ALLOCATION_V1.md)
define this boundary. Production and proof-settlement gates remain OPEN.

## Validation

Both CKB 0.121.0 and 0.210.0 run the complete updated mechanism matrix:

| Suite | Committed transaction events | Expected CKB rejections | Relevant result |
|---|---:|---:|---|
| Batch/genesis publication | 7 | 32 | Exact maximum allocation and batch data; genesis attacks rejected |
| Independent EVM recovery | 5 | 0 | Chain-derived allocation agrees; mismatched allocation and chain rejected |
| Authenticated A2 obligations | 24 | 20 | Bounded authenticated obligations; challenge-only forced inclusion still fails |
| Mandatory A3 sealed gate | 210 | 69 | Full queues, snapshots, rollback and independent cold recovery |
| Two-node P2P reorg | 27 | 0 | Canonical A3/EVM recovery using chain-derived genesis |
| Matched admission contention | 290 | 36 | Same-lane contention remains; independent paths admit both actors |
| Original A1/A2/A3 controls | 198 | 32 | Bare-head reference and original mechanism baselines remain separate |

Counts are transaction events, including planned/network-orphaned transactions
where the suite explicitly exercises them. CKB rejection counts do not include
host-process negative controls or rejected Ethereum input slots. These are bounded
mechanism regressions, not an aggregate production throughput measurement.

The seven new genesis creation attacks are: no allocation output; incorrect
allocation commitment; spendable allocation lock; noncanonical suffix; duplicate
account addresses; the old allocation-unbound type format; and an allocation above
262,144 bytes. Each rejection must come from the exact expected anchor program and
error code. A positive allocation containing eleven bounded code-bearing accounts
is exactly 262,144 bytes, commits on CKB and is recovered byte-for-byte. Attempting
to spend that publication also fails. Existing maximum batch-publication and
planned-reorg controls still execute afterward.

The EVM suite runs eleven separate recovery processes, including a fresh
`--chain` observer with no local allocation file, a wrong-allocation rejection,
a wrong trusted CKB genesis rejection, and the existing wrong recovery-directory
network rejection. It compares canonical headers and state roots through rollback
and replacement. The two-node network suite now uses chain-derived genesis for
all four independent EVM observations; it still executes signed deployment,
storage write and clear through mandatory A3 prefixes.

Six allocation codec tests use an independent byte/hash vector and adversarial
boundary cases. A journal test verifies that zero storage entries in local JSON
and their canonical omission resolve to the same genesis identity. All **96
workspace tests**, Rustfmt, strict workspace Clippy and RISC-V anchor Clippy pass.

The execution rules descriptor now includes allocation limits and normalization.
This changes the rules hash, so the independent Geth 1.17.8 comparison was
regenerated and run again: **9 scenarios, 14 blocks, 20 included transactions and
6 rejected slots** agree. The frozen fixture and [raw Geth evidence](evidence/execution-v1)
were refreshed from the actual independent run, not just from the Rust generator.

Parallel validation also exposed a shared-artifact linker race. The build script
now locks its build/link phase and atomically publishes completed ELFs. Two
simultaneous script builds both complete with identical five-program checksums;
the final dual-version matrix runs with that corrected launcher path.

## Evidence and reproduction

[Retained evidence](evidence/genesis-allocation) contains each suite's raw results,
source/program/configuration manifests, summaries and replay logs, plus checksums.
The genesis/EVM and network entries retain the public genesis configuration and
observer artifacts where applicable. Source hashes identify the exact pre-commit
worktree. CI already runs these seven suites on both supported CKB versions; local
success is not a claim of remote CI execution on unpushed changes.

```bash
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
TACTUS_DEVNET_SUITE=replay-batch CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
TACTUS_DEVNET_SUITE=replay-evm CKB_BIN=/absolute/path/to/ckb-0.121.0 scripts/run-devnet-experiments.sh
```

Set `TACTUS_CKB_VERSION=0.210.0` and its matching binary for compatibility runs.
Use the other suite names listed in the launcher to reproduce the full matrix.
The new profile requires a fresh allocation-bound deployment; it does not migrate
old allocation-unbound anchors or their journals in place.

## Remaining production boundaries

The allocation is now unambiguous and recoverable for the configured deployment.
It is not a proof of the initial Ethereum state root, a supply-conservation bridge,
a verifier/upgrade policy or proof-backed execution. The experimental 18.45 ETH
maximum initial supply, genesis resource limits, L1 timestamp constraints, proving
liveness, custody/exit conservation, sustained admission fairness and long-running
operational qualification still need completion. No production gate is passed by
publishing an allocation commitment alone.
