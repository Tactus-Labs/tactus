# Live Ethereum RPC across actual CKB P2P branch replacement

Measured on CKB **0.210.0**, 10 October 2026. The same observer process now
tracks A3 publication rollback and republication over real peer synchronization.
This does not qualify proof-covered RPC settlement or production finality.

## Experiment and observed result

The existing [A3 obligation reorg](OBLIGATION_REORG_REPORT.md) now optionally
launches the read-only observer after publication 9. An RAII guard owns and
stops that service process. The existing harness owns the two CKB nodes.
Independent cold recovery still checks both nodes at each transition.

1. The observer serves block 9, its three transactions and three receipts.
2. The longer competing branch replaces eight CKB blocks, removing the seal
   and mandatory publication. No `truncate` or `submit_block` is used.
3. After both nodes converge, the same observer refuses the obsolete pin while
   it refreshes (11 unsuccessful polls), then serves block 8. The old block
   hash, all three transaction hashes and all three receipts return null.
4. The recovered seal and identical mandatory payload are republished. The
   same process serves block 9 with exactly the original header, transactions
   and receipts. CKB transaction identities changed, while the prescribed EVM
   execution result remained identical.

At all three observations, `tactus_getStatus` matches the phase's actual CKB
pin, reports published counts **9 → 8 → 9** and proved count **0**. No process
restart, proof-settlement, safe/finalized or withdrawal claim is made. The
probe fails if it obtains the old head after canonical convergence. This is a
controlled phase-boundary experiment, not sustained concurrent query traffic
through every instant of network synchronization.

Raw run: `artifacts/sealed-settlement-tQWjyCbO`. Retained archive:
`specs/evidence/observer-reorg`, including the HTTP-returned views in canonical
transaction evidence, raw node/peer/service logs, configuration, binaries/source
hash manifest, and SHA256SUMS. Observer PID was 190050 in all three phases.
The canonical transaction experiment has 20 historical commits (including
orphaned transactions) and five negative controls.

`scripts/check-observer-reorg.py` first reconciles the entire A3 P2P experiment,
then joins each returned RPC view to its phase and execution results.
`scripts/test-observer-reorg.py` rejects seven corruptions: orphan transactions,
orphan receipts, orphan block lookup, a service restart, false proof settlement,
wrong CKB pin, and changed receipt gas. All passed. Root driver Clippy and 22
unit tests pass; root Cargo.lock remains unchanged.

## Reproduce

Build the observer as described in [its report](OBSERVER_RPC_REPORT.md), then:

```sh
CKB_BIN=/path/to/ckb-0.210.0 \
TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_OBLIGATION_REORG=1 \
TACTUS_OBSERVER_RPC_BIN="$PWD/artifacts/observer-rpc-target/debug/tactus-o1-observer-rpc" \
TACTUS_CKB_RPC_ADDR=127.0.0.1:18744 TACTUS_DEVNET_RPC_PORT=18744 \
TACTUS_DEVNET_P2P_PORT=18745 TACTUS_PEER_RPC_ADDR=127.0.0.1:18746 \
TACTUS_PEER_P2P_PORT=18747 scripts/run-devnet-experiments.sh
```

The optional probe uses loopback port 18545 and refuses an occupied listener.
The launcher rejects this option outside the pending-duty P2P reorg mode. The
observer's full-history reconstruction and current/genesis-only state queries
remain limitations; no throughput, contract-log, or wallet certification follows
from this experiment.
