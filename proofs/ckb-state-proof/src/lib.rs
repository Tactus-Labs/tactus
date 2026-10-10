//! Bounded Ethereum state witnesses authenticated by a canonical SettlementTip.
//! Typed read certificates do not authorize vault withdrawals or fund release.
#![cfg_attr(target_arch = "riscv64", no_std)]
extern crate alloc;
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Bytes, B256, U256};
use alloy_trie::{proof::verify_proof, Nibbles, TrieAccount, EMPTY_ROOT_HASH};

pub const CLAIM_BYTES: usize = 272;
pub const MAX_NODES: usize = 65;
pub const MAX_NODE_BYTES: usize = 1024;
pub const MAX_PROOF_BYTES: usize = 128 * 1024;
pub const MAX_WITNESS_BYTES: usize = MAX_PROOF_BYTES + 4096;
pub const MAX_CELLS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub root: B256,
    pub header: B256,
    pub settled_batches: u64,
    pub address: [u8; 20],
    pub exists: bool,
    pub nonce: u64,
    pub balance: U256,
    pub storage_root: B256,
    pub code_hash: B256,
    pub slot: B256,
    pub value: U256,
}
impl Claim {
    pub fn decode(bytes: &[u8]) -> Result<Self, i8> {
        if bytes.len() != CLAIM_BYTES
            || &bytes[..8] != b"TO1EVMR1"
            || bytes[100] > 1
            || bytes[101..104] != [0; 3]
        {
            return Err(3);
        }
        let claim = Self {
            root: B256::from_slice(&bytes[8..40]),
            header: B256::from_slice(&bytes[40..72]),
            settled_batches: u64::from_le_bytes(bytes[72..80].try_into().unwrap()),
            address: bytes[80..100].try_into().unwrap(),
            exists: bytes[100] == 1,
            nonce: u64::from_le_bytes(bytes[104..112].try_into().unwrap()),
            balance: U256::from_be_slice(&bytes[112..144]),
            storage_root: B256::from_slice(&bytes[144..176]),
            code_hash: B256::from_slice(&bytes[176..208]),
            slot: B256::from_slice(&bytes[208..240]),
            value: U256::from_be_slice(&bytes[240..272]),
        };
        if claim.root.is_zero()
            || claim.header.is_zero()
            || claim.settled_batches == 0
            || (!claim.exists
                && (claim.nonce != 0
                    || !claim.balance.is_zero()
                    || !claim.storage_root.is_zero()
                    || !claim.code_hash.is_zero()
                    || !claim.value.is_zero()))
        {
            return Err(3);
        }
        Ok(claim)
    }
    pub fn encode(&self) -> [u8; CLAIM_BYTES] {
        let mut out = [0; CLAIM_BYTES];
        out[..8].copy_from_slice(b"TO1EVMR1");
        out[8..40].copy_from_slice(self.root.as_slice());
        out[40..72].copy_from_slice(self.header.as_slice());
        out[72..80].copy_from_slice(&self.settled_batches.to_le_bytes());
        out[80..100].copy_from_slice(&self.address);
        out[100] = u8::from(self.exists);
        out[104..112].copy_from_slice(&self.nonce.to_le_bytes());
        out[112..144].copy_from_slice(&self.balance.to_be_bytes::<32>());
        out[144..176].copy_from_slice(self.storage_root.as_slice());
        out[176..208].copy_from_slice(self.code_hash.as_slice());
        out[208..240].copy_from_slice(self.slot.as_slice());
        out[240..272].copy_from_slice(&self.value.to_be_bytes::<32>());
        out
    }
}

