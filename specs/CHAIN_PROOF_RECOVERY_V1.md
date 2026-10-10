# Canonical input proving and cold SettlementTip recovery

Measured on 10 October 2026. The unchanged SP1 guest executed the canonical
bootstrap export under real CKB/Anchor/SettlementTip identities and matched all
768 expected public bytes. This archive captures execution before Groth16
completion; it does **not** claim an accepted settlement transition. The full
proof job remains separate from the completed checks below.

## Independent recovery

`recover-settlement CKB_GENESIS_HASH ANCHOR_TYPE_SCRIPT_HEX SETTLEMENT_TYPE_SCRIPT_HEX`
reads canonical blocks from the selected RPC node. It binds the supplied complete
deployment scripts to the chain and immutable allocation, scans to a pinned
canonical block, follows unique connected Tip successors, checks atomic Anchor/Tip
genesis and preserves capacity/lock/type. For initialized Tips it reexecutes the
exact published prefix and compares the complete Anchor, state root and header
hash. It rejects destruction, ambiguous or disconnected successors, nonadvancing
cursors, unavailable input prefixes, wrong roots and a changed pinned chain. A
final live-cell check confirms the exact latest output and data before returning.

The tool uses no local execution or prover cache. The RPC node remains the
consensus and script-validity trust boundary; this is not a light client or an
independent Groth16 verifier. `settled_batches` and `published_batches` are
separate. An uninitialized Tip exposes zero proved roots even though allocation
and published transactions can already be replayed. No output authorizes asset
release.

Both CKB 0.121.0 and 0.210.0 independently recover the same uninitialized Tip with
one published batch and zero settled batches. Each fresh-process run rejects a
wrong network, wrong Anchor identity and changed settlement guest key. The
bootstrap's four committed records and 20 script rejections still pass. A unit
test checks replay binding against real signed execution data, including forged
state/header roots, cursor, reserved bytes and premature initialization.
[Raw evidence and source manifests](evidence/chain-proof-recovery/) are retained.

## Real-domain proving runner

The separate `chain-proof` host loads a bounded canonical input export, checks its
schema and nonempty interval, independently replays allocation and batches,
compares the expected journal, checks the actual ELF verification key against the
deployment key and runs the unchanged guest. `verify` reopens an existing proof
and repeats the exact input/journal/key binding in a fresh process. It refuses a
verification-bypass environment. Four measured input controls reject a changed
journal, empty interval, unknown schema and verification bypass before creating
an output directory or initializing the expensive prover.

```bash
SP1_GROTH16_CIRCUIT_PATH=/absolute/path/to/circuits/groth16 \
  scripts/run-chain-proof.sh /absolute/path/to/proving-input.json
```

The runner requires the already built pinned guest, uses release circuits,
records input/binary/source hashes, limits worker and Go concurrency, serializes
local proof jobs with the existing lock, and assigns a dedicated temporary
directory so an OOM does not silently lose retained recursive witnesses. A fresh
verification follows a successful complete proof. The measured launch additionally
used a systemd scope with MemoryHigh=20G, MemoryMax=23G and MemorySwapMax=6G; these
are workstation experiment limits, not production memory qualification.

## Pending positive settlement qualification

The bootstrap driver accepts `TACTUS_SETTLEMENT_PROOF_DIR` only for a completed
real Groth16 result with the exact regenerated canonical journal and guest key.
Its optional qualification path preflights and commits the genuine proof,
requires a spent predecessor and exact live successor, then attempts replay on
that successor and rollback to the initial state. Controls include all public
fields, proof-byte changes, and coordinated changes to both journal and output
roots that must reach and fail the actual cryptographic verifier. Cold recovery
then independently reconstructs the proved successor. There is no mock-success
mode. These positive-path assertions are implemented but **not measured as passed
in this archive**; they require the running real-domain proof to complete.

```bash
TACTUS_SETTLEMENT_PROOF_DIR=/absolute/path/to/completed/proof \
TACTUS_DEVNET_SUITE=replay-settlement-bootstrap \
CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
python3 scripts/check-first-settlement.py /absolute/path/to/evidence.json /absolute/path/to/completed/proof
```

The independent Python checker reconciles proof witnesses, complete successor
data, checkpoint dependencies, fees, node-packed size, exact script rejections,
and cold recovery. It performs no cryptographic verification itself. Multiple
proved intervals, reorg rollback, another prover's completion, proof-bound
obligations, custody/exits and all remaining production gates stay open.

## Sequential proof qualification

The driver also accepts `TACTUS_SECOND_SETTLEMENT_PROOF_DIR` with a completed
first proof directory. It publishes the second canonical batch before applying
either proof, rejects applying interval two to the initial Tip, and then applies
both proofs in order. This exercises first-proof settlement using its immutable
checkpoint after the mutable Anchor has already advanced. The second phase checks
both predecessor roots, coordinated successor mutations, replay of either proof,
rollback and fresh recovery of two proved transitions. The accompanying
`scripts/check-two-settlements.py` reconciles the two publication/proof phases,
actual fee inputs, exact retained proof bytes, expected script failures and cold
recovery. Its intended seven commits and 62 rejections remain assertions awaiting
successful runs, not measured results.

On CKB 0.210.0, a measured negative run supplied the previously completed genuine
synthetic-domain Groth16 proof to the canonical-deployment loader. The loader
rejected the journal mismatch; the suite records `complete=false`, `settled=false`
and the exact error. This validates the host guard, not a positive on-chain proof
transition. Two CLI controls reject missing first-proof prerequisites and mixed
preparation/application modes before connecting to a node. Artifact reads are
bounded to 4,096 proof bytes, 768 public bytes and 65,536 metadata bytes.
[Guard logs and exact source proof](evidence/settlement-proof-guards/) are retained.

```bash
TACTUS_SETTLEMENT_PROOF_DIR=/absolute/path/to/completed/first/proof \
TACTUS_SECOND_SETTLEMENT_PROOF_DIR=/absolute/path/to/completed/second/proof \
TACTUS_DEVNET_SUITE=replay-settlement-bootstrap CKB_BIN=/absolute/path/to/ckb-0.210.0 \
  scripts/run-devnet-experiments.sh
python3 scripts/check-two-settlements.py /absolute/path/to/evidence.json \
  /absolute/path/to/completed/first/proof /absolute/path/to/completed/second/proof
```
