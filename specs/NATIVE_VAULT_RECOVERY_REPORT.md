# Canonical native CKB vault recovery

Status: **qualified for the funded deposit fixture, not production custody**.
Only CKB 0.210.0 is in scope. This adds a read-only recovery client; it changes
neither the deployed vault program nor the existing A3 proof domain.

Four independent `recover-native-vault` processes reconstruct the target vault
at genesis, after each deposit, and after creation of an unrelated second vault.
They recover deposit counts **0 → 1 → 2 → 2**, and finish with **250 CKB deposited
plus the 390 CKB occupied reserve = 640 CKB**. No operator database, indexer or
previous process state is provided to the recovery executable. The actual-node
suite still has five committed transactions and nineteen script rejections.

## Trust and consistency

The caller pins the CKB genesis and complete vault type script, including the
immutable program hash and custody configuration. The selected CKB RPC node
supplies consensus and script validity. This is **not** an independent CKB light
client, proof verifier, L2 deposit credit, or production readiness assertion.

The client pins a header, reads every canonical block from genesis to that
height, checks parent continuity and reconstructs the singleton vault chain.
It checks funded capacity, reserve, the full immutable deposit transcript,
recipient, sequence, cumulative amount and accumulator. It rejects missing
predecessors, standalone receipts, consumed immutable receipts and additional
typed custody inputs. It derives deposit IDs from the trusted complete config.
All monetary report fields are decimal strings, including u128 cumulative sums.

Before returning, it re-reads the current vault and every receipt as live cells,
compares complete outputs/data and rechecks the pinned block hash. Reorg,
unavailable RPC data, spent custody or changed receipts cause failure and require
a fresh scan. An extension after the pin is allowed if the relevant cells still
match. The report is a snapshot, not a promise that its cells remain live after
return; any transaction builder must resolve inputs/dependencies again.

The tracker is poisoned on any failed block so callers cannot ignore an error
and publish partially applied state. Optional raw block capture is written to a
temporary file and published only after successful recovery using a link that
cannot overwrite an existing destination. Failure produces stderr and a nonzero
exit status, with no JSON success report.

The default limits are 100,000 blocks and 4,096 deposits; explicit maximums are
1,000,000 and 65,536 respectively. Missing vaults and limits fail closed. This is
a bounded full scan, **not** a durable incremental indexer or a demonstrated
mainnet recovery performance envelope. The implementation also recognizes the
release witness and verifies its MPT/claim/payout rules, but actual release
history remains unqualified. It currently resolves direct code dependencies,
not settlement Tips hidden inside dependency groups.

## Validation and use

The archive retains 21 actual blocks, all four reports, node/source/binary
fingerprints, configs, full experiment receipts and replay logs. Rust tests
reproduce all four prefixes, exercise fourteen corrupted-history cases plus
limits/domain checks, and inject eleven RPC failures (including changed pin,
reorg, missing blocks, unavailable parents and changed live cells). CLI tests
verify capture cleanup, successful publication and no overwrite. These RPC fault
tests are explicitly synthetic; they are not an actual P2P vault reorg trial.

An independent Python audit reconciles each recovered field with captured blocks
and funded deposit evidence, including twelve deliberately forged archives.
It checks evidence relationships, not consensus or cryptography.

```sh
bash scripts/build-native-anchor-script.sh
cargo build --locked --manifest-path services/native-vault-lab/Cargo.toml
TACTUS_CKB_RPC_ADDR=127.0.0.1:18744 \
  services/native-vault-lab/target/debug/recover-native-vault \
  "$EXPECTED_CKB_GENESIS" "$TRUSTED_VAULT_SCRIPT_HEX"

cargo test --locked --manifest-path services/native-vault-lab/Cargo.toml
python3 -B scripts/test-native-vault-recovery.py
python3 -B scripts/check-native-vault-recovery.py \
  specs/evidence/native-vault-recovery/0.210.0
```

The RPC address is mandatory: this executable has no default node on port 8114.
Optional `TACTUS_VAULT_CAPTURE`, `TACTUS_VAULT_MAX_BLOCKS` and
`TACTUS_VAULT_MAX_DEPOSITS` select capture and bounds. The experiment launcher
executes four fresh recovery processes automatically for `replay-native-vault`.

Exactly-once system credit must still be bound to these authenticated records
inside a new execution/proof domain. A real proof containing token burns and an
independently constructed vault release are still required. G1–G9 remain OPEN.
