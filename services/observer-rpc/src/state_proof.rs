//! Account and storage witnesses for independently verifiable Ethereum state.
use crate::RpcError;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_trie::{
    proof::{verify_proof, ProofRetainer},
    root::storage_root_unhashed,
    HashBuilder, Nibbles, TrieAccount, EMPTY_ROOT_HASH,
};
use serde_json::{json, Value};
use tactus_o1_execution::{Executor, GenesisAccount};

fn trie_account(account: &GenesisAccount) -> TrieAccount {
    TrieAccount::new(
        account.nonce,
        account.balance,
        storage_root_unhashed(
            account
                .storage
                .iter()
                .map(|(k, v)| (B256::from(k.to_be_bytes::<32>()), *v)),
        ),
        keccak256(&account.code),
    )
}
fn prove(mut leaves: Vec<(B256, Vec<u8>)>, target: B256) -> Result<(B256, Vec<Bytes>), RpcError> {
    leaves.sort_unstable_by_key(|(key, _)| *key);
    let path = Nibbles::unpack(target);
    let mut builder = HashBuilder::default().with_proof_retainer(ProofRetainer::new(vec![path]));
    let expected = leaves
        .iter()
        .find(|(key, _)| *key == target)
        .map(|(_, v)| v.clone());
    for (key, value) in leaves {
        builder.add_leaf(Nibbles::unpack(key), &value);
    }
    let root = builder.root();
    let proof: Vec<_> = if root == EMPTY_ROOT_HASH {
        vec![]
    } else {
        builder
            .take_proof_nodes()
            .matching_nodes_sorted(&path)
            .into_iter()
            .map(|(_, node)| node)
            .collect()
    };
    verify_proof(root, path, expected, proof.iter())
        .map_err(|_| RpcError(-32603, "constructed trie proof failed verification"))?;
    Ok((root, proof))
}
pub fn storage_key(value: &Value) -> Result<U256, RpcError> {
    let text = value
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .ok_or(RpcError(-32602, "invalid storage key"))?;
    if text.is_empty() || text.len() > 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(RpcError(-32602, "invalid storage key"));
    }
    U256::from_str_radix(text, 16).map_err(|_| RpcError(-32602, "invalid storage key"))
}
pub fn account_proof(
    engine: &Executor,
    address: Address,
    requested: &[U256],
) -> Result<Value, RpcError> {
    if requested.len() > 64 {
        return Err(RpcError(-32005, "at most 64 storage keys per proof"));
    }
    let leaves = engine
        .accounts()
        .map(|(address, account)| {
            (
                keccak256(address),
                alloy_rlp::encode(trie_account(&account)),
            )
        })
        .collect();
    let (root, proof) = prove(leaves, keccak256(address))?;
    if root != engine.head().state_root {
        return Err(RpcError(
            -32603,
            "proof state differs from canonical header",
        ));
    }
    let account = engine.account(address);
    let value = account.as_ref().map(trie_account);
    let mut storage = Vec::new();
    for key in requested {
        let (root, proof) = if let Some(account) = account.as_ref() {
            prove(
                account
                    .storage
                    .iter()
                    .map(|(key, value)| {
                        (
                            keccak256(key.to_be_bytes::<32>()),
                            alloy_rlp::encode(*value),
                        )
                    })
                    .collect(),
                keccak256(key.to_be_bytes::<32>()),
            )?
        } else {
            (B256::ZERO, vec![])
        };
        if root != value.as_ref().map(|v| v.storage_root).unwrap_or(B256::ZERO) {
            return Err(RpcError(-32603, "storage proof root differs"));
        }
        let stored = account
            .as_ref()
            .and_then(|a| a.storage.get(key))
            .copied()
            .unwrap_or_default();
        storage.push(
            json!({"key":format!("0x{key:x}"),"value":format!("0x{stored:x}"),"proof":proof}),
        );
    }
    // Match Geth's zero code/storage hashes for a nonexistent account; its
    // account exclusion proof, rather than a storage trie, establishes zero slots.
    Ok(
        json!({"address":address,"balance":format!("0x{:x}",value.as_ref().map(|v|v.balance).unwrap_or_default()),"nonce":format!("0x{:x}",value.as_ref().map(|v|v.nonce).unwrap_or_default()),"codeHash":value.as_ref().map(|v|v.code_hash).unwrap_or(B256::ZERO),"storageHash":value.as_ref().map(|v|v.storage_root).unwrap_or(B256::ZERO),"accountProof":proof,"storageProof":storage}),
    )
}
