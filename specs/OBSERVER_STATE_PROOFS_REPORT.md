# Independently verifiable account and storage witnesses

10 October 2026. The observer now supports `eth_getProof`, providing Ethereum
Merkle Patricia trie witnesses for accounts and storage. These witnesses let a
caller verify reads against an authenticated state root; they are separate from
the Groth16 proof that establishes correct EVM execution. Custody and exits are
still unimplemented, and G7 remains open.

## Method and trust boundary

The method accepts an address, up to 64 storage keys, and a supported block tag.
As with current state queries, genesis and the latest reconstructed state are
available; other historical state and undefined safe/finalized/pending tags
return explicit errors. Storage keys accept prefixed hex up to 32 bytes,
including padded keys. Response keys use canonical hex quantities.

The executor exposes an immutable iterator over live account snapshots, using
its existing account visibility rules. The observer builds account/storage
tries from actual recovered execution state, checks the resulting root against
the selected header, and verifies its own generated inclusion/exclusion proofs
before returning them. No account mutation or new execution rule is introduced.
The iterator is compiled only for the host. A fresh guest rebuild is byte-for-byte
identical to the original ELF (`63e7879070038b22cab411bd025b0b876511734cb7ef78a6c849c491aa909a3c`);
the root lock file and execution rules are unchanged. The archive retains the
build log and comparison hashes.

The result follows [EIP-1186](https://eips.ethereum.org/EIPS/eip-1186): account
fields with `accountProof`, and values with `storageProof`. For nonexistent
accounts, zero code/storage hashes and empty storage proofs follow the current
[Geth implementation](https://github.com/ethereum/go-ethereum/blob/v1.17.8/internal/ethapi/api.go);
the account exclusion proof establishes absence. Existing empty storage uses
the Ethereum empty-trie root. Zero storage slots are proved absent, not encoded
as zero-valued trie leaves.

`services/proof-checker` is a separate Go CLI pinned to **Go Ethereum 1.17.8**.
It uses Geth's `trie.VerifyProof` and RLP decoding/encoding, independently of the
Rust trie builder. Given an array of `{stateRoot,result}` entries, it checks
account fields, every requested storage value, and exclusion proofs. It bounds
document, node and storage-key sizes. It does not authenticate the supplied
root or verify CKB consensus; callers must obtain that root from an independently
trusted canonical settlement boundary. A root supplied by the same untrusted
RPC response is insufficient by itself.

## Measured results

1. All nine frozen independent Geth execution cases are replayed through the
   actual snapshot/proof implementation. Geth independently verifies **57 account
   proofs and 285 storage keys**, including 18 absent accounts and five nonzero
   storage values. Genesis and final execution states are covered. Final roots
   also match the frozen Geth execution roots.
2. The live HTTP experiment copies the stopped node from the actual accepted
   A3 settlement (`sealed-settlement-nyQqhzGj`). It uses only its own CKB 0.210.0
   process and loopback ports. **49 HTTP requests pass**, including outage/restart,
   log queries, six proof requests and three invalid proof requests.
3. Geth independently verifies those **six live account proofs and 18 storage
   keys**. The archived journal from the accepted real A3 proof supplies the
   expected genesis/final state roots. The HTTP status reports nine published
   and nine proved batches, and the exact accepted SettlementTip outpoint.
4. Thirteen Go forgery controls reject altered root/address/balance/nonce/code/
   storage roots, corrupt or truncated nodes, excessive keys, changed storage
   key/value, missing storage nodes and invented storage for an absent account.
   Five retained-HTTP controls reject forged status/root/request evidence.

The service's ten tests, independent Go tests, root execution regression tests
and service Clippy pass. CI regenerates witnesses from the current Rust source
and verifies them with Geth, in addition to checking retained artifacts. This
prevents an archive-only check from masking a broken current generator.

Raw live run: `artifacts/observer-rpc-m45i9jse`. The retained archive is
`specs/evidence/observer-state-proofs`, including HTTP replies, live and fixture
witnesses, Geth verification summaries, source/binary manifests and node logs.
All retained files are covered by SHA256SUMS.

```sh
TACTUS_OBSERVER_PROOF_EXPORT="$PWD/artifacts/observer-state-proofs.json" \
CARGO_TARGET_DIR="$PWD/artifacts/observer-rpc-target" \
  cargo test --locked --manifest-path services/observer-rpc/Cargo.toml
(cd services/proof-checker && go test ./...)
(cd services/proof-checker && go run . ../../artifacts/observer-state-proofs.json)
python3 -B scripts/test-observer-state-proofs.py
```

Proof generation currently rebuilds tries from the in-memory full replay;
persistent state checkpoints and trie indexing are still required for production
scale. Nonempty contract-storage witnesses have executed fixture evidence;
live A3 storage in this run is empty. Historical proof retrieval, archival access,
[CKB verification of settled-state reads](SETTLED_STATE_PROOF_REPORT.md) now passes
a separate real-Tip experiment. Replay-resistant vault releases and real
deposit/withdrawal conservation remain outstanding. No fund-release authority
is created by this read API.
