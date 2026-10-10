# Real canonical-domain execution proof

On 10 October 2026, the first canonical-chain execution interval produced a real
SP1 **Groth16** proof. A fresh `chain-proof verify` process independently replayed
the export, derived the pinned ELF's verification key, verified the proof and
matched all 768 expected public bytes. The current **CKB 0.210.0** export has the
same domain, allocation, batches, interval and journal as the proving input.
This replaces the earlier synthetic-domain proof as a valid candidate for the
actual SettlementTip deployment. **The later [first-settlement experiment](FIRST_SETTLEMENT_REPORT.md) now proves
on-chain acceptance and planned P2P rollback/reapplication. G3 and production
readiness remain OPEN.**

## Proof and verification

- Guest key: `0x00eb2677694b390a31db8883db49c1a828d73618173ce6f3e5790b7678958b14`.
- Journal SHA-256: `c8df35222b39de81a8569905a3dc6385afd7f70ddc955d63c6239cd4227fbc65`.
- Interval: prefix zero, one batch containing the three signed transfer inputs.
- Proof container: 2,462 bytes; CKB verifier wire proof: 356 bytes; public journal: 768 bytes.
- Actual Groth16 verification rejects 14 individual public-field changes and a
  changed guest key. A fresh SDK verifier then accepts the exact original proof
  against independently replayed chain input and the actual ELF.
- SP1 SDK/verifier 6.8.1, released circuit v6.1.0, unchanged guest and execution rules.
  No verifier-key bypass, development circuit or synthetic proof is used.

[Retained proof, raw logs, inputs and manifests](evidence/chain-groth16-proof/)
allow inspection and independent reruns. The launch environment's
`proof_completed=false` is its original launch snapshot; final proof status is
in `proof/result.json`. `wrap-result.json` preserves the unmodified native wrapper
result before canonical verification metadata was added. That added metadata is
supported by the retained fresh verifier's successful output and exact journal.

## Deliberate phase handoff and cost limits

The original local process completed recursive shrink and wrap. During final
Groth16 key loading it retained the earlier Rust prover's allocations, hit heavy
`memory.high` throttling and consumed its full 6 GiB swap allowance. Its complete
2,802,829-byte recursive witness was copied and checked before **intentional
SIGTERM**. The process had not crashed or been declared dead based on silence.
The saved handoff record includes its live process state and cgroup counters;
`oom` and `oom_kill` were zero at that observation.

The initial process ran 56:33.20 and reached 21,136,916 KiB maximum resident memory.
A new native process finished the same retained witness, checked its exact journal
digest, guest key, exit code, proof nonce and release VK root, and generated and
verified the final proof. That final stage took **130.20 seconds** including the
15 negative controls (130.85 seconds as timed externally), with maximum RSS
17,468,292 KiB. The subsequent fresh SDK verification initialized its prover in
40.95 seconds before successfully verifying the canonical statement.

**130 seconds is final wrapping, not full execution proving latency.** This run
is deliberately staged and does not establish uninterrupted monolithic success,
production throughput, bounded history replay cost, or an affordability threshold.
The unchanged guest still replays the full prefix from canonical allocation.

## Runner change

`scripts/run-chain-proof.sh` now defaults to `TACTUS_PROOF_MODE=staged` and builds
both existing native binaries. Its Python coordinator waits for a complete
recursive witness with the expected guest identity, preserves and fsyncs it,
terminates only its own child process, reaps that process, and invokes the native
wrapper in a fresh process. It never restarts the expensive prefix based on time
or observation failure. Partial, mismatched, missing-field and oversized witness
candidates are ignored; a terminal prover failure without a complete witness is
an error. No witness-selection check is treated as cryptographic validation.

The native wrapper still performs all cryptographic checks. The runner then
requires fresh canonical `chain-proof verify` success before writing completion
metadata. The selected proof directory is reported explicitly; staged output is
`resumed-proof/`, and the original partial directory is retained. An explicit
`TACTUS_PROOF_MODE=monolithic` remains available for separate benchmarking.

The selector passes a retained-real-witness positive control and six corrupted
candidate controls. The manually staged first proof above validates the underlying
native wrapping and fresh verification path. The [second canonical interval](TWO_SETTLEMENTS_REPORT.md) subsequently completed
the automated coordinator end to end, fresh verification and actual sequential
CKB settlement. Its measured recursive and final-wrapping costs are reported separately.
