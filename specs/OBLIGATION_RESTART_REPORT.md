# Abrupt CKB restart after A3 rollback and republication

On 10 October 2026, an isolated **CKB 0.210.0** primary node was forcibly terminated
with **SIGKILL** after the complete A3 seal/publication reorg and republication.
Its launcher observed child exit **137**, kept the peer running, then restarted
the same CKB binary against the same database. A fresh observer recovered exactly
the same four published duties and zero settled duties as the uninterrupted peer.
This is an actual process-crash experiment at a completed-publication boundary;
it is not a power-loss, mid-transaction, long-duration or proof-settlement test.

The preceding run repeats the measured [eight-block A3 rollback](OBLIGATION_REORG_REPORT.md).
Its final canonical block is height **81**, hash
`0x194e47960bc8cbfab3a0c4dcec7faa10c3db14b8efc4e43398e4991fd0034272`.
While the primary is down, the independent peer still reconstructs all four
original admission identities, replacement seal/publication locations and native
execution outcomes, including the malformed input. The restarted node's startup
log shows that same height/hash loaded from its database before the post-restart
observer runs. Main and peer reports then match the pre-crash report exactly.
The SettlementTip remains uninitialized and cannot acquire proof progress from
the restart.

Only the launcher-owned primary child is killed. The launcher retains the actual
PID, waits for SIGKILL termination, verifies the uninterrupted peer before restart,
starts the new child from its original isolated directory, waits for the expected
prefix, and finally cleans up its own nodes. Existing nodes are not addressed.
The optional crash flag requires the A3 reorg mode without a proof directory.

[Retained evidence](evidence/obligation-restart/) includes original and restarted
node logs, launcher SIGKILL observation, the peer report taken during downtime,
both post-restart reports, signed reorg transactions and all configurations/source
hashes. The independent checker first reconciles the complete reorg experiment,
then binds exit status, changed child PID, loaded database prefix and all four
observer snapshots. Eight corrupted controls reject graceful exit, the same PID,
changed peer state, a missing duty, a different database, overclaimed power-loss
coverage, missing kill evidence and missing restart evidence. CI rechecks those
artifacts; remote CI execution is not claimed.

```sh
TACTUS_OBLIGATION_CRASH=1 TACTUS_OBLIGATION_REORG=1 \
TACTUS_DEVNET_SUITE=replay-sealed-settlement \
TACTUS_DEVNET_RPC_PORT=18734 TACTUS_DEVNET_P2P_PORT=18735 \
TACTUS_PEER_RPC_PORT=18736 TACTUS_PEER_P2P_PORT=18737 \
CKB_BIN=/path/to/ckb-0.210.0 scripts/run-devnet-experiments.sh
python3 -B scripts/check-obligation-restart.py specs/evidence/obligation-restart/0.210.0
python3 -B scripts/test-obligation-restart.py
```

Storage failure, repeated/randomized interruption during writes, archival loss,
independent operators and proof-bound recovery still require qualification.
This bounded result does not close G6/G9 or establish production readiness.
