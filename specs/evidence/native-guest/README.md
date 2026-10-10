# Native custody SP1 guest execution

Retained local run `artifacts/native-guest-qXKMQ1qa`, SP1 6.8.1 CPU backend.
`execution/` contains full-range and prefix-one inputs, actual guest journals,
execution telemetry, guest/host build logs and source/ELF/host SHA-256 bindings.
Both runs replay the exact CKB 0.210.0 native publication archive. The Python audit
rechecks that publication evidence and independently reconstructs both journals
using Geth state roots. See `specs/NATIVE_PROOF_STATEMENT_V2.md` for the wire format,
new guest key, timing, scope and reproduction.

No cryptographic proof was generated, no new settlement was executed, and no
custody payout is authorized. The input configuration retains the uninstantiated
settlement hash of the standalone publication experiment. A future deployment
must bind its actual native settlement program and use a new corresponding proof.

The execution manifest describes runtime sources at the time of the run.
`audit-manifest.json` records the later checker/test/CI documentation revisions.
Checksums detect file corruption; neither manifests nor this audit replace proof
verification or independent CKB consensus validation.

```sh
(cd specs/evidence/native-guest && sha256sum --check SHA256SUMS)
python3 -B scripts/check-native-guest.py specs/evidence/native-guest/execution
python3 -B scripts/test-native-guest.py
```
