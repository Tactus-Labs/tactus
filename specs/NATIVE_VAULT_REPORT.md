# Native CKB vault: funded deposits and release validation core

Status: **real CKB 0.210.0 deposits measured; no L2 credit or CKB withdrawal
executed**. G7 remains OPEN. This is an isolated devnet qualification, not a
custody deployment. The fixture intentionally uses an uninstantiated settlement
identity and a separate rollup domain; its funds are not production assets.

## Measured deposits

Five transactions commit: immutable code/funding, primary vault genesis, two
independent actor deposits, and a second vault genesis used to test transaction
isolation. Nineteen negative controls fail at the exact expected program/type
boundary. Raw run: `artifacts/native-vault-OTEvFCc5`.

| Deposit | Principal | CKB cycles | Transaction bytes | Immutable receipt capacity |
| --- | ---: | ---: | ---: | ---: |
| Actor 0 | 100 CKB | 1,883,793 | 1,241 | 238 CKB |
| Actor 1 | 150 CKB | 1,877,703 | 1,241 | 238 CKB |

The primary vault ends with **640 CKB = 390 CKB fixed occupied reserve + 250 CKB
principal**. Each committed transaction pays one CKB from separate funding. The
receipt capacity is paid separately and is not credited principal. Neither record
is an EVM credit. Costs here are fixture measurements, not affordability claims.

Each deposit consumes the unique vault predecessor, increases capacity by exactly
the declared principal, and creates an immutable record in the same transaction.
The vault and receipt type groups both verify the record against this transition.
The receipt commits to sequence, EVM recipient, amount, previous/next deposit
accumulators and cumulative funded amount. Genesis identity derives from input 0
and the vault output index; occupied reserve, permissionless lock, type identity
and all non-deposit state remain fixed. A second creation of the same singleton
is rejected.

Controls cover false reserve, forged genesis identity, standalone unfunded
receipt, each record field, missing/duplicate records, underpayment, lock takeover,
reserve rewrite, mutable receipt lock, two typed custody inputs, singleton
recreation, release without proof and destruction of the funded vault. Depending
on CKB script-group execution order, a shared deposit violation can be reported
by the vault input type or receipt output type; only those exact program/type
failures count. Transaction-level capacity errors do not count as script tests.

## Release rules implemented, not yet exercised with a settled bridge proof

`proofs/ckb-native-vault` reuses the bounded Ethereum MPT library. A release must
bind to a live initialized Tip of the configured settlement type, authenticate
the exact compiled NativeCKB runtime and the nonzero permanent withdrawal slot,
reconstruct the domain/ID/amount/recipient/owner commitment, and prove that the
withdrawal ID is still unclaimed. It atomically changes a 64-level sparse Merkle
claim tree from absent to claimed, increases released principal, and reduces
vault capacity by the same amount. Other deposit state and occupied reserve stay
unchanged. IDs are positive u64 values; proofs use 64 bottom-up sibling hashes
and low-to-high ID bits. Claimed leaves cannot be reset by a supported transition.

The designated recipient output must be untyped, have empty data and the exact
committed lock hash, and contain **at least** the committed amount. Separate
funding may top up a tiny withdrawal to its output's occupied-capacity floor;
fees and any top-up cannot reduce the committed payment. Exactly one typed input
is permitted in a vault transition; other fee inputs may use arbitrary locks but
must be untyped. This prevents two vault claims from counting the same recipient
output twice. The real-node two-vault negative control exercises this shared
boundary. Asset-specific xUDT inputs require a separate future profile.

Six native tests cover wire bounds, funded deposit arithmetic, domain changes,
actual MPT verification over a **synthetic unit-test trie**, code/root/claim tamper,
replay rejection, multiple claim paths against a separately built sparse-tree
model, and lifetime counters beyond u64. These synthetic roots were never
submitted as CKB settlements. The release path still needs a real bridge
execution proof, positive CKB payouts and their adversarial/reorg qualification.

Geth independently recomputes the fixture's deployment DOMAIN and patches the
pinned compiler's immutable runtime template to check the exact account code hash.
This is a domain/code audit, not evidence that the fixture has a settled EVM state.

## Stable wire contract

