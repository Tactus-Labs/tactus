# Ethereum event queries and receipt qualification

10 October 2026. The local observer now implements `eth_getLogs` over its
canonical snapshot. No execution rules, guest program or root lock file changed.
This improves application event access; full Ethereum tooling and G5 remain open.

A filter selects an inclusive block range (omitted endpoints mean `latest`) or
one `blockHash`, then optional address alternatives and positional topic rules.
Topic positions are ANDed; alternatives within a position are ORed. Null and an
empty alternative list are wildcards, but a specified position still requires
a topic at that position. The method returns canonical logs in block,
transaction and log order, with `removed: false`. Stateless queries do not
emit notifications about previously returned orphan logs.

These semantics are checked against the official
[Ethereum RPC reference](https://ethereum.org/developers/docs/apis/json-rpc/)
and the pinned [Geth 1.17.8 filter API](https://github.com/ethereum/go-ethereum/blob/v1.17.8/eth/filters/api.go).
Unknown block hashes fail explicitly. Hash/range combinations, reversed or
future ranges, malformed hashes and more than four topic positions are rejected.
The existing undefined `safe`/`finalized`/`pending` policy is preserved.

Per query: at most 1,024 blocks, 256 addresses, 256 alternatives per topic,
10,000 returned logs and 4 MiB of serialized log-array content. Exceeding a limit
returns `-32005`; the service never returns a silently truncated successful
result. The byte limit excludes the small enclosing JSON-RPC response envelope.
Clients can narrow or paginate their range. Persistent filters, subscriptions,
transaction submission, `eth_call` and gas estimation remain absent.

## Verification

The nine service tests include replay of all **nine retained independent Geth
execution cases (14 blocks)** through the actual RPC snapshot construction:

- State, transaction and receipt roots, bloom and gas match the Geth fixture.
- Transaction/receipt identity, dense indices, gas, cumulative gas and creation
  addresses match. Geth's transition-tool dummy block hash is replaced by the
  actual reconstructed Ethereum block hash when comparing log metadata.
- The deployed storage-writing contract emits two logs over two blocks. Full
  log objects match, including address, topics, data, indices and timestamp.
- Reverted and halted executions expose no rolled-back logs.
- Default, range, block-hash, address and topic queries preserve expected order.
  Separate filter tests exercise OR/AND/null/empty alternatives and bounds.
- Oversized synthetic result inputs exercise rejection rather than truncation;
  these are resource-limit unit tests, not throughput measurements.

The HTTP experiment reran the owned CKB **0.210.0** outage/restart fixture and
added nine log-filter requests: **31 baseline + 9 log requests passed**. Its A3
transfer fixture has no nonempty contract events, so the nonempty-log evidence
above is specifically executed Geth-fixture comparison, not a live CKB contract
admission test or a live Geth JSON-RPC server differential.

Raw run: `artifacts/observer-rpc-63z2liml`. Retained archive:
`specs/evidence/observer-logs`, including 40 HTTP observations, process logs,
configuration, source/binary/dependency manifest and SHA256SUMS. The independent
HTTP checker rejects five corruptions. Service Clippy passes with warnings denied.

```sh
CARGO_TARGET_DIR="$PWD/artifacts/observer-rpc-target" \
  cargo test --locked --manifest-path services/observer-rpc/Cargo.toml
python3 -B scripts/test-observer-rpc.py
python3 -B scripts/test-observer-logs.py
```

Live nonempty-log reorgs, broader topics/data workloads, third-party application
integration, persistent incremental indexing and sustained load still need
qualification. This milestone grants no proof settlement or withdrawal rights.
