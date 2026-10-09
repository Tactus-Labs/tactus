# History checkpoints across a real P2P reorg

On 10 October 2026, the checkpoint suite passed on CKB 0.121.0 and 0.210.0 with
two real peers per run. Both runs replaced four original canonical blocks through
peer fork choice. Neither `truncate` nor `submit_block` was used.

The [retained evidence](evidence/checkpoint-reorg/manifest.json) contains full
transaction records, node/peer logs and configurations, source/binary hashes,
branch headers, live-cell responses and independent evidence checks.

## Experiment

After code deployment and Anchor genesis, the nodes synchronize a common prefix
over P2P and then disconnect. The main node commits an Anchor advance and its
typed immutable checkpoint. The peer commits a different valid advance spending
the same Anchor and fee inputs, with a different timestamp and batch commitment.
It mines a longer branch while the main node remains on its original tip.

Reconnecting the peers causes both canonical tips and transaction pools to
converge on the peer's longer branch. The original checkpoint was demonstrably
live before the reorg and is no longer live afterward. The original Anchor
outpoint is also unavailable. The winning checkpoint is live and contains the
exact winning Anchor state. Canonical batch reconstruction recovers the peer's
batch bytes and state rather than the orphaned history.

Two freshly signed dependency controls then spend the same surviving wallet
input with identical outputs and all other dependencies unchanged. Referencing
the orphaned checkpoint fails with the node's exact `TransactionFailedToResolve`
boundary. Referencing the winning checkpoint commits successfully. Both peers
subsequently converge on that final transaction's chain.

| Measurement | CKB 0.121.0 | CKB 0.210.0 |
|---|---:|---:|
| Original canonical blocks replaced | 4 | 4 |
| Retained committed-event records | 6 | 6 |
| Orphan checkpoint dependency rejections | 1 | 1 |
| Canonical reconstructed batches | 1 | 1 |

Committed-event records include the original transaction when it was canonical.
It later becomes an orphan; six records must not be interpreted as six final
canonical transactions. The alternative transaction was initially submitted only
to the disconnected peer and is later recorded from the main node's canonical view.

`scripts/check-checkpoint-reorg.py` independently reconciles the competing inputs,
checkpoint types/data, branch heights, liveness observations, recovered state and
matched dependency controls. Four corrupted evidence copies fail those checks.
The checker does not re-execute CKB consensus. The script ELF is byte-identical to
the [earlier checkpoint qualification](HISTORY_CHECKPOINT_V1.md); removing one
unused import allowed strict target Clippy without changing the ELF hash.

## Reproduction and limits

```bash
TACTUS_DEVNET_SUITE=replay-checkpoint-reorg CKB_BIN=/absolute/path/to/ckb scripts/run-devnet-experiments.sh
python3 scripts/check-checkpoint-reorg.py /absolute/path/to/evidence.json
```

Set `TACTUS_CKB_VERSION=0.210.0` for that binary. The launcher uses distinct
loopback ports 18714–18717 and refuses occupied ports without touching their
owners. An attempted overlapping compatibility launch exercised that refusal;
the retained successful compatibility experiment ran after the first run exited.

This was a controlled partition on funded permanent-Dummy devnets with empty
ordering batches. It establishes the checkpoint dependency's fork-resolution
boundary, not unplanned public-network fault qualification, EVM proof verification,
SettlementTip recovery, confirmation-depth policy or withdrawal safety. A future
settlement consumer must pin the exact checkpoint and Anchor identities and bind
them to the proof journal. G3 and the production goal remain OPEN.
