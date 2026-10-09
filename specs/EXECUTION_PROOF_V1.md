# Experimental execution proof statement

`proofs/sp1` is an isolated, locked SP1 6.8.1 workspace. Its guest calls the same
serial executor used by canonical recovery. The local host explicitly selects
the CPU prover; the SDK's network and experimental features are disabled.
This work implements an execution-proof experiment, not a CKB settlement or
withdrawal authorization. G3 remains OPEN until a real CKB verifier authenticates
the deployment, history, predecessor, key and proof and advances a settlement cell.

## Witness and computation

The guest reads a 136-byte deployment domain, canonical `TO1GEN01` allocation,
prefix batch count, nonzero interval batch count, then the exact encoded batches
in that order. Each batch passes the existing bounded batch decoder and serial
Ethereum execution. Wrong predecessor/order/domain or malformed batch encoding
fails execution. Invalid transaction slots retain the executor's total outcome
semantics. The guest never accepts a caller-supplied Ethereum state root as truth.

It reconstructs the prefix from genesis, captures the preceding anchor, state root
and Ethereum header hash, executes the contiguous interval, and commits its result.
This deliberately establishes correctness before adding authenticated checkpoint
witnesses. Prefix replay grows with history; its cost is not production-qualified.
The existing experimental genesis supply and execution limits still apply.

The interval digest starts with `hash("tactus/o1/proof-interval/v1", count_le64)`.
Each ordered step is
`hash("tactus/o1/proof-interval-step/v1", previous_digest || len_le64 || batch_bytes)`.
`hash` is the protocol's CKB-personalized Blake2b-256 function. The ending anchor
also commits the existing predecessor-linked batch history. This proves execution
of that history; only a settlement verifier can establish that it is canonical CKB
history by comparing authenticated checkpoints and live settlement state.

## Public journal

All integers are little-endian and hashes remain full 32-byte strings. The journal
is exactly 768 bytes; truncation, trailing bytes and unknown profile are rejected.
There is no reduction of deployment/state digests modulo a proof field.

| Byte offset | Length | Value |
|---:|---:|---|
| 0 | 8 | `TO1PRF01` |
| 8 | 32 | Proof profile commitment |
| 40 | 32 | CKB genesis block hash |
| 72 | 32 | Ordering type script hash |
| 104 | 32 | Settlement type script hash |
| 136 | 32 | Rollup ID |
| 168 | 8 | Ethereum chain ID |
| 176 | 32 | Canonical genesis allocation commitment |
| 208 | 200 | Anchor immediately before the interval |
| 408 | 200 | Anchor immediately after the interval |
| 608 | 32 | Previous Ethereum state root |
| 640 | 32 | Next Ethereum state root |
| 672 | 32 | Previous Ethereum header hash |
| 704 | 32 | Next Ethereum header hash |
| 736 | 32 | Exact ordered interval digest |

The anchors carry batch/block cursors, last batch commitment, execution rules,
DA policy and admission limits. Both must agree with the deployment domain and
the executor's pinned rules. The interval must advance the batch cursor.

The profile explicitly excludes withdrawal authorization: no withdrawal tree or
custody conservation proof has been implemented. A future custody statement needs
those commitments and a reviewed version transition. The guest/key and proof-system
identities are authenticated by the verifier's selected ELF-derived key and pinned
proof implementation, rather than an unauthenticated key supplied by the prover.

## Local checks and limitations

The first [retained guest execution](evidence/proof-execution) runs fixture 0's
three signed transfers in one block. Its 768-byte journal matches native execution
and the retained Geth roots; it executes 15,851,885 RV64 instructions. The measured
39.454 seconds includes CPU prover initialization (34.496 seconds) and guest setup.
This archive establishes actual guest execution only: cryptographic proof
generation was still running when it was captured. It retains the guest ELF,
compiler/source hashes, exact launch-time lock and the earlier unsupported-RVC
failure. These are workstation observations, not sustained proving-capacity claims.

The journal tests use all nine retained Geth fixtures, including a nonzero-prefix
case reconstructed as two consecutive batches. They check exact native/Geth state
roots, ordered history, allocation changes, domain changes and canonical encoding.
These ordinary tests are not proof verification.

The CPU host checks native state/transaction/receipt roots against the independent
Geth fixture, executes the actual guest, compares the complete journal, and in
`prove` mode generates a real core STARK and verifies it with the ELF-derived key.
It then changes public fields and checks rejection. `verify` loads the retained
proof in a separate process and requires both cryptographic verification and the
exact expected journal. Laboratory domain hashes `01/02/03` are explicit test
identities, not authenticated CKB deployments.

Core STARK verification on the host is not the final CKB verification path. Proof
compression, CKB-VM cost, production key/setup provenance, authenticated history
and SettlementTip transitions remain required. Reproducibility must also qualify
the currently available custom Rust toolchain, rather than treating its existence
as production provenance. The root workspace lock and execution rules remain
unchanged; the isolated proof workspace records its additional dependencies.

## Reproduction

The reference build uses Rust 1.97.1 for the local host, the available `succinct`
Rust toolchain for the RV64 guest, Clang, and the official SP1 6.8.1 CLI commit
`c84ada1`. `TACTUS_CARGO_PROVE` selects that executable. The build script explicitly
sets `-march=rv64im -mabi=lp64`: Clang's default target otherwise introduces
compressed instructions which compile successfully but the zkVM cannot decode.
`guest/string.h` only declares standard memory routines; it replaces no crypto
operation. The linked guest runtime provides those routines.

```bash
cargo +1.97.1 test --locked --manifest-path proofs/sp1/Cargo.toml -p tactus-o1-proof-journal
TACTUS_CARGO_PROVE=/absolute/path/to/cargo-prove scripts/run-proof-experiment.sh 0
```

The runner retains build logs, binary/input hashes, public values, the actual
proof, resource usage and a fresh-process verification log. It caps CPU worker
concurrency for the reference machine. Its numbered fixture argument is 0..8;
running one fixture is not evidence that all nine were proved. All nine are
covered by the native journal tests independently of which expensive proofs run.
