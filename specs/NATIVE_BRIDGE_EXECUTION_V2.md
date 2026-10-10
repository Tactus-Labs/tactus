# Candidate native custody execution v2

Status: **execution implemented and independently compared; new proof
authentication remains pending**. A [subsequent CKB experiment](NATIVE_PUBLICATION_REPORT.md)
now implements standalone publication authentication. This is a new optional
`native-bridge` feature, not a change to the v1 executor or authorization to mint
against an existing vault. G1–G9 remain OPEN. Only CKB 0.210.0 is in scope.

The executor now consumes a contiguous native vault deposit transcript, invokes
the pinned NativeCKB runtime as the zero system caller, and then executes signed
Ethereum transactions. Genesis prohibits any zero-address account, requires the
exact domain-patched bridge runtime with nonce one, zero native balance and empty
storage, and excludes Shanghai precompile addresses as the bridge address.
At every batch boundary the contract code/nonce remain fixed, its cumulative
deposits equal the transcript cursor, and supply plus cumulative burns equals
cumulative deposits. Outstanding token supply remains at most u64::MAX; gross
flow uses u128 and can exceed u64 after circulation.

**Transcript validity is not deposit authenticity.** A caller can construct an
arithmetically consistent transcript for nonexistent deposits. The synthetic
test case deliberately does so. A CKB publication verifier must resolve each
record against its live immutable receipt for the trusted vault and reject
forgeries before canonical admission. The updated proof/settlement statement
must bind that same domain, ordered prefix and cursor. None of those missing
checks can be replaced by accepting this executable's JSON output. The publication
script now supplies the receipt check; proof and A3 integration are still required.

## Wire and execution rules

The normative candidate descriptor is
`crates/tactus-o1-execution/rules-native-v2.txt`. Its rules commitment hashes that
descriptor, the unchanged v1 descriptor, root Cargo.lock and exact compiled
NativeCKB artifact with domain `tactus/o1/native-execution-rules/v2`.

Config and deposit records use the actual vault's `TO1VAU01` (164 bytes) and
`TO1DPR01` (124 bytes). The 56-byte cursor is count u64 LE, cumulative shannons
u128 LE and append accumulator32. Its genesis accumulator, record append hash
and unique deposit ID match the deployed native vault. The candidate ordering
state is the existing 200-byte base anchor plus that cursor, totaling 256 bytes.

The wrapper is `TO1BRG02 || parentCursor56 || depositCount_u16le || records ||
userEnvelopeLength_u32le || TO1BAT01_userEnvelope`. Maximum total size is 262,144
bytes with at most 32 records, plus all existing user batch/block limits. The
records must exactly extend the prior cursor with nonzero amounts, valid
recipients and checked count/cumulative arithmetic. Duplicate, reordered, missing
or cross-domain records are rejected. The next base anchor's batch commitment is
the CKB-personalized hash of the **complete wrapper**, domain
`tactus/o1/native-batch/v2`, so subsequent publications bind deposits and users.

Credits execute serially before the first user block. They use zero native value,
charge no user fee and consume no user block gas. The pinned REVM system-call
handler has a 30-million gas ceiling; each successful pinned-contract call must
also fit the 200,000-gas credit bound. A failing credit aborts the complete batch,
including earlier successful credits. Invalid framing/transcript, fatal engine
errors or invariant failures also leave the original state untouched.

Credits return separate outcomes and logs. They are not signed Ethereum
transactions and are excluded from user transaction/receipt roots and user log
bloom. The first block's state root includes credits; each block's extra data
binds the complete wrapper via the existing user outcome commitment, the resulting
cursor and full vault script. Native-credit log RPC/indexer support is still
required; the existing observer only implements v1. Ordinary callers cannot mint.
Native ETH-like gas balances remain separate from wrapped CKB balances. Zero may
receive native transfers/fees, but may never acquire code, nonce or storage, and
system calls preserve any accumulated native balance.

## Evidence and regression scope

Six Rust tests cover actual receipt bytes/IDs, old/new domain isolation,
genesis authority constraints, signed transfer/burn/mint rejection, whole-batch
rollback on a later credit overflow, every truncated wrapper prefix and selected
field corruptions, duplicates/gaps/domain mismatch, maximum 32-credit batches,
multiple user blocks and gross circulation beyond u64.

Geth 1.17.8 independently reproduces **seven blocks, five credits, five signed
transactions and one expected revert**. It checks genesis/state roots, user
transaction/receipt roots and complete encoded receipts, gas, bloom, system-call
logs, header hashes and parent linkage. This oracle verifies Ethereum transitions;
it does not independently verify CKB consensus or the bridge publication rules.
The Python audit independently recomputes the vault script hash, record IDs,
ordered accumulator, complete-wrapper commitment, cursor and user framing, and
reconciles Geth outputs. Nine forged-evidence controls fail.

The first vector case reuses the **exact two real funded records** from the cold
recovery archive and credits 250 CKB of ledger units in a local candidate replay.
It is not a real settled L2 credit. The second case has **three synthetic records**,
a transfer, three permanent burns (including deposit and burn in one batch), and
an unauthorized mint attempt. It ends with supply zero and 1,750 cumulative
credited/burned units. The zero system caller receives actual user fees in this
fixture; Geth confirms later credits preserve its native balance.

The v1 SP1 guest was rebuilt after adding the opt-in modules. Its ELF SHA-256
remains `63e7879070038b22cab411bd025b0b876511734cb7ef78a6c849c491aa909a3c`;
root Cargo.lock and v1 rules bytes remain unchanged. Existing proof keys and A3
receipts therefore keep their original scope. No new zkVM guest/key, Groth16
proof, CKB ordering script or settlement verifier is claimed by this milestone.

Retained fixtures, independent output, source/tool fingerprints and build log:
`specs/evidence/native-bridge-execution`. Reproduce with:

```sh
cargo test --locked -p tactus-o1-execution --features native-bridge --test native_bridge
cargo run --locked -p tactus-o1-execution --features native-bridge \
  --example native-bridge-vectors -- /tmp/native-bridge-candidate.json
(cd services/bridge-checker && \
  TACTUS_NATIVE_BRIDGE_VECTOR=/tmp/native-bridge-candidate.json \
  go test -mod=readonly -count=1 -run TestCandidateNativeExecutionAgainstGeth .)
python3 -B scripts/test-native-bridge-execution.py
```

## Next integration constraints

Standalone publication now authenticates every included receipt at the script
boundary. That check must be preserved in A3 integration, otherwise a forged
irreversible publication can prevent valid proof settlement. The 256-byte native
anchor, carried inside 428-byte publication metadata, still needs explicit
A3/sealed/checkpoint/recovery support.
The [native proof journal](NATIVE_PROOF_STATEMENT_V2.md) now binds full vault
identity, before/after cursors, allocation, ordering interval and resulting
Ethereum roots; real proof generation and settlement remain pending. The settlement Tip can retain
the existing root/header layout only if its new verifier binds all those fields.

Avoid a deployment hash cycle: the vault's EVM DOMAIN omits settlement identity,
but full vault config includes it. Do not put a full vault type hash in anchor
type arguments while settlement arguments also contain that anchor type hash.
Genesis-authenticated immutable metadata/state can bind the final vault config
after deriving the ordering identity and allocation commitment. The standalone
publication experiment now validates this construction on a real node; binding
the final, actually deployed settlement verifier remains required.

Finally generate the new real proof, settle it, independently derive nonzero burn
witnesses, and execute CKB payouts with duplicate/redirected/forged controls and
operator-independent reorg recovery. The current execution work closes none of
those remaining production gates by itself.
