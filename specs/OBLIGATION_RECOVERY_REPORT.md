# Canonical A3 obligation lifecycle recovery

On 10 October 2026, the new `recover-obligations` observer reconstructed all four
A3 duties in three fresh processes on **CKB 0.210.0**: four admitted duties before
sealing, four sealed duties before publication, and four published duties before
proof. Each report recovered the atomic SettlementTip as uninitialized, with
**zero proved transitions and zero settled duties**. The exact canonical proving
input remains byte-identical to the [A3 preparation](SEALED_SETTLEMENT_REPORT.md).

## Reconstruction and trust boundary

The A3 scanner now retains each `(lane, sequence)` admission with its original
payload, admission outpoint, optional immutable seal outpoint and optional
publication location. It derives these from authenticated lane successors,
complete all-lane seals and mandatory-prefix publications. A publication records
the canonical Anchor transaction, zero-based batch, Ethereum block and input slot.
An admission cannot be sealed or published twice, and a duty missing from the
admission history cannot appear as published. Fresh reconstruction of an earlier
canonical prefix removes stages no longer present; no local saved duty cursor is
accepted as input.

The combined observer first recovers settlement through the existing complete
canonical scan and native execution replay. It then scans A3 and published inputs
at **that same pinned CKB height/hash**, allowing ordinary later chain growth.
It verifies both scanners agree on the exact Anchor state and outpoint, replays
published execution, and matches every duty's original payload to its publication
transaction and executed input slot. Only a publication batch strictly below the
recovered proved batch frontier is labeled `settled`. Rechecking the pinned block
at the end rejects a replaced prefix instead of returning a mixed-branch result.

All four lifecycle states remain explicit: `admitted`, `sealed`, `published`,
`settled`. A deterministic invalid input remains a duty and retains its rejection
outcome; successful Ethereum execution is not required to count processing once
covered by a real settled prefix. The measured malformed slot stays `published`
with `Malformed`, no transaction index, and `proof_settled=false`.

The configured full node remains the consensus/script-validity trust boundary.
This observer does not independently verify Groth16 or authorize asset release.
Reports explicitly retain those limitations, the deployment identities and shared
pinned prefix. It scans full history and holds duty history in memory; it is not
an archival availability or bounded long-duration performance qualification.

## Actual validation and remaining work

The run still has **17 committed transactions and four rejected controls**.
Adding read-only recovery does not change deployment identities, canonical batch
bytes, proof journal or executable protocol scripts. The independent Python
checker joins the three observer reports to retained admission, seal and
publication receipts and verifies all outcomes, locations and pending settlement
boundaries. Nine forged reports fail, including mixed pinned blocks, a false
proof claim, a missing duty, wrong slot/admission/seal/outcome, a fabricated proved
frontier and a custody claim.

Rust regression tests rebuild the measured existing A3 transaction fixture at
five different prefixes. They check duty phases before/after sealing and
publication, retain eight later active duties alongside three published duties,
and preserve exact admission/seal/publication identities. Existing malformed
lane, incomplete seal, omitted mandatory-prefix, changed lock/capacity and
recovery reorg tests still pass. The complete root workspace test suite passes. These prefix tests are not an actual new P2P
rollback experiment; the subsequent [actual P2P obligation experiment](OBLIGATION_REORG_REPORT.md) now
qualifies rollback of the pending seal and publication. Proof-bound rollback remains open.

The real A3 proof path invokes the combined cold observer again after Tip
consumption and requires four settled duties, including the malformed one.
**That positive proof-bound path remains pending at this report's date.**
The dedicated proof is queued behind the running second canonical interval.
G2/G3 and production readiness remain OPEN.

[Retained evidence and checksums](evidence/obligation-recovery/) include all three
fresh reports, signed transaction receipts, launcher/source manifests, node
configuration and raw logs. CI reconciles these reports and the nine forged
controls. Remote CI execution is not claimed.

```sh
cargo build --locked --bin recover-obligations
TACTUS_CKB_RPC_ADDR=127.0.0.1:18724 target/debug/recover-obligations \
  CKB_GENESIS_HASH GATE_TYPE_SCRIPT_HEX ANCHOR_TYPE_SCRIPT_HEX SETTLEMENT_TYPE_SCRIPT_HEX
python3 -B scripts/check-obligation-recovery.py \
  specs/evidence/obligation-recovery/0.210.0/evidence.json.gz
python3 -B scripts/test-obligation-recovery.py
```
