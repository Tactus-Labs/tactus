# Real Groth16 execution proof and CKB receipts

Measured locally on 10 October 2026. A genuine SP1 Groth16 proof of the signed
three-transfer execution fixture verifies in a fresh host process and in actual
CKB transactions on both 0.121.0 and 0.210.0. Each node committed two immutable
proof receipts and rejected all 28 negative controls. **G3 and production remain
OPEN**: this proof uses the fixture's synthetic deployment identities and does
not advance SettlementTip or authorize withdrawals.

## Proof provenance and recovery

The original local CPU run executed 15,851,885 guest instructions, matched the
native executor and independent Geth roots, and completed recursive shrink/wrap.
The kernel killed that process for memory exhaustion during final Go proving at
06:02:51 JST. Its retained wrapped witness was copied before cleanup, bound to the
exact 768-byte journal, guest key, zero exit code, zero proof nonce and release VK
root, and passed to `proofs/sp1/host/src/bin/wrap-witness.rs` in a fresh process.
The final proof is cryptographically verified before publishing its result.

This is a recovered real recursive proof, not a substituted witness or mock proof.
The [archive](evidence/execution-groth16-proof/) preserves the witness, failed-run
log, kernel OOM evidence, original source manifest, recovery-time manifest, final
proof, successful logs and release key. `recovery.json` records the historical
pending status at recovery time; `proof/result.json` records the later success.
The original run did not complete and has no successful final result.

SP1 SDK/verifier 6.8.1 uses release circuit v6.1.0. Its 492-byte wrapper VK is
byte-identical to the pinned verifier crate. The release archive SHA-256 is
`18beebb6cd0cc9b4d4a240ee4f49511da6c2a7e51724bad4232de538a9147810`.
The multi-gigabyte proving key is not committed; its hash and download provenance
are retained. This establishes artifact identity, not an independent ceremony or
compiler audit. The existing [guest provenance](EXECUTION_CORE_PROOF_REPORT.md)
remains applicable: ELF SHA-256
`63e7879070038b22cab411bd025b0b876511734cb7ef78a6c849c491aa909a3c`;
guest key
`0x00eb2677694b390a31db8883db49c1a828d73618173ce6f3e5790b7678958b14`.

## Measured costs

| Measurement | Result |
|---|---:|
| Final wrapping only, wall time | 116.76 s |
| Final wrapping maximum RSS (`time -v`) | 17,536,920 KiB |
| Final Groth16 constraints | 15,972,262 |
| SDK proof container | 2,463 bytes |
| CKB wire proof | 356 bytes |
| Public journal | 768 bytes |
| Full receipt transaction, node-packed | 1,759 bytes |
| First / second receipt VM cycles, both node versions | 3,976,633,357 / 3,976,637,370 |
| Devnet consensus maximum block cycles | 10,000,000,000 |
| Fixed laboratory transaction fee | 1 CKB |
| Permanently occupied receipt capacity | 894 CKB |

The 116.76 seconds excludes all previous execution and recursive proving. There
is **no successful uninterrupted end-to-end Groth16 latency measurement**. The
original attempt failed after approximately 38 minutes 43 seconds; adding its
runtime to the resumed stage does not establish a production latency bound.
The fresh process used GOMEMLIMIT=16GiB, GOGC=25, GOMAXPROCS=2 and a systemd scope
with MemoryHigh=18G, MemoryMax=21G and MemorySwapMax=6G. The block cycle ceiling was
not increased for the verification experiment. These fees are fixed dummy-chain
inputs, not market fee estimates. Roughly 40% of this chain's cycle budget per
verification is a material unresolved throughput cost.

## Verification and adversarial controls

The resumer rejects all 14 public-field mutations and a changed guest key. An
independent invocation of the original SDK host verifies the exact proof and
expected fixture journal. Another fresh invocation with a distinct valid guest
ELF fails specifically on the guest verification key mismatch.

Both real CKB nodes accept the same two receipt transactions:

- `0xcf70daf6aa40d49fd5dcfa6ecb22c0e0b69c0f2b0492cbde375979baecc749bd`
- `0x76a38b09ab9f18b17245308235dedf8a4e72c915b4accfd282e6c9ed31a9e51f`

Each run has three committed records including code deployment, and 28 exact
script rejections: 14 journal mutations, two proof-byte mutations, wrong/short
key, short/long journal, empty/trailing proof, oversized witness, duplicate
receipt outputs, and consume/replace attempts against each committed receipt.
The receipt type allows creating another receipt for the same proof; this is not
a replay-resistant settlement cursor. Script SHA-256 is
`ea7def2e1d98a8f8098eec8755e85d36e1a90b9c51738b77ccdc47d455ae8e3a`.
The [CKB initialization adapter](../proofs/vendor/lazy_static-1.5.1/CKB_ADAPTER.md)
removes unsupported atomics under explicit single-thread VM assumptions; no
cryptographic algorithm or verifier key is replaced.

`scripts/check-proof-receipts.py` independently reconciles retained transaction
journals, proof witnesses, mutation locations, receipt outpoints, exact script
errors, cycles and Molecule wire sizes. Five evidence mutations also reject:
missing rejection, false cycle count, false wire size, changed proof witness and
wrong script error. This checker checks consistency, not cryptography; the actual
host and CKB verifier executions supply the cryptographic evidence. Archive
hash checks and reconciliation are configured in CI; remote CI is not claimed.

## Reproduction and next boundary

See [receipt format and runner](PROOF_RECEIPT_V1.md). To replay the final wrapping
stage, decompress `wrapped-witness.json.gz`, build `wrap-witness` with
`--features native-gnark`, and pass the released circuit directory, witness,
`proof/public-values.bin`, the guest key above and a new output directory. Use the
recorded memory limits. Fresh SDK verification still requires the exact guest ELF
and fixture; receipt reproduction requires an actual isolated CKB node.

The next positive proof must use the canonical export from
[SettlementTip bootstrap](SETTLEMENT_TIP_V1.md), including the real network,
Anchor and SettlementTip type hashes. This fixture proof cannot be relabeled for
that deployment. Valid-proof succession, replay rejection, multiple intervals,
reorg/recovery, proof-bound obligations and custody/exit conservation remain to
be qualified before production.
