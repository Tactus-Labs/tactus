//! Native CKB custody rules. A valid vault configuration still requires an
//! authenticated bridge execution profile and a real proof-settled bridge root.
#![cfg_attr(target_arch = "riscv64", no_std)]
extern crate alloc;
use alloc::vec::Vec;
use alloy_primitives::{keccak256, B256, U256};
use tactus_o1_state_proof_script::{bind_tip, verify, Claim};
mod runtime {
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}
pub const CONFIG_BYTES: usize = 164;
pub const STATE_BYTES: usize = 120;
pub const RECORD_BYTES: usize = 124;
pub const MAX_CELLS: usize = 64;
pub const RELEASE_PREFIX: usize = 2404;

pub fn hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut out = [0; 32];
    let mut h = blake2b_ref::Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    h.update(domain);
    h.update(bytes);
    h.finalize(&mut out);
    out
}
fn n(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap())
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub identity: [u8; 32],
    pub ckb_genesis: [u8; 32],
    pub rollup: [u8; 32],
    pub chain: u64,
    pub contract: [u8; 20],
    pub settlement: [u8; 32],
}
impl Config {
    pub fn decode(bytes: &[u8]) -> Result<Self, i8> {
        if bytes.len() != CONFIG_BYTES || &bytes[..8] != b"TO1VAU01" {
            return Err(1);
        }
        let c = Self {
            identity: bytes[8..40].try_into().unwrap(),
            ckb_genesis: bytes[40..72].try_into().unwrap(),
            rollup: bytes[72..104].try_into().unwrap(),
            chain: n(&bytes[104..112]),
            contract: bytes[112..132].try_into().unwrap(),
            settlement: bytes[132..164].try_into().unwrap(),
        };
        if [c.identity, c.ckb_genesis, c.rollup, c.settlement].contains(&[0; 32])
            || c.chain == 0
            || c.contract == [0; 20]
        {
            return Err(1);
        }
        Ok(c)
    }
    pub fn encode(&self) -> [u8; CONFIG_BYTES] {
        let mut b = [0; CONFIG_BYTES];
        b[..8].copy_from_slice(b"TO1VAU01");
        b[8..40].copy_from_slice(&self.identity);
        b[40..72].copy_from_slice(&self.ckb_genesis);
        b[72..104].copy_from_slice(&self.rollup);
        b[104..112].copy_from_slice(&self.chain.to_le_bytes());
        b[112..132].copy_from_slice(&self.contract);
        b[132..].copy_from_slice(&self.settlement);
        b
    }
    pub fn domain(&self) -> B256 {
        let bridge = keccak256(
            [
                b"TO1CKBD1".as_slice(),
                &self.ckb_genesis,
                &self.rollup,
                &self.identity,
            ]
            .concat(),
        );
        keccak256(
            [
                b"TO1BRDG1".as_slice(),
                bridge.as_slice(),
                &U256::from(self.chain).to_be_bytes::<32>(),
                &self.contract,
            ]
            .concat(),
        )
    }
    pub fn runtime_hash(&self) -> B256 {
        let mut code = runtime::TEMPLATE.to_vec();
        let domain = self.domain();
        for offset in runtime::DOMAIN_OFFSETS {
            code[*offset..*offset + 32].copy_from_slice(domain.as_slice());
        }
        keccak256(code)
    }
    pub fn empty_deposits(&self) -> [u8; 32] {
        hash(b"tactus/o1/deposits/empty/v1", &self.encode())
    }
    pub fn deposit_id(&self, sequence: u64) -> [u8; 32] {
        hash(
            b"tactus/o1/deposit/id/v1",
            &[self.encode().as_slice(), &sequence.to_le_bytes()].concat(),
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub reserve: u64,
    pub deposited: u128,
    pub released: u128,
    pub count: u64,
    pub deposits: [u8; 32],
    pub claimed: [u8; 32],
}
impl State {
    pub fn genesis(config: &Config, reserve: u64) -> Self {
        Self {
            reserve,
            deposited: 0,
            released: 0,
            count: 0,
            deposits: config.empty_deposits(),
            claimed: empty_claims(),
        }
    }
    pub fn decode(b: &[u8]) -> Result<Self, i8> {
        if b.len() != STATE_BYTES || &b[..8] != b"TO1VST01" {
            return Err(3);
        }
        let s = Self {
            reserve: n(&b[8..16]),
            deposited: u128::from_le_bytes(b[16..32].try_into().unwrap()),
            released: u128::from_le_bytes(b[32..48].try_into().unwrap()),
            count: n(&b[48..56]),
            deposits: b[56..88].try_into().unwrap(),
            claimed: b[88..120].try_into().unwrap(),
        };
        s.capacity()?;
        Ok(s)
    }
    pub fn encode(&self) -> [u8; STATE_BYTES] {
        let mut b = [0; STATE_BYTES];
        b[..8].copy_from_slice(b"TO1VST01");
        b[8..16].copy_from_slice(&self.reserve.to_le_bytes());
        b[16..32].copy_from_slice(&self.deposited.to_le_bytes());
        b[32..48].copy_from_slice(&self.released.to_le_bytes());
        b[48..56].copy_from_slice(&self.count.to_le_bytes());
        b[56..88].copy_from_slice(&self.deposits);
        b[88..120].copy_from_slice(&self.claimed);
        b
    }
    pub fn capacity(&self) -> Result<u64, i8> {
        self.reserve
            .checked_add(
                u64::try_from(self.deposited.checked_sub(self.released).ok_or(4)?)
                    .map_err(|_| 4)?,
            )
            .ok_or(4)
    }
    pub fn deposit(
        &self,
        cfg: &Config,
        recipient: [u8; 20],
        amount: u64,
    ) -> Result<(Self, Record), i8> {
        self.capacity()?;
        if recipient == [0; 20] || recipient == cfg.contract || amount == 0 {
            return Err(5);
        }
        let mut next = self.clone();
        next.count = self.count.checked_add(1).ok_or(5)?;
        next.deposited = self.deposited.checked_add(u128::from(amount)).ok_or(5)?;
        let mut record = Record {
            sequence: next.count,
            recipient,
            amount,
            before: self.deposits,
            after: [0; 32],
            cumulative: next.deposited,
        };
        record.after = hash(
            b"tactus/o1/deposits/append/v1",
            &[
                cfg.encode().as_slice(),
                &record.encode()[..76],
                &record.cumulative.to_le_bytes(),
            ]
            .concat(),
        );
        next.deposits = record.after;
        next.capacity()?;
        Ok((next, record))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub sequence: u64,
    pub recipient: [u8; 20],
    pub amount: u64,
    pub before: [u8; 32],
    pub after: [u8; 32],
    pub cumulative: u128,
}
impl Record {
    pub fn decode(b: &[u8]) -> Result<Self, i8> {
        if b.len() != RECORD_BYTES || &b[..8] != b"TO1DPR01" {
            return Err(5);
        }
        Ok(Self {
            sequence: n(&b[8..16]),
            recipient: b[16..36].try_into().unwrap(),
            amount: n(&b[36..44]),
            before: b[44..76].try_into().unwrap(),
            after: b[76..108].try_into().unwrap(),
            cumulative: u128::from_le_bytes(b[108..124].try_into().unwrap()),
        })
    }
    pub fn encode(&self) -> [u8; RECORD_BYTES] {
        let mut b = [0; RECORD_BYTES];
        b[..8].copy_from_slice(b"TO1DPR01");
        b[8..16].copy_from_slice(&self.sequence.to_le_bytes());
        b[16..36].copy_from_slice(&self.recipient);
        b[36..44].copy_from_slice(&self.amount.to_le_bytes());
        b[44..76].copy_from_slice(&self.before);
        b[76..108].copy_from_slice(&self.after);
        b[108..].copy_from_slice(&self.cumulative.to_le_bytes());
        b
    }
}
fn branch(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    hash(
        b"tactus/o1/claims/branch/v1",
        &[left.as_slice(), right.as_slice()].concat(),
    )
}
fn leaf(claimed: bool) -> [u8; 32] {
    hash(b"tactus/o1/claims/leaf/v1", &[u8::from(claimed)])
}
pub fn empty_claims() -> [u8; 32] {
    let mut root = leaf(false);
    for _ in 0..64 {
        root = branch(root, root)
    }
    root
}
pub fn empty_siblings() -> [[u8; 32]; 64] {
    let mut out = [[0; 32]; 64];
    let mut root = leaf(false);
    for item in &mut out {
        *item = root;
        root = branch(root, root)
    }
    out
}
pub fn claim_once(root: [u8; 32], id: u64, siblings: &[[u8; 32]; 64]) -> Result<[u8; 32], i8> {
    if id == 0 {
        return Err(7);
    }
    let mut before = leaf(false);
    let mut after = leaf(true);
    for (level, sibling) in siblings.iter().enumerate() {
        if (id >> level) & 1 == 0 {
            before = branch(before, *sibling);
            after = branch(after, *sibling)
        } else {
            before = branch(*sibling, before);
            after = branch(*sibling, after)
        }
    }
    if before != root {
        return Err(7);
    }
    Ok(after)
}
pub struct Release {
    pub claim: Claim,
    pub id: u64,
    pub amount: u64,
    pub recipient: [u8; 32],
    pub owner: [u8; 20],
    pub payout: u32,
    pub siblings: [[u8; 32]; 64],
    pub proof: Vec<u8>,
}
impl Release {
    pub fn encode(&self) -> Result<Vec<u8>, i8> {
        if self.proof.len() > tactus_o1_state_proof_script::MAX_PROOF_BYTES {
            return Err(6);
        }
        let mut out = Vec::from(&b"TO1WDR01"[..]);
        out.extend_from_slice(&self.claim.encode());
        out.extend_from_slice(&self.id.to_le_bytes());
        out.extend_from_slice(&self.amount.to_le_bytes());
        out.extend_from_slice(&self.recipient);
        out.extend_from_slice(&self.owner);
        out.extend_from_slice(&self.payout.to_le_bytes());
        for sibling in &self.siblings {
            out.extend_from_slice(sibling);
        }
        out.extend_from_slice(&(self.proof.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.proof);
        Ok(out)
    }
    pub fn decode(b: &[u8]) -> Result<Self, i8> {
        if b.len() < RELEASE_PREFIX
            || b.len() > RELEASE_PREFIX + tactus_o1_state_proof_script::MAX_PROOF_BYTES
            || &b[..8] != b"TO1WDR01"
        {
            return Err(6);
        }
        let len = u32::from_le_bytes(b[2400..2404].try_into().unwrap()) as usize;
        if b.len() != RELEASE_PREFIX + len {
            return Err(6);
        }
        let mut siblings = [[0; 32]; 64];
        for (i, s) in siblings.iter_mut().enumerate() {
            s.copy_from_slice(&b[352 + i * 32..384 + i * 32]);
        }
        Ok(Self {
            claim: Claim::decode(&b[8..280]).map_err(|_| 6)?,
            id: n(&b[280..288]),
            amount: n(&b[288..296]),
            recipient: b[296..328].try_into().unwrap(),
            owner: b[328..348].try_into().unwrap(),
            payout: u32::from_le_bytes(b[348..352].try_into().unwrap()),
            siblings,
            proof: b[RELEASE_PREFIX..].to_vec(),
        })
    }
    pub fn commitment(&self, cfg: &Config) -> B256 {
        keccak256(
            [
                b"TO1EXIT1".as_slice(),
                cfg.domain().as_slice(),
                &self.id.to_be_bytes(),
                &self.amount.to_be_bytes(),
                &self.recipient,
                &self.owner,
            ]
            .concat(),
        )
    }
    pub fn verify(&self, cfg: &Config, current: &State, tip: &[u8]) -> Result<State, i8> {
        current.capacity()?;
        if self.id == 0 || self.amount == 0 || self.recipient == [0; 32] || self.owner == [0; 20] {
            return Err(6);
        }
        let slot = keccak256(
            [
                U256::from(self.id).to_be_bytes::<32>(),
                U256::from(7).to_be_bytes::<32>(),
            ]
            .concat(),
        );
        if !self.claim.exists
            || self.claim.address != cfg.contract
            || self.claim.code_hash != cfg.runtime_hash()
            || self.claim.slot != slot
            || self.claim.value != U256::from_be_bytes(self.commitment(cfg).0)
        {
            return Err(8);
        }
        bind_tip(&self.claim, tip).map_err(|_| 9)?;
        verify(&self.claim, &self.proof).map_err(|_| 10)?;
        let mut next = current.clone();
        next.released = current
            .released
            .checked_add(u128::from(self.amount))
            .ok_or(4)?;
        next.capacity()?;
        next.claimed = claim_once(current.claimed, self.id, &self.siblings)?;
        Ok(next)
    }
}

#[cfg(target_arch = "riscv64")]
mod onchain;
