use alloy_primitives::{keccak256, Bytes, B256, U256};
use alloy_trie::{proof::ProofRetainer, HashBuilder, Nibbles, TrieAccount};
use tactus_o1_native_vault_script::*;
use tactus_o1_state_proof_script::{encode_proofs, Claim};
fn cfg() -> Config {
    Config {
        identity: [1; 32],
        ckb_genesis: [2; 32],
        rollup: [3; 32],
        chain: 31337,
        contract: [4; 20],
        settlement: [5; 32],
    }
}
fn proof(key: B256, value: Vec<u8>) -> (B256, Vec<Bytes>) {
    let path = Nibbles::unpack(key);
    let mut b = HashBuilder::default().with_proof_retainer(ProofRetainer::new(vec![path]));
    b.add_leaf(path, &value);
    let root = b.root();
    let proof = b
        .take_proof_nodes()
        .matching_nodes_sorted(&path)
        .into_iter()
        .map(|(_, node)| node)
        .collect();
    (root, proof)
}
// Synthetic trie fixture for pure verification tests. Never a CKB settlement receipt.
fn fixture() -> (Config, State, Release, Vec<u8>) {
    let cfg = cfg();
    let (state, _) = State::genesis(&cfg, 100)
        .deposit(&cfg, [6; 20], 1000)
        .unwrap();
    let slot = keccak256(
        [
            U256::from(1).to_be_bytes::<32>(),
            U256::from(7).to_be_bytes::<32>(),
        ]
        .concat(),
    );
    let claim = Claim {
        root: B256::repeat_byte(1),
        header: B256::repeat_byte(2),
        settled_batches: 1,
        address: cfg.contract,
        exists: true,
        nonce: 1,
        balance: U256::ZERO,
        storage_root: B256::ZERO,
        code_hash: cfg.runtime_hash(),
        slot,
        value: U256::ZERO,
    };
    let mut r = Release {
        claim,
        id: 1,
        amount: 300,
        recipient: [7; 32],
        owner: [6; 20],
        payout: 1,
        siblings: empty_siblings(),
        proof: vec![],
    };
    r.claim.value = U256::from_be_bytes(r.commitment(&cfg).0);
    let (storage, nodes) = proof(keccak256(slot), alloy_rlp::encode(r.claim.value));
    r.claim.storage_root = storage;
    let account = TrieAccount::new(1, U256::ZERO, storage, cfg.runtime_hash());
    let (root, accounts) = proof(keccak256(cfg.contract), alloy_rlp::encode(account));
    r.claim.root = root;
    r.proof = encode_proofs(&accounts, &nodes).unwrap();
    let mut tip = vec![0; 280];
    tip[..8].copy_from_slice(b"TO1TIP01");
    tip[8] = 1;
    tip[16..24].copy_from_slice(b"TO1ANC01");
    tip[56..64].copy_from_slice(&1u64.to_le_bytes());
    tip[216..248].copy_from_slice(root.as_slice());
    tip[248..280].copy_from_slice(r.claim.header.as_slice());
    (cfg, state, r, tip)
}
#[test]
fn deposit_conservation_and_all_authenticated_fields() {
    let c = cfg();
    assert_eq!(Config::decode(&c.encode()).unwrap(), c);
    let s = State::genesis(&c, 400);
    let (n, r) = s.deposit(&c, [8; 20], 1000).unwrap();
    assert_eq!(n.capacity(), Ok(1400));
    assert_eq!(n.released, 0);
    assert_eq!(n.claimed, s.claimed);
    assert_eq!(n.count, 1);
    assert_eq!(r.cumulative, 1000);
    assert_eq!(State::decode(&n.encode()).unwrap(), n);
    assert_eq!(Record::decode(&r.encode()).unwrap(), r);
    for (address, amount) in [
        ([0; 20], 1),
        (c.contract, 1),
        ([8; 20], 0),
        ([8; 20], u64::MAX),
    ] {
        assert!(s.deposit(&c, address, amount).is_err());
    }
    let mut overflow = s.clone();
    overflow.count = u64::MAX;
    assert!(overflow.deposit(&c, [8; 20], 1).is_err());
    let mut release = s.clone();
    release.released = 1;
    assert!(release.capacity().is_err());
    for mode in 0..6 {
        let mut other = c.clone();
        match mode {
            0 => other.identity[0] ^= 1,
            1 => other.ckb_genesis[0] ^= 1,
            2 => other.rollup[0] ^= 1,
            3 => other.chain += 1,
            4 => other.contract[0] ^= 1,
            _ => other.settlement[0] ^= 1,
        };
        assert_ne!(c.empty_deposits(), other.empty_deposits());
        assert_ne!(c.deposit_id(1), other.deposit_id(1));
        if mode != 5 {
            assert_ne!(c.runtime_hash(), other.runtime_hash());
        }
    }
    assert_ne!(
        s.deposit(&c, [8; 20], 1000).unwrap().1.after,
        s.deposit(&c, [9; 20], 1000).unwrap().1.after
    );
}
#[test]
fn sparse_claims_cannot_replay_redirect_or_change_siblings() {
    let empty = empty_claims();
    let siblings = empty_siblings();
    let claimed = claim_once(empty, 1, &siblings).unwrap();
    assert_ne!(claimed, empty);
    assert_eq!(claim_once(claimed, 1, &siblings), Err(7));
    assert_eq!(claim_once(empty, 0, &siblings), Err(7));
    for id in [2, 1 << 63, u64::MAX] {
        assert_ne!(claim_once(empty, id, &siblings).unwrap(), claimed);
    }
    for level in [0, 1, 31, 63] {
        let mut bad = siblings;
        bad[level][0] ^= 1;
        assert_eq!(claim_once(empty, 1, &bad), Err(7));
    }
}
#[test]
fn release_requires_code_domain_burn_proof_and_unclaimed_id() {
    let (c, s, r, tip) = fixture();
    let encoded = r.encode().unwrap();
    let mut r = Release::decode(&encoded).unwrap();
    let next = r.verify(&c, &s, &tip).unwrap();
    assert_eq!(next.released, 300);
    assert_eq!(next.capacity(), Ok(800));
    assert_eq!(next.deposits, s.deposits);
    assert_eq!(r.verify(&c, &next, &tip), Err(7));
    for mode in 0..9 {
        let mut bad = Release::decode(&encoded).unwrap();
        match mode {
            0 => bad.amount += 1,
            1 => bad.recipient[0] ^= 1,
            2 => bad.owner[0] ^= 1,
            3 => bad.id += 1,
            4 => bad.claim.code_hash.0[0] ^= 1,
            5 => bad.claim.address[0] ^= 1,
            6 => bad.claim.slot.0[0] ^= 1,
            7 => bad.claim.value += U256::from(1),
            _ => bad.claim.root.0[0] ^= 1,
        };
        assert!(bad.verify(&c, &s, &tip).is_err());
    }
    r.proof[12] ^= 1;
    assert_eq!(r.verify(&c, &s, &tip), Err(10));
    let mut badtip = tip.clone();
    badtip[8] = 0;
    assert_eq!(
        Release::decode(&encoded).unwrap().verify(&c, &s, &badtip),
        Err(9)
    );
    let mut unsupported = s;
    unsupported.deposited = 299;
    assert_eq!(
        Release::decode(&encoded)
            .unwrap()
            .verify(&c, &unsupported, &tip),
        Err(4)
    );
}
#[test]
fn strict_release_framing_rejects_truncation_suffixes_and_oversize() {
    let (_, _, r, _) = fixture();
    let encoded = r.encode().unwrap();
    assert_eq!(
        Release::decode(&encoded).unwrap().encode().unwrap(),
        encoded
    );
    for end in [0, 7, 8, 279, 2403, encoded.len() - 1] {
        assert!(Release::decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(Release::decode(&trailing).is_err());
    let mut bad = encoded;
    bad[2400..2404].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Release::decode(&bad).is_err());
    assert!(Release::decode(&vec![0; RELEASE_PREFIX + 131073]).is_err());
}

#[test]
fn lifetime_flows_can_exceed_u64_while_live_capacity_stays_bounded() {
    let c = cfg();
    let mut s = State::genesis(&c, 400);
    s.count = 2;
    s.deposited = u128::from(u64::MAX) + 1000;
    s.released = u128::from(u64::MAX);
    assert_eq!(s.capacity(), Ok(1400));
    let (next, record) = s.deposit(&c, [8; 20], 2000).unwrap();
    assert_eq!(next.capacity(), Ok(3400));
    assert_eq!(record.cumulative, u128::from(u64::MAX) + 3000);
    assert_eq!(State::decode(&next.encode()).unwrap(), next);
    assert_eq!(Record::decode(&record.encode()).unwrap(), record);
    s.released = 0;
    assert!(s.capacity().is_err());
}

#[test]
fn multiple_claim_paths_match_an_independent_sparse_tree_model() {
    use std::collections::{BTreeMap, BTreeSet};
    fn branch(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        hash(
            b"tactus/o1/claims/branch/v1",
            &[left.as_slice(), right.as_slice()].concat(),
        )
    }
    fn model(ids: &[u64], target: u64) -> ([u8; 32], [[u8; 32]; 64]) {
        let mut nodes: BTreeMap<u64, _> = ids
            .iter()
            .map(|id| (*id, hash(b"tactus/o1/claims/leaf/v1", &[1])))
            .collect();
        let mut empty = hash(b"tactus/o1/claims/leaf/v1", &[0]);
        let mut siblings = [[0; 32]; 64];
        for (level, sibling) in siblings.iter_mut().enumerate() {
            *sibling = nodes
                .get(&((target >> level) ^ 1))
                .copied()
                .unwrap_or(empty);
            let parents: BTreeSet<_> = nodes.keys().map(|id| id / 2).collect();
            nodes = parents
                .into_iter()
                .map(|id| {
                    (
                        id,
                        branch(
                            nodes.get(&(2 * id)).copied().unwrap_or(empty),
                            nodes.get(&(2 * id + 1)).copied().unwrap_or(empty),
                        ),
                    )
                })
                .collect();
            empty = branch(empty, empty);
        }
        (nodes.get(&0).copied().unwrap_or(empty), siblings)
    }
    let mut ids = Vec::new();
    let mut root = empty_claims();
    for id in [1, 2, 3, 7, 8, 255, 256, 1 << 63, u64::MAX] {
        let (before, siblings) = model(&ids, id);
        assert_eq!(before, root);
        root = claim_once(root, id, &siblings).unwrap();
        ids.push(id);
        assert_eq!(root, model(&ids, id).0);
        assert_eq!(claim_once(root, id, &model(&ids, id).1), Err(7));
    }
}