Config `TO1VAU01` is 164 bytes: identity32, CKB genesis32, rollup32, EVM chain
u64 little endian, token address20, settlement type hash32. The bridge seed is
`keccak256("TO1CKBD1" || ckbGenesis || rollup || vaultIdentity)`; the token DOMAIN
then uses the [NativeCKB contract encoding](NATIVE_CKB_BRIDGE.md). The immutable
runtime hash is derived from that DOMAIN and the hash-pinned compiled template,
not chosen freely by a withdrawal claimant. The configured settlement type must
belong to the future authenticated deposit execution profile; a random Tip or an
arbitrary genesis token allocation does not establish backing.

State `TO1VST01` is 120 bytes:

| Offset | Field |
| --- | --- |
| 8..16 | Fixed occupied reserve, u64 LE |
| 16..32 | Cumulative deposited principal, u128 LE |
| 32..48 | Cumulative released principal, u128 LE |
| 48..56 | Deposit count, u64 LE |
| 56..88 | Deposit accumulator |
| 88..120 | Claimed-ID tree root |

Live capacity is `reserve + deposited - released`, checked to fit u64. Cumulative
u128 counters support repeated circulation beyond a u64 lifetime sum; with a
u64 number of positive u64 deposits the cumulative bound is sufficient.

A 124-byte `TO1DPR01` record contains sequence u64 LE at 8, recipient20 at 16,
amount u64 LE at 36, previous accumulator32 at 44, next accumulator32 at 76,
and cumulative deposits u128 LE at 108. The empty accumulator is CKB-personalized
Blake2b-256 over `tactus/o1/deposits/empty/v1 || config`. An append hashes
`tactus/o1/deposits/append/v1 || config || record[0..76] || cumulative_u128le`.
Deposit ID hashes `tactus/o1/deposit/id/v1 || config || sequence_u64le`.
Receipt type args are `TO1REC01 || vaultTypeHash`; both its type and the empty-args
program lock forbid consumption. Receipt creation requires the actual funded
vault input/output transition. This append log has no cancellation path that
could leave credited EVM tokens without backing.

Release `WitnessArgs.input_type` is `TO1WDR01`, a 272-byte state claim, ID u64 LE,
amount u64 LE, recipient lock hash32, owner20, payout output index u32 LE, 64
siblings of 32 bytes, proof length u32 LE, and the existing MPT proof framing.
The fixed prefix is 2,404 bytes; proof length is at most 128 KiB. Input/output/
dependency scans are bounded at 64 cells. Full maximum-size VM resource behavior
has not yet been qualified as a production envelope.

## MPT library regression and reproducibility

The account-proof crate now builds as an rlib for reuse, with its CKB entry point
behind the default `entry` feature. Its build script explicitly emits the
standalone static library. This changes standalone binary layout, so the full
actual A3 read-certificate suite was rerun on separate isolated ports:
`artifacts/sealed-settlement-OLe1IcTf`, **23 commits and 25 rejections**. The real
A3 proof and original settlement transaction still reconcile. No proof key or
root Cargo.lock change was needed.

- Native vault ELF SHA-256:
  `218b27d0effbf9e41f8950cc2c0f1ee08da88969dead746a68b2f580a166e877`.
- Rebuilt standalone state-proof ELF SHA-256:
  `905695dbb3dae2f71e70f3dd2354fe426269d66fdab77bbca36517b616f1202f`.
- [Deposit evidence](evidence/native-vault/0.210.0/check.json) and
  [actual MPT regression](evidence/native-vault/state-proof-regression/check.json)
  retain raw receipts, logs, configs and source/binary manifests with SHA256SUMS.

```sh
cargo test --locked --manifest-path proofs/ckb-native-vault/Cargo.toml
CKB_BIN=/path/to/ckb-0.210.0 TACTUS_DEVNET_SUITE=replay-native-vault \
TACTUS_DEVNET_RPC_PORT=18744 TACTUS_DEVNET_P2P_PORT=18745 \
scripts/run-devnet-experiments.sh
python3 -B scripts/test-native-vault.py
```

[Canonical record reconstruction](NATIVE_VAULT_RECOVERY_REPORT.md) now passes
four independent process recoveries. The next required integration is exactly-once
system credit in a new execution/proof domain, a real accepted proof containing
burns, and independently constructed vault release. These deposits alone close
none of the custody, escape, upgrade or operational production gates.
