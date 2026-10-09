# First locally verified execution proof

The pinned SP1 guest now has a **real local CPU core STARK** for the first retained
Geth fixture: one block containing three signed Ethereum envelope types. The proof
commits the complete [execution journal](EXECUTION_PROOF_V1.md), and its public
values agree with the native executor and the independent Geth state, transaction
and receipt roots. This is execution-proof evidence; **CKB settlement and production
readiness remain unachieved**.

## Result and controls

The [retained proof archive](evidence/execution-core-proof) contains the actual
proof bundle, public bytes, prover log, resource usage, source/binary hashes and
fresh-process verification results. The positive proof is verified again by a
separate process with the ELF-derived key and exact expected journal. A second
fresh process uses a distinct valid guest ELF and must reject the same proof.

The generation process also verifies rejection after separately changing the
profile, CKB network, ordering script, settlement script, rollup ID, chain ID,
allocation commitment, predecessor cursor, ending history commitment, either state
root, either header hash and exact interval digest. The fifteenth control uses a
wrong guest key. These controls alter an existing proof's public bytes or verifier
key; they do not demonstrate on-chain authentication of a real deployment.

The host explicitly instantiates the CPU prover. Network and experimental SDK
features are disabled; no mock, skipped key verification, development setup or
remote prover produces this proof. The laboratory deployment hashes are repeated
`01`, `02` and `03` bytes, not claimed CKB identities.

| Measurement | Observed value |
|---|---:|
| Input blocks / signed transactions | 1 / 3 |
| Guest RISC-V instructions | 15,851,885 |
| Native/Geth/guest public-output agreement | PASS |
| Core proof bytes | 38,675,417 |
| Whole generation process elapsed | 770.38 seconds |
| Setup + guest execution | 39.454 seconds |
| Process CPU time (user + system) | 2,615.24 seconds |
| Maximum resident set | 14,258,192 KiB (about 13.6 GiB) |
| Public-value/key negative controls | 15 rejected |

Whole-process time includes initialization, execution, proving, verification and
negative controls; it is not isolated cryptographic proving time. Four Rayon/Tokio
threads and one core proving worker were configured, with the retained shard
thresholds. Other compilation and artifact preparation occurred on this workstation
while it ran. These measurements are a reproducible initial cost observation, not
sustained proving throughput or a production hardware qualification.

The positive guest SHA-256 is
`63e7879070038b22cab411bd025b0b876511734cb7ef78a6c849c491aa909a3c`.
Its SP1 verifying-key identity is
`0x00eb2677694b390a31db8883db49c1a828d73618173ce6f3e5790b7678958b14`.
The prior [guest execution archive](evidence/proof-execution) retains the ELF,
compiler/source identity, Geth fixture identity and exact launch-time lock.

## Reproduction and remaining work

`scripts/run-proof-experiment.sh 0` builds the pinned guest and a wrong-key control,
generates a real local proof and verifies it in a new process. Set
`TACTUS_CARGO_PROVE` to the official SP1 6.8.1 executable. The manual command used for
this first run is retained in the execution archive; the integrated runner was
added while that run was active and is not represented as the original command.

To verify the retained bundle without generating another proof, decompress
`proof.bin.gz` into a local output directory as `proof.bin`, then run:

```bash
artifacts/prover-host/target/release/tactus-o1-sp1-host verify \
  proofs/sp1/target/elf-compilation/riscv64im-succinct-zkvm-elf/release/tactus-o1-sp1-guest \
  specs/test-vectors/execution-v1/geth-1.17.8.json 0 /absolute/path/to/output-directory
```

Only fixture 0 is proved here. The nine-fixture native statement suite and prior
CKB publication tests do not imply all nine fixtures were proved or any proof was
accepted by CKB. Still required are compression to the selected final proof form,
a CKB-VM verifier with measured cost and negative controls, authenticated canonical
history and SettlementTip succession, reviewed setup/key provenance, prefix/state
witness scalability, broader execution conformance and proving economics. No
withdrawal or custody authority is introduced by this experiment.