pub fn bind_tip(claim: &Claim, tip: &[u8]) -> Result<(), i8> {
    if tip.len() != 280
        || &tip[..8] != b"TO1TIP01"
        || tip[8] != 1
        || tip[9..16] != [0; 7]
        || &tip[16..24] != b"TO1ANC01"
    {
        return Err(4);
    }
    if tip[216..248] != claim.root.0
        || tip[248..280] != claim.header.0
        || tip[56..64] != claim.settled_batches.to_le_bytes()
    {
        return Err(8);
    }
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], i8> {
        let end = self.offset.checked_add(size).ok_or(5)?;
        let value = self.bytes.get(self.offset..end).ok_or(5)?;
        self.offset = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<usize, i8> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize)
    }
    fn nodes(&mut self) -> Result<Vec<Bytes>, i8> {
        let count = self.u16()?;
        if count > MAX_NODES {
            return Err(5);
        }
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let len = self.u16()?;
            if len == 0 || len > MAX_NODE_BYTES {
                return Err(5);
            }
            out.push(Bytes::copy_from_slice(self.take(len)?));
        }
        Ok(out)
    }
}
pub fn encode_proofs(account: &[Bytes], storage: &[Bytes]) -> Result<Vec<u8>, i8> {
    let mut out = Vec::from(&b"TO1MPW01"[..]);
    for nodes in [account, storage] {
        if nodes.len() > MAX_NODES {
            return Err(5);
        }
        out.extend_from_slice(&(nodes.len() as u16).to_le_bytes());
        for node in nodes {
            if node.is_empty() || node.len() > MAX_NODE_BYTES {
                return Err(5);
            }
            out.extend_from_slice(&(node.len() as u16).to_le_bytes());
            out.extend_from_slice(node);
        }
    }
    if out.len() > MAX_PROOF_BYTES {
        return Err(5);
    }
    Ok(out)
}
pub fn verify(claim: &Claim, bytes: &[u8]) -> Result<(), i8> {
    Claim::decode(&claim.encode())?;
    if bytes.len() > MAX_PROOF_BYTES {
        return Err(5);
    }
    let mut reader = Reader { bytes, offset: 0 };
    if reader.take(8)? != b"TO1MPW01" {
        return Err(5);
    }
    let account = reader.nodes()?;
    let storage = reader.nodes()?;
    if reader.offset != bytes.len() {
        return Err(5);
    }
    // The empty trie has the unique empty proof encoding in this wire protocol.
    if claim.root == EMPTY_ROOT_HASH && !account.is_empty() {
        return Err(6);
    }
    let value = claim.exists.then(|| {
        alloy_rlp::encode(TrieAccount::new(
            claim.nonce,
            claim.balance,
            claim.storage_root,
            claim.code_hash,
        ))
    });
    verify_proof(
        claim.root,
        Nibbles::unpack(keccak256(claim.address)),
        value,
        account.iter(),
    )
    .map_err(|_| 6)?;
    if !claim.exists {
        return if storage.is_empty() && claim.value.is_zero() {
            Ok(())
        } else {
            Err(7)
        };
    }
    if claim.storage_root == EMPTY_ROOT_HASH && !storage.is_empty() {
        return Err(7);
    }
    let value = (!claim.value.is_zero()).then(|| alloy_rlp::encode(claim.value));
    verify_proof(
        claim.storage_root,
        Nibbles::unpack(keccak256(claim.slot)),
        value,
        storage.iter(),
    )
    .map_err(|_| 7)
}

