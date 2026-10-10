# Canonical read-only Ethereum observer

Measured on CKB **0.210.0**, 10 October 2026. This is a local read-only RPC
subset, not a production gateway, transaction admission service, or wallet
compatibility certification. G1–G9 remain open.

## Implementation

`services/observer-rpc` recovers the canonical SettlementTip and publications at
one pinned CKB block through the existing authenticated recovery code. It then
replays the complete publication prefix with the same executor. The separate
Cargo workspace adds HTTP dependencies without changing the root lock file,
execution rules, or proof input. CI checks shared package versions/checksums.

The service atomically replaces snapshots, clears the cache after failed
recovery, expires snapshots after 30 seconds, and verifies the CKB pin before
and after each query/batch. A node failure or changed pin returns `-32001`,
retaining request IDs. Recovery trusts the configured CKB node's consensus/script
validation; it does not independently rerun Groth16 verification.

Supported methods:

- `web3_clientVersion`, `net_version`, `eth_chainId`, `eth_blockNumber`.
- `eth_getBlockByNumber`, `eth_getBlockByHash`, with hashes or full transactions.
- `eth_getTransactionByHash`, `eth_getTransactionReceipt`.
- `eth_getBalance`, `eth_getTransactionCount`, `eth_getCode`, `eth_getStorageAt`.
- `tactus_getStatus`, reporting the CKB pin and separate published/proved counts.

Block/transaction/receipt queries retain the reconstructed prefix. State queries
support genesis and the latest reconstructed state; other historical state is
explicitly unavailable. `latest` means published and executed, which can be ahead
of proof settlement. `safe`, `finalized`, and `pending` return an explicit error
because their policies are not defined. Malformed input slots have no Ethereum
transaction index or receipt. Unknown hashes return null.

The wire formats follow the [Ethereum JSON-RPC reference](https://ethereum.org/developers/docs/apis/json-rpc/)
and [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification): canonical
hex quantities, JSON-RPC IDs, batches, errors and notification suppression.
Only positional method parameters are supported. Notifications return HTTP 204.
Limits are 64 requests per batch, 64 KiB request bodies and eight active query
workers; both listeners and upstream CKB addresses must be loopback.

## Measured evidence

`scripts/qualify-observer-rpc.py` copied the **stopped**, previously qualified A3
republication node database into `artifacts/observer-rpc-l8qnnm_c`, cleared peer
addresses in the copy, and started only its own CKB/service children on
18744/18745 and 18545. The user's node was not contacted. Evidence is retained
under `specs/evidence/observer-rpc` with SHA256SUMS and raw process logs.

All **31 actual HTTP exchanges** passed:

- Nine published batches, zero proved batches, no withdrawal authority.
- Block 9 hash `1030192b9be2e4061b123f3224090a9d3a9eb868c5554d78fbd2a28db6ad78cd`
  and state root `1dc8ce7e500d0a3107311c94119589e9208e8517992ac275041a0242ebd5fd45`
  match the already retained A3 proof input; those execution roots also have
  [independent Geth evidence](SEALED_SETTLEMENT_REPORT.md).
- Three transaction/receipt pairs have dense indices 0–2 despite the malformed
  third input slot. Gas used is 21,000 / 21,000 / 25,300; sender nonce becomes 3.
- Hash and number lookup, full transactions, balances, code, storage, unknown
  hash, unsupported tags, malformed quantities, mixed batches and notifications.
- Stopping the owned CKB process returns an unavailable error with the original
  request ID. Restarting that same database restores the same block and status
  without restarting the observer.

The independent retained-evidence checker rejects five corruptions. Four Rust
protocol tests cover IDs, notifications, malformed batches and quantity/hash
encoding. The existing 22 driver tests still pass after exposing the common
pinned recovery result. Clippy passes with warnings denied.

## Running

```sh
CARGO_TARGET_DIR="$PWD/artifacts/observer-rpc-target" \
  cargo build --locked --manifest-path services/observer-rpc/Cargo.toml
artifacts/observer-rpc-target/debug/tactus-o1-observer-rpc config.json
```

The retained `config.json` is a reproducible laboratory example. For another
chain, supply that chain's genesis hash and exact immutable Anchor and
SettlementTip type scripts. Do not reuse the laboratory identities.

```sh
python3 -B scripts/test-observer-rpc.py
python3 -B scripts/qualify-observer-rpc.py /path/to/ckb-0.210.0 STOPPED_A3_LAB_DIR
```

## Remaining limits

Each refresh still scans canonical history and replays from genesis. The
`max_batches` setting rejects an oversized recovered prefix before the observer's
replay, but does **not** bound the upstream recovery scan or its memory. This
must be replaced with bounded incremental indexing and durable checkpoints
before production. No throughput or sustained-load claim is made.

There is no `eth_call`, gas estimation, log filtering, mempool, transaction
submission, signing, WebSocket subscriptions, arbitrary historical state,
EIP-1898 state selectors, or external exposure/authentication design. This
fixture has empty logs and no contract creation; those RPC representations
need separate qualification. The live RPC process has not yet been observed
across an actual P2P reorg; its pin checks alone are not that evidence. The
underlying recovery path has separate P2P evidence. Proof-covered A3 status
will require the real proof currently being generated.
