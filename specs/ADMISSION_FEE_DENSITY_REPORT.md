# Admission contention with matched serialized-byte fees

CKB **0.210.0** completes the five-arm, 120-race matrix under each of two fee
policies: fixed absolute fees and fees proportional to complete serialized
transaction bytes. Matching byte fee density preserves the targeted admission
conflict observed in the earlier [admission experiment](ADMISSION_CONTENTION_REPORT.md).
**Experiment A, G2 and production readiness remain OPEN.**

## Method

The driver uses the same five authenticated arms, two independent signing wallets,
64-byte payloads, adversary-first submission, delays of 0/1/3/6 generated blocks,
fee multipliers of 1/2/10 and two repeats. Each race starts with fresh protocol
state. A1 stores the payload commitment; A2 and A3 store the payload bytes.

In `absolute` mode actor 0 pays 100,000,000 shannons (1 CKB). In `wire-density`
mode actor 0 pays exactly **100,000 shannons per serialized byte**, including
witnesses; actor 1 pays that rate times the selected multiplier. The driver first
builds a signed candidate to measure its length, rebuilds and signs with the
resulting fee, then requires that the final length is unchanged. Fee changes
never mutate an already signed transaction. Every final candidate passes the
real node's cycle estimator before either actor broadcasts.

Before submission, the driver resolves every candidate input using the node's
live-cell RPC. The independent checker subtracts output capacities from those
recorded input capacities, recalculates Molecule wire size, and checks the exact
fee policy. It reconciles all 120 unique schedule cases, independent funding,
shared protocol inputs, canonical actor sets, full node-returned transaction
fields, packed lengths, block identities and outcome classifications. It is an
evidence consistency check, not a substitute for the node's consensus validation.

The rate is matched across arms for the same actor. It is **not CKB cycle-weighted
virtual-size pricing**. Block cycle/byte limits, proposal window and txpool policy
are retained in the node configuration and consensus metadata. The experiment
uses the isolated Dummy devnet miner; it does not model permissionless miners.

## Measured outcomes under each policy

| Arm | Races | Victim committed | Both committed | Live pool rejection | Accepted then lost | Stale input |
|---|---:|---:|---:|---:|---:|---:|
| A1 shared head | 24 | 8 | 0 | 6 | 4 | 6 |
| A2 independent messages | 24 | 24 | 24 | 0 | 0 | 0 |
| A3 one lane | 24 | 8 | 0 | 6 | 4 | 6 |
| A3 four lanes, targeted | 24 | 8 | 0 | 6 | 4 | 6 |
| A3 four lanes, disjoint | 24 | 24 | 24 | 0 | 0 | 0 |

For each conflicting arm, at delays 0 and 1, the 2× and 10× victim fees win
while equal fees are rejected. At delay 3, higher-fee replacements are accepted
but lose canonical inclusion. At delay 6, the shared input has already been
spent at every fee multiplier. No conflicting pair both commits. Every independent
pair commits both transactions. The victim's fee input remains live at submission.

Each policy records 168 canonical admissions, 120 setup commits, two deployment
commits and 36 rejected submissions. These are deterministic schedule controls,
not 240 independent random observations or a measured population success rate.

| Arm | Serialized bytes | Actor 0 fee in density mode (CKB) |
|---|---:|---:|
| A1 shared head | 1,034 | 1.034 |
| A2 independent messages | 999 | 0.999 |
| A3 lane admission (all three arms) | 985 | 0.985 |

The experiment closes the narrow missing byte-fee-matching control. It does not
establish a miner inclusion bound, solve targeted same-lane starvation, force
A2 obligations to execute, or bind sealed A3 processing into a validity proof.
Ordinary wallet competition, alternative txpool/miner policies, sustained
admission attacks, resource-weighted pricing and funded liability remain open.

## Reproduction and evidence

Only the supported CKB 0.210.0 binary is used:

```sh
TACTUS_ADMISSION_FEES=wire-density TACTUS_DEVNET_SUITE=replay-admission \
  CKB_BIN=/path/to/ckb scripts/run-devnet-experiments.sh
# Repeat with TACTUS_ADMISSION_FEES=absolute for the regression control.
python3 -B scripts/test-admission-fees.py
```

The launcher runs `check-admission-fees.py` before declaring completion. CI runs
both policies and checks archived positive cases plus eight falsified-evidence
controls. Local validation includes driver Clippy, formatting and both real-node
runs; remote CI results are not claimed.

[Retained evidence](evidence/admission-fees/) includes compressed raw records,
node logs/configuration, launch/build logs, source/binary manifests, checker
summaries and validation output. `SHA256SUMS` covers the committed archive.
Historical CKB-version evidence remains an archive and is not rerun or maintained.