#[cfg(all(target_arch = "riscv64", feature = "entry"))]
mod onchain {
    use super::*;
    use alloc::vec;
    use ckb_std::{
        ckb_constants::Source,
        ckb_types::{packed::WitnessArgs, prelude::*},
        error::SysError,
        high_level::*,
        syscalls,
    };
    ckb_std::default_alloc!();
    ckb_std::entry!(entrypoint);
    fn entrypoint() -> i8 {
        run().map_or_else(|e| e, |()| 0)
    }
    fn data<const N: usize>(index: usize, source: Source) -> Result<[u8; N], i8> {
        let mut out = [0; N];
        if syscalls::load_cell_data(&mut out, 0, index, source) != Ok(N) {
            return Err(3);
        }
        Ok(out)
    }
    fn dependency(identity: &[u8; 32]) -> Result<[u8; 280], i8> {
        let mut found = None;
        for index in 0..=MAX_CELLS {
            match load_cell_type_hash(index, Source::CellDep) {
                Err(SysError::IndexOutOfBound) => return found.ok_or(4),
                Ok(hash) if index < MAX_CELLS => {
                    if hash == Some(*identity) {
                        if found.is_some() {
                            return Err(4);
                        }
                        found = Some(data(index, Source::CellDep).map_err(|_| 4)?);
                    }
                }
                _ => return Err(4),
            }
        }
        Err(4)
    }
    fn run() -> Result<(), i8> {
        let script = load_script().map_err(|_| 1)?;
        let args = script.args().raw_data();
        if args.len() != 40 || &args[..8] != b"TO1MPT01" || args[8..] == [0; 32] {
            return Err(1);
        }
        if load_cell_capacity(1, Source::GroupInput) != Err(SysError::IndexOutOfBound)
            || load_cell_capacity(1, Source::GroupOutput) != Err(SysError::IndexOutOfBound)
        {
            return Err(2);
        }
        let input = load_cell_capacity(0, Source::GroupInput);
        let output = load_cell_capacity(0, Source::GroupOutput);
        // Immutable certificates may be destroyed under their ordinary owner lock.
        if input.is_ok() && output == Err(SysError::IndexOutOfBound) {
            return Ok(());
        }
        if input != Err(SysError::IndexOutOfBound) || output.is_err() {
            return Err(2);
        }
        let claim = Claim::decode(&data::<CLAIM_BYTES>(0, Source::GroupOutput)?)?;
        bind_tip(&claim, &dependency(&args[8..].try_into().map_err(|_| 1)?)?)?;
        let size = match syscalls::load_witness(&mut [], 0, 0, Source::Input) {
            Ok(size) | Err(SysError::LengthNotEnough(size)) => size,
            _ => return Err(5),
        };
        if size > MAX_WITNESS_BYTES {
            return Err(5);
        }
        let mut raw = vec![0; size];
        syscalls::load_witness(&mut raw, 0, 0, Source::Input).map_err(|_| 5)?;
        let witness = WitnessArgs::from_slice(&raw).map_err(|_| 5)?;
        let proof = witness.output_type().to_opt().ok_or(5)?.raw_data();
        verify(&claim, &proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn number(value: &Value) -> U256 {
        U256::from_str_radix(value.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
    }
    fn hash(value: &Value) -> B256 {
        value.as_str().unwrap().parse().unwrap()
    }
    fn nodes(value: &Value) -> Vec<Bytes> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect()
    }
    fn cases() -> Vec<(Claim, Vec<u8>)> {
        let source: Value = serde_json::from_str(include_str!(
            "../../../specs/evidence/observer-state-proofs/geth-fixture-proofs.json"
        ))
        .unwrap();
        let mut out = Vec::new();
        for row in source.as_array().unwrap() {
            let account = &row["result"];
            for storage in account["storageProof"].as_array().unwrap() {
                let claim = Claim {
                    root: hash(&row["stateRoot"]),
                    header: hash(&row["blockHash"]),
                    settled_batches: 1,
                    address: account["address"]
                        .as_str()
                        .unwrap()
                        .parse::<alloy_primitives::Address>()
                        .unwrap()
                        .0
                         .0,
                    exists: hash(&account["codeHash"]) != B256::ZERO,
                    nonce: number(&account["nonce"]).to::<u64>(),
                    balance: number(&account["balance"]),
                    storage_root: hash(&account["storageHash"]),
                    code_hash: hash(&account["codeHash"]),
                    slot: B256::from(number(&storage["key"]).to_be_bytes::<32>()),
                    value: number(&storage["value"]),
                };
                let bytes =
                    encode_proofs(&nodes(&account["accountProof"]), &nodes(&storage["proof"]))
                        .unwrap();
                out.push((claim, bytes));
            }
        }
        out
    }
    #[test]
    fn pure_mpt_verifier_accepts_independently_checked_account_and_storage_cases() {
        let cases = cases();
        assert_eq!(cases.len(), 285);
        for (claim, bytes) in cases {
            assert_eq!(Claim::decode(&claim.encode()), Ok(claim.clone()));
            assert_eq!(verify(&claim, &bytes), Ok(()));
        }
    }
    #[test]
    fn account_storage_and_absence_forgeries_fail() {
        let all = cases();
        let (claim, bytes) = all.iter().find(|(c, _)| !c.balance.is_zero()).unwrap();
        for field in 0..6 {
            let mut bad = claim.clone();
            match field {
                0 => bad.root.0[0] ^= 1,
                1 => bad.address[0] ^= 1,
                2 => bad.balance += U256::from(1),
                3 => bad.nonce += 1,
                4 => bad.storage_root.0[0] ^= 1,
                _ => bad.code_hash.0[0] ^= 1,
            };
            assert!(verify(&bad, bytes).is_err());
        }
        let (claim, bytes) = all.iter().find(|(c, _)| !c.value.is_zero()).unwrap();
        let mut bad = claim.clone();
        bad.value += U256::from(1);
        assert_eq!(verify(&bad, bytes), Err(7));
        bad = claim.clone();
        bad.slot.0[0] ^= 1;
        assert_eq!(verify(&bad, bytes), Err(7));
        let (claim, bytes) = all.iter().find(|(c, _)| !c.exists).unwrap();
        let mut bad = claim.clone();
        bad.balance = U256::from(1);
        assert_eq!(verify(&bad, bytes), Err(3));
    }
    #[test]
    fn proof_codec_rejects_malformed_and_oversized_inputs() {
        let (claim, bytes) = cases().remove(0);
        for cut in [0, 7, 8, 9, bytes.len() - 1] {
            assert_eq!(verify(&claim, &bytes[..cut]), Err(5));
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(verify(&claim, &trailing), Err(5));
        let mut count = bytes.clone();
        count[8..10].copy_from_slice(&66u16.to_le_bytes());
        assert_eq!(verify(&claim, &count), Err(5));
        let mut length = bytes.clone();
        length[10..12].copy_from_slice(&1025u16.to_le_bytes());
        assert_eq!(verify(&claim, &length), Err(5));
        let mut corrupt = bytes;
        corrupt[12] ^= 1;
        assert_eq!(verify(&claim, &corrupt), Err(6));
        assert_eq!(verify(&claim, &alloc::vec![0;MAX_PROOF_BYTES+1]), Err(5));
    }
    #[test]
    fn claim_must_bind_initialized_tip_root_header_and_batch_boundary() {
        let (claim, _) = cases().remove(0);
        let mut tip = [0u8; 280];
        tip[..8].copy_from_slice(b"TO1TIP01");
        tip[8] = 1;
        tip[16..24].copy_from_slice(b"TO1ANC01");
        tip[56..64].copy_from_slice(&claim.settled_batches.to_le_bytes());
        tip[216..248].copy_from_slice(claim.root.as_slice());
        tip[248..280].copy_from_slice(claim.header.as_slice());
        assert_eq!(bind_tip(&claim, &tip), Ok(()));
        for offset in [56, 216, 248] {
            let mut bad = tip;
            bad[offset] ^= 1;
            assert_eq!(bind_tip(&claim, &bad), Err(8));
        }
        for offset in [0, 8, 9, 16] {
            let mut bad = tip;
            bad[offset] ^= 1;
            assert_eq!(bind_tip(&claim, &bad), Err(4));
        }
        assert_eq!(bind_tip(&claim, &tip[..279]), Err(4));
        let mut data = claim.encode();
        data[101] = 1;
        assert_eq!(Claim::decode(&data), Err(3));
    }
}
