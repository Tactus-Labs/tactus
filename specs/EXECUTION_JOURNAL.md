# Durable execution journal V1

`tactus-o1-execution::store::Store` stores complete input batches and replays them
from an explicitly supplied genesis when opened. `replay-execution` exposes this
as an offline command. It does not query CKB, assert canonicality or settle assets.

## Publication and restart

The parent directory must exist. A newly created journal directory and its parent
are synced. An exclusive OS file lock is held throughout startup replay and the
store's lifetime, so cooperating writers cannot append concurrently. The manifest
pins genesis allocation, execution rules and genesis header hash even before the
first batch. It must agree with the caller's trusted genesis.

Each batch is executed on a private clone first. An execution/encoding failure
leaves memory and disk unchanged. A successful transition is encoded as:

`TO1LOG01 || input_length_u32le || complete_input || post_header_hash32 ||
post_state_root32 || record_checksum32`

The checksum is CKBHash with domain `tactus/o1/journal-record/v1` over all preceding
record bytes. This is corruption detection, not an authentication signature.
Records are named `00000000000000000000.batch`, then increasing zero-padded batch
numbers. The input length is bounded by the admission limit before replay.

Publication writes the reserved `pending` file, fsyncs it, hard-links it to its
final name without overwriting any existing record, fsyncs the directory, removes
the staging name and fsyncs the directory again. Memory advances only after these
operations succeed. Any error after I/O begins poisons the open handle: callers
must drop and reopen it because disk may already contain the new record. Blind
retry on stale memory is prohibited.

An interrupted `pending` file is ignored at startup. A linked complete record is
replayed exactly once even if `pending` still links to it. All committed records
must form a contiguous sequence, pass length/checksum validation and produce the
saved state/header hashes under actual EVM replay. Wrong genesis/rules, missing
manifest, malformed records, gaps, unexpected files or differing replay outputs
fail closed. No committed record is silently truncated or repaired.

## CLI

```bash
cargo run --locked --bin replay-execution -- genesis.json journal-dir batch-0.bin batch-1.bin
cargo run --locked --bin replay-execution -- genesis.json journal-dir
```

Inputs are binary batch-V1 files and bounded JSON genesis. The first command
prints each durably published block and the recovered head as JSON lines. The
second restarts from the same input journal and prints the same head. An error
returns a nonzero process exit status. `settled: false` is explicit in the result.

## Evidence and limits

Ten journal regressions cover exclusive writer ownership, restart replay of an
independent Geth fixture, incomplete staging, publication-before-unlink layout,
checksum corruption, truncation, sequence gaps, wrong derived roots even with a
recomputed checksum, wrong/missing genesis, execution-domain changes, failed
publication/poisoning, malformed input, unknown filenames, and a separate-process
CLI restart. These are real filesystem tests and deterministic interrupted-write
layouts; they are **not power-loss or faulty-hardware qualification**.

This implementation assumes a local filesystem honoring file/directory fsync,
hard links and advisory file locking (tested on Linux). Replay is linear in the
entire input history and the EVM state is in memory. Checkpoints, pruning, space
management, sustained operation and reorg journals remain future work. A complete
suffix deletion cannot be distinguished from an earlier valid local journal:
the canonical CKB anchor must independently determine the required final input
frontier. Neither this local journal nor its saved roots replace chain recovery,
an authenticated genesis, a validity proof or settlement verification.
