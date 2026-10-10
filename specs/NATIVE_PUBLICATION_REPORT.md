# Authenticated native custody publication on CKB 0.210.0

Status: **funded records authenticated by the publication script; new proof
settlement and actual CKB payouts remain unimplemented**. This is a new standalone
native publication program, not an upgrade of the existing A3 deployment. G1–G9
remain OPEN. The fixture settlement type hash is deliberately `[0x05; 32]`, with
no corresponding deployed settlement verifier or release authority.

The actual-node experiment commits **ten transactions** and rejects **27 exact
script controls**. Two independent actors deposit 100 and 150 CKB into the pinned
vault. Other actors publish those receipts with two signed EVM withdrawal calls.
The candidate v2 replay burns 40 and 50 CKB of token units, leaving 160 circulating
and 90 burned but unreleased. The vault still holds **640 CKB: 390 occupied reserve
+ 250 deposited principal**. These are local execution results for authenticated
published input, not proof-settled balances or CKB payments.

Geth 1.17.8 independently reproduces both Ethereum blocks, system credits and
signed burns, including state/transaction/receipt roots, encoded receipts, gas,
logs, bloom and header hashes. A separate cold vault process reconstructs exactly
the same two funded records from canonical CKB history and rechecks live cells.
The EVM oracle explicitly does not claim independent CKB consensus verification.

## Publication authority

The program pins the actual native vault Data1 code hash
`3b0c9f82f019407ad1784fcf0d62fe695eba3cf235c2e8ce474af5aebbe39237`, the permissionless
head-lock program, the v2 execution rules hash and compiled NativeCKB runtime.
Its build rejects a different vault executable. Code identity is immutable via
the publication type script; allocation and published batch cells are untyped
and locked by this program with empty arguments, which always returns error 1.

Type arguments are `orderingIdentity32 || allocationCommitment32`. Cell data is
`TO1NAP02 || vaultConfig164 || nativeAnchor256`, totaling **428 bytes**. The config
is fixed at genesis and must match byte-for-byte on every transition. It includes
the vault identity, CKB genesis, rollup, EVM chain/address and settlement type.
The base anchor's execution rules and domain are checked against pinned values.

Genesis must create exactly one ordering output under the type-bound head lock
at its exact **598 CKB occupied reserve**. Ordering identity derives from input
zero and the actual output index. Header dependency zero must be the actual
height-zero header and hash to the configured CKB genesis. The same transaction
must create the exact configured vault output, with no predecessor of that vault
type among its inputs. The pinned vault program executes its own empty-genesis
and identity checks in that transaction. A cell dependency cannot substitute for
joint vault creation.

Genesis also publishes the exact committed allocation under the immutable lock.
The script validates its bounded canonical encoding, prohibits zero-address
authority, and requires the domain-patched NativeCKB code with nonce one, zero
native balance and absolutely no storage. The bridge address cannot be a
Shanghai precompile. This closes the allocation path to seeded unbacked tokens.
The full vault config is stored in genesis-authenticated data rather than type
arguments, avoiding the ordering/settlement/vault type-hash cycle.

An append consumes and recreates the single ordering cell without changing lock,
capacity or config. `WitnessArgs.input_type` points to an untyped immutable output
containing the complete `TO1BRG02` wrapper. The shared v2 validator checks record
order, amounts, recipients, accumulators, bounds, user framing and exact next
anchor/cursor. For **every included deposit**, the script requires a live CKB
dependency with the exact pinned vault receipt type, immutable vault lock and
complete 124-byte record. Byte-identical untyped copies, another vault's genuine
records, and self-consistent forged transcripts are insufficient.

Input, output and expanded dependency scans are bounded at 64 cells; the type
group has at most one predecessor and exactly one successor. Witness allocation
is bounded at 4,096 bytes. The candidate wrapper remains limited to 262,144 bytes
and 32 deposits. The maximum-size CKB-VM resource envelope has not been qualified.

