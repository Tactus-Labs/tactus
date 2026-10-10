# Real A3 proof fulfillment and rollback

**Measured on CKB 0.210.0, 10 October 2026.** The canonical A3 duties now have a
real SP1 Groth16 proof, an accepted on-chain SettlementTip transition, and actual
P2P proof rollback/reapplication. G2/G3/G6 still require broader production
qualification; G7 custody and exits remain unimplemented.

## Proof and first settlement

The exact [nine-batch A3 input](SEALED_SETTLEMENT_REPORT.md) was proved: eight
empty epoch batches followed by the mandatory all-lane publication. Its four
slots retain three successful Ethereum transactions and one deterministic
`Malformed` rejection. All four duties become proof-covered; the rejected slot
is not converted into a successful transaction or given an Ethereum receipt.

The SP1 6.8.1 guest and release v6.1.0 circuit are unchanged. Native replay agrees
with all 768 public bytes. The local CPU recursive phase took **4,560.600 seconds**
(76 minutes), with maximum RSS **20,975,204 KiB**. After retaining the complete
2,802,818-byte recursive witness, the coordinator deliberately terminated its
own recursive child and performed fresh-process wrapping in **114.078 seconds**
(17,547,256 KiB maximum RSS). This is a verified staged proof, not a continuous
monolithic proving success. A fresh SDK process then verified the real proof,
exact journal and guest key. Fifteen cryptographic negative controls passed.

The retained proof container is 2,462 bytes; the CKB Groth16 wire proof is 356
bytes. Journal SHA-256:
`72382e25dce7c3cba0e47702e9dcbc0d3ddf87c52e6126237bcbb78c5473240f`.
Guest ELF SHA-256:
`63e7879070038b22cab411bd025b0b876511734cb7ef78a6c849c491aa909a3c`.

Actual first settlement transaction:
`0x89fa2703c707a66fe2762c8c328c46e2987a76fdf60088b71f26ac7f34e5db27`.
It consumes the canonical initial Tip, preserves its 554 CKB reserved capacity,
references the typed history checkpoint and advances to nine proved batches.
Measured cost: **3,984,799,764 VM cycles**, **2,403 serialized bytes**, **1 CKB fee**.
Fresh duty recovery reports four settled duties and one proved transition.

The run has **18 commits and eight rejected transactions**. Controls include
omitted/reordered/dropped-invalid duties, malformed proof, modified interval,
modified final state, modified predecessor, and replay after settlement.

## Real proof rollback and reuse

A second isolated two-node experiment synchronizes after publication, partitions,
and proves on the primary branch. The peer consumes only the proof's fee input
and grows a longer branch. Rejoining replaces four original CKB blocks and
orphans the real proof transaction. Neither `truncate` nor `submit_block` is used.

Both fresh observers recover nine published batches, zero proved batches and
four published-but-unsettled duties. Admission, seal, payload, publication and
execution identities remain unchanged. The old signed proof transaction fails
because its fee input was consumed on the winning branch.

A replacement reuses the **same exact proof, journal, checkpoint and Tip output**
with the restored Tip and fresh funding:
`0x9624689bb831d3f6de1bac968a54ba91d9408e931a6a6a5198a1d2df15d20f0d`.
Its cost is **3,984,822,194 cycles**, 2,403 bytes and 1 CKB fee. Both observers then
recover four settled duties and **one canonical proved transition**, excluding
the orphaned proof. This run has 20 historical commits and nine rejections.

## Evidence and verification

- `specs/evidence/sealed-chain-proof`: actual proof, exact input, recursive witness,
  native execution report, circuit/environment metadata, wrapping and fresh SDK logs.
- `specs/evidence/sealed-proof-settlement/0.210.0`: first transition, full raw node
  evidence, configuration, source/binary manifest and independent reconciliation.
- `specs/evidence/sealed-proof-reorg/0.210.0`: actual peer fork, restored canonical
  duties, exact-proof replacement, both cold observers and raw node/peer logs.

All archives have SHA256SUMS. Raw runs are `chain-proof-oRoFWR3b`,
`sealed-settlement-nyQqhzGj` and `sealed-settlement-l13WhNV4` under `artifacts/`.
The Python checkers reconcile receipts and lifecycle data; they are not crypto
verifiers. Native settlement tests independently invoke the actual SP1 verifier
on the archived proof and reject 14 altered journal domains, a wrong guest key
and corrupted proof bytes. Retained-evidence tests reject 15 forged settlement
or rollback views. CI includes those tests and archive integrity.

```sh
python3 -B scripts/test-sealed-proof.py
python3 -B scripts/check-sealed-proof.py \
  specs/evidence/sealed-proof-settlement/0.210.0/evidence.json.gz \
  specs/evidence/sealed-chain-proof/proof
python3 -B scripts/check-sealed-proof-reorg.py \
  specs/evidence/sealed-proof-reorg/0.210.0/evidence.json.gz \
  specs/evidence/sealed-chain-proof/proof
cargo test --locked --manifest-path proofs/ckb-settlement/Cargo.toml
```

The roughly 78-minute proof cost is not a production latency result. Independent
operators/provers, sustained adversarial admission/miner workloads, bounded
incremental recovery, setup provenance, production envelope, asset conservation
and independently constructible withdrawals still block the full objective.
