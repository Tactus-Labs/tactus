# Independent execution comparison

Geth **1.17.8** and the pinned revm-backed Tactus executor agree on **9 scenarios,
14 blocks, 20 included transactions and 6 rejected slots**. Locally measured on
10 October 2026; not a production gate pass. The GitHub Actions job is configured
but has not been run remotely on these unpushed commits.

| Scenario | Blocks | Included | Rejected |
|---|---:|---:|---:|
| Legacy, access list, EIP-1559 | 1 | 3 | 0 |
| Invalid nonce, balance, intrinsic gas, base fee, replay; valid successors | 1 | 2 | 5 |
| Deployment, storage write/log, clear/refund | 2 | 3 | 0 |
| Revert after SSTORE and LOG | 1 | 2 | 0 |
| Exceptional halt after SSTORE and LOG | 1 | 2 | 0 |
| Shanghai SELFDESTRUCT and subsequent call | 2 | 2 | 0 |
| Block environment, parent BLOCKHASH, intervening empty block | 3 | 2 | 0 |
| SHA256 and identity precompiles | 1 | 2 | 0 |
| Nearly exhausted block gas and next-block retry | 2 | 2 | 1 |

The comparison checks state, transaction and receipt trie roots, logs bloom,
block gas, each included transaction's receipt status and gas, and rejected input
indices. Geth applies each block to its **own previous post-state**, independent
of revm. The raw signed transaction fields are passed to Geth; its independently
calculated transaction root must match the root of Tactus's signed byte inputs.

The supplied block environment intentionally matches the rollup profile. Geth
validates EVM transitions, not the rollup outcome commitment, CKB anchors or proof
settlement. For the BLOCKHASH case, Geth receives the executor's prior rollup
header hashes as environment inputs; this is not an independent check of rollup
header hashing. Empty withdrawal lists and no rewards match the profile. Malformed
raw envelope totality and forbidden unprotected signatures are separately tested
in Rust because these are rollup input-admission decisions.

The [frozen Geth result](test-vectors/execution-v1/geth-1.17.8.json) includes genesis,
batch input, all Geth results and the binary hash. The normal Rust workspace test
replays these inputs and checks the frozen independent roots. The [raw archive](evidence/execution-v1/geth-raw.tar.gz)
contains all per-block input allocations, environments, signed transaction JSON,
Geth output allocations, bodies, result JSON and process logs. Its
[manifest](evidence/execution-v1/manifest.json) records compiler, module checksums,
source hashes and commands; [SHA256SUMS](evidence/execution-v1/SHA256SUMS) protects
the portable evidence files. Regenerating vectors alone is not a successful
comparison: the Python checker must invoke Geth and fail on any disagreement.

Reproduce from the repository root with Go 1.26.8 and Rust 1.92:

```bash
mkdir -p artifacts/evm-tools
GOBIN="$PWD/artifacts/evm-tools" go install github.com/ethereum/go-ethereum/cmd/evm@v1.17.8
cargo run --locked --quiet -p tactus-o1-execution --example execution-vectors -- artifacts/evm-differential
python3 scripts/check-execution-geth.py artifacts/evm-tools/evm artifacts/evm-differential specs/test-vectors/execution-v1/geth-1.17.8.json
cargo test --locked -p tactus-o1-execution
```

Still outstanding: the full Ethereum state-test corpus, broader randomized and
precompile differential testing, crash/reorg recovery, RPC tool integration, real
proving and on-chain verification. See [execution rules](EXECUTION_V1.md) and
[production readiness](PRODUCTION_READINESS.md).

Independent implementation: [Go Ethereum v1.17.8 transition tool](https://github.com/ethereum/go-ethereum/tree/v1.17.8/cmd/evm/internal/t8ntool).
