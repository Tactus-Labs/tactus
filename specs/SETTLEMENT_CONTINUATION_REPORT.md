# State-changing second settlement interval inputs

Measured on 10 October 2026. On the current supported CKB **0.210.0**, two real
canonical batch publications reconstruct a contiguous prefix-one proof input.
The second batch contains a signed Ethereum legacy transfer with nonce 3 and
value 321, after the first three accepted transfers. Both state and header change.
This prepares the second real proof; **neither publication is itself settlement**.
The first real-domain Groth16 job remains separate and incomplete at this report's
initial publication.

## Independent execution and canonical data

The `settlement-continuation` example signs the fixed laboratory transaction,
replays the existing canonical export, constructs the second batch and writes
candidate block environments. The pinned Geth 1.17.8 transition tool independently
checks both blocks: state, transaction and receipt roots, gas, logs bloom,
acceptance indices and individual receipt outcomes agree. The second sender nonce
is 4 after execution. No production wallet or user key is involved.

With `TACTUS_PREPARE_SECOND_PROOF=1`, the bootstrap driver publishes an actual
second batch and authenticated ending checkpoint, then recovers both batches and
allocation from canonical CKB blocks before exporting `proving-input-next.json`.
The current-node run has five committed records and the existing 20 exact script
rejections. The second publication is
`0xed5bb1c1e2088e66b691a7abb35143b90dbf43a5c70d4fbbf2f8615796c25d74`.

The next proof input has `prefix_batches=1`, two complete batches, an interval
length of one, exact predecessor Anchor/state/header, authenticated successor
Anchor and the second batch's interval digest. Native execution of the exact
shared guest statement matches all 768 bytes. Its state roots are:

- Predecessor: `39d75d240f98e864a088b6003e2f94134baec5a63bd0bdea9c955e14427527e7`.
- Successor: `1afe4d4380885cb37d1f7179daa70cd8b97e381bee3e291581c5c4b9e7484698`.

These roots also match Geth's first and second results. Independent cold recovery
still reports two published batches, zero settled batches and the live
uninitialized Tip. The export explicitly records `predecessor_ready=false` and
the required first proved Tip data; it does not pretend the first interval has
already settled. A proof of interval two can be prepared before interval one is
settled, but it cannot advance the uninitialized on-chain Tip.

## Rejections and retained evidence

Four real host invocations reject missing prefix input, reversed batch order,
restarting the interval from genesis and a changed predecessor state root, before
creating an output directory or starting the expensive prover. The independent
Python checker reconciles the actual second publication, typed checkpoint,
original deployment identities, prefix continuity, interval digest, fee outpoint,
required predecessor and cold recovery. It does not verify cryptography.

[Raw evidence](evidence/settlement-next-input/) includes the current default-version
run, actual node transactions/logs, canonical exports, native checks, signed
continuation, Geth raw files/results and source hashes. An earlier 0.121.0 run was
completed before that version was retired; it is retained only as historical
evidence. Development and CI now target 0.210.0 exclusively.

```bash
cargo run --locked -p tactus-o1-execution --example settlement-continuation -- \
  specs/evidence/chain-proof-recovery/0.210.0/proving-input.json artifacts/new-continuation
python3 scripts/check-execution-geth.py /absolute/path/to/evm artifacts/new-continuation
TACTUS_PREPARE_SECOND_PROOF=1 TACTUS_DEVNET_SUITE=replay-settlement-bootstrap \
  CKB_BIN=/absolute/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
python3 scripts/check-next-proof-input.py /absolute/path/to/evidence.json
```

Actual interval-one and interval-two proof consumption, replay/skip rejection,
canonical reorg rollback, proving recovery, proof-bound obligations and asset
conservation remain required. G3 and the full production objective stay open.