## Measured results and controls

| Authenticated publication | CKB-VM cycles | Wire bytes | Principal credited in replay |
| --- | ---: | ---: | ---: |
| Batch 0 | 1,975,817 | 1,933 | 100 CKB |
| Batch 1 | 1,990,972 | 1,933 | 150 CKB |

Every committed transaction pays one CKB from external fee funding; the audit
reconciles all known input/output capacities. Receipt and data-cell capacity is
separate from deposited principal. This laboratory fee is not a production fee
recommendation or an affordability qualification.

Eleven genesis controls reject missing/wrong network headers, nonempty cursor,
missing joint vault, wrong lock/capacity/rules, seeded token storage, zero caller,
wrong bridge code and forged ordering identity. Thirteen append controls reject
missing/untyped/foreign receipts, valid-hash forged amounts or recipients, wrong
cursor, mutable DA, config changes, truncated input, wrong witness pointer,
capacity changes, lock takeover and a duplicate record. A further control rejects
replaying a record after its actual publication. Actual attempts to spend the
immutable allocation and batch data fail at `Inputs[0].Lock` with this program's
error 1. All other negatives require this exact program's input/output type group;
transport errors and transaction-level capacity failures do not count.

Three host tests use actual deployment bytes for genesis/runtime/wire constraints.
Native-target Clippy checks the onchain module. The shared transaction builder
now signs explicit header dependencies in both Molecule and JSON representations;
the existing 22 driver tests pass, and actual CKB accepts the signed genesis.
The four cold-recovery tests still pass. Twelve forged-evidence controls are
rejected by the independent Python archive audit.

## Reproduce and remaining work

Raw run: `artifacts/native-publication-56w3tq2H`. The archive under
`specs/evidence/native-publication/0.210.0` retains complete receipts, deployment,
configs, replay logs, EVM inputs, independent Geth outputs and source/binary
fingerprints. The vault ELF is unchanged. Native publication ELF SHA-256:
`7ec26b5ab39c2622bc9215e0c7f5e1120396364f32d7a9111fb437e60858f89e`.
Its CKB Data1 code hash is
`429ba46208c39ed0b2b0012d03148b33870885856043e368c3bb42f9b27a9a9b`.

```sh
bash scripts/build-native-anchor-script.sh
cargo test --locked --manifest-path proofs/ckb-native-anchor/Cargo.toml
CKB_BIN=/path/to/ckb-0.210.0 TACTUS_DEVNET_SUITE=replay-native-publication \
  TACTUS_DEVNET_RPC_PORT=18744 TACTUS_DEVNET_P2P_PORT=18745 \
  scripts/run-devnet-experiments.sh
# For the printed evidence directory, execute the independent oracle:
(cd services/bridge-checker && \
  TACTUS_NATIVE_PUBLICATION_VECTOR=/absolute/run/execution.json \
  TACTUS_NATIVE_PUBLICATION_GETH_EXPORT=/absolute/run/geth.json \
  go test -mod=readonly -count=1 -run TestPublishedNativeExecutionAgainstGeth .)
python3 -B scripts/check-native-publication.py /absolute/run
python3 -B scripts/test-native-publication.py
```

This program authenticates admitted deposit records; it does not force admission
of every pending deposit. A3 sealed-duty integration, 428-byte publication-state
checkpoint/reorg recovery, full v2 observer RPC and system-credit log indexing
remain required. The new guest/journal must prove the exact canonical interval
and deposit cursors; a new CKB settlement verifier must bind those outputs to the
trusted publication/vault configuration. Then a real proof, nonzero burn witnesses
and actual replay-resistant payouts must be qualified with the original operator
stopped. Native-publication P2P reorg behavior remains unqualified; earlier v1 A3
reorg evidence cannot establish it. xUDT, upgrades, independent review and the
remaining liveness/operational gates are still part of the production goal.
