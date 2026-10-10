# Native vault cold recovery evidence

CKB 0.210.0 only. Raw run: `artifacts/native-vault-pUgeaRdr`.
Base commit: `ec6b08ff66a56fb0fb935355720148337756ba13`; the manifest fingerprints
the actual working-tree sources and binaries. Four independent child processes
recover 0, 1, 2 and 2 deposits from 21 canonical blocks.

The native vault program is unchanged. These are funded development deposits;
no L2 credit, actual withdrawal, independent consensus verification or production
readiness is claimed. See `specs/NATIVE_VAULT_RECOVERY_REPORT.md`.

`canonical-blocks.json` is intentionally kept as plain JSON so Rust tests consume
the exact actual-node fixture without a decompressor dependency. `recovery.json`
contains the four child reports also retained inside `evidence.json.gz`.
`recovery-check.json` is the independent Python reconciliation. SHA256SUMS covers
all archived evidence and this note.
