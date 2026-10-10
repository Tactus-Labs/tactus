# Actual native checkpoint qualification

CKB 0.210.0 run `artifacts/native-checkpoint-ApK8qvfW`: 11 committed transactions,
43 precisely attributed script rejections, two immutable full-state checkpoints.
See `specs/NATIVE_CHECKPOINT_REPORT.md` for program identities, measured costs,
reproduction and remaining production gaps.

The archive preserves original compressed evidence/summary/node logs, launcher
manifest, isolated chain configuration, replay log and independent Geth results.
`check.json` joins custody/publication/checkpoint data and rejects inflated claims.
The runtime manifest records sources before later audit/docs/CI additions;
`audit-manifest.json` records those later files. Checksums detect corruption,
not chain finality or cryptographic proof validity.

```sh
(cd specs/evidence/native-checkpoint && sha256sum --check SHA256SUMS)
python3 -B scripts/check-native-publication.py specs/evidence/native-checkpoint/0.210.0
python3 -B scripts/test-native-checkpoint.py
```

No native proof settlement or withdrawal occurred. The 574 CKB per checkpoint
remains locked permanently; economical production archival storage is unresolved.
