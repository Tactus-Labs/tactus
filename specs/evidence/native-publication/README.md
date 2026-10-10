# Native custody publication evidence

Only CKB 0.210.0. Raw run: `artifacts/native-publication-56w3tq2H`.
Base commit: `ce0eb38`; the original manifest records the actual working-tree
sources, compiled programs, node and configuration. Critical current onchain,
build and driver sources match that manifest. `audit-manifest.json` records
the later evidence audit, independent oracle and CI integration sources.

Ten transactions commit; 27 exact script controls reject. Two funded receipts
(100 and 150 CKB) are authenticated by the new publication script. Signed EVM
burns of 40 and 50 CKB token units execute in local replay and match independent
Geth. A fresh process reconstructs the same funded vault history.

No new proof settlement, CKB payout, A3 native-profile integration or production
readiness is claimed. The fixture settlement type is deliberately uninstantiated.
The EVM oracle input/output retains false CKB-authentication flags because Geth
only verifies Ethereum execution; the actual CKB evidence supplies the separate
publication-authority result. See `specs/NATIVE_PUBLICATION_REPORT.md`.

`deployment.json` is the exact genesis subset used by host tests. `execution.json`
and `geth.json` retain independent transition inputs/results; `evidence.json.gz`
contains actual accepted/rejected transactions, cycles and cold recovery output.
`check.json` is the independent evidence reconciliation. All retained files are
covered by SHA256SUMS, including replay.log.
