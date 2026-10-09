//! OrderingHead type script — CKB-VM (RISC-V) with a host-testable core.
//!
//! The transition rules are the A1 reference semantics of the architecture
//! specification §4.1–4.2, devnet tier:
//!
//! - **ENQUEUE** (witness `input_type` carries a 32-byte message commitment):
//!   `inbox_root ← H(inbox_root ‖ msg)`, `inbox_tail += 1`; every batch field
//!   and every genesis-bound field unchanged.
//! - **APPEND_BATCH** (witness `input_type` carries a 32-byte batch
//!   commitment): `next_batch_number += 1`,
//!   `batch_accumulator_root ← H(accumulator ‖ batch_commitment)`; the
//!   processed cursor may advance up to `inbox_tail`; inbox root/tail and
//!   every genesis-bound field unchanged.
//!
//! No lock signature or operator certificate is consulted: succession is
//! permissionless (spec §4.2). Genesis-bound fields — `rollup_id`,
//! `protocol_version`, `execution_rules_hash`, `da_policy_id` — are frozen
//! for the cell's lifetime. The same blake2b-ref implementation backs host
//! tests and the on-chain build, so validated semantics are identical.

#![cfg_attr(target_arch = "riscv64", no_std)]
#[cfg(target_arch = "riscv64")]
extern crate alloc;

/// Blake2b-256 with the CKB personalization — identical on host and on-chain.
pub fn ckb_blake2b(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut hasher = blake2b_ref::Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    hasher.update(data);
    hasher.finalize(&mut out);
    out
}

/// Blake2b-160 with the CKB personalization (lock args / code hash prefix).
pub fn ckb_blakeb160(data: &[u8]) -> [u8; 20] {
    let mut out = [0u8; 20];
    let mut hasher = blake2b_ref::Blake2bBuilder::new(20)
        .personal(b"ckb-default-hash")
        .build();
    hasher.update(data);
    hasher.finalize(&mut out);
    out
}

/// Fixed-layout encoding of the OrderingHead (188 bytes; spec §3.2,
/// devnet-tier layout with a canonical round-trip test).
pub const HEAD_LEN: usize = 32 + 4 + 8 + 32 + 32 + 8 + 8 + 32 + 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderingHead {
    pub rollup_id: [u8; 32],
    pub protocol_version: u32,
    pub next_batch_number: u64,
    pub batch_accumulator_root: [u8; 32],
    pub inbox_root: [u8; 32],
    pub inbox_tail: u64,
    pub processed_inbox_cursor: u64,
    pub execution_rules_hash: [u8; 32],
    pub da_policy_id: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Witness commitment for one priority message.
    Enqueue { message_commitment: [u8; 32] },
    /// Witness commitment for one candidate batch.
    AppendBatch { batch_commitment: [u8; 32] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    BadLength,
    BadWitness,
    PreservedFieldMutated,
    BadEnqueue,
    BadAppendBatch,
    /// Script args do not match the head's rollup identity.
    IdentityMismatch,
}

fn chain_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(left);
    buf[32..].copy_from_slice(right);
    ckb_blake2b(&buf)
}

impl OrderingHead {
    /// Canonical little-endian fixed layout.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; HEAD_LEN] {
        let mut b = [0u8; HEAD_LEN];
        b[0..32].copy_from_slice(&self.rollup_id);
        b[32..36].copy_from_slice(&self.protocol_version.to_le_bytes());
        b[36..44].copy_from_slice(&self.next_batch_number.to_le_bytes());
        b[44..76].copy_from_slice(&self.batch_accumulator_root);
        b[76..108].copy_from_slice(&self.inbox_root);
        b[108..116].copy_from_slice(&self.inbox_tail.to_le_bytes());
        b[116..124].copy_from_slice(&self.processed_inbox_cursor.to_le_bytes());
        b[124..156].copy_from_slice(&self.execution_rules_hash);
        b[156..188].copy_from_slice(&self.da_policy_id);
        b
    }

    pub fn from_bytes(b: &[u8]) -> Result<Self, ValidationError> {
        if b.len() != HEAD_LEN {
            return Err(ValidationError::BadLength);
        }
        let arr32 = |r: core::ops::Range<usize>| -> [u8; 32] { b[r].try_into().expect("32 bytes") };
        Ok(Self {
            rollup_id: arr32(0..32),
            protocol_version: u32::from_le_bytes(b[32..36].try_into().expect("4")),
            next_batch_number: u64::from_le_bytes(b[36..44].try_into().expect("8")),
            batch_accumulator_root: arr32(44..76),
            inbox_root: arr32(76..108),
            inbox_tail: u64::from_le_bytes(b[108..116].try_into().expect("8")),
            processed_inbox_cursor: u64::from_le_bytes(b[116..124].try_into().expect("8")),
            execution_rules_hash: arr32(124..156),
            da_policy_id: arr32(156..188),
        })
    }

    /// Validate one OrderingHead transition (spec §4.1–4.2).
    pub fn validate_transition(
        &self,
        next: &OrderingHead,
        transition: Transition,
    ) -> Result<(), ValidationError> {
        // Genesis-bound identity (spec §3.1, §10.2): immutable for life.
        if self.rollup_id != next.rollup_id
            || self.protocol_version != next.protocol_version
            || self.execution_rules_hash != next.execution_rules_hash
            || self.da_policy_id != next.da_policy_id
        {
            return Err(ValidationError::PreservedFieldMutated);
        }
        match transition {
            Transition::Enqueue { message_commitment } => {
                if next.inbox_root != chain_hash(&self.inbox_root, &message_commitment)
                    || next.inbox_tail != self.inbox_tail.wrapping_add(1)
                    || next.next_batch_number != self.next_batch_number
                    || next.batch_accumulator_root != self.batch_accumulator_root
                    || next.processed_inbox_cursor != self.processed_inbox_cursor
                {
                    return Err(ValidationError::BadEnqueue);
                }
            }
            Transition::AppendBatch { batch_commitment } => {
                if next.batch_accumulator_root
                    != chain_hash(&self.batch_accumulator_root, &batch_commitment)
                    || next.next_batch_number != self.next_batch_number.wrapping_add(1)
                    || next.inbox_root != self.inbox_root
                    || next.inbox_tail != self.inbox_tail
                    || next.processed_inbox_cursor > next.inbox_tail
                    || next.processed_inbox_cursor < self.processed_inbox_cursor
                {
                    return Err(ValidationError::BadAppendBatch);
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_arch = "riscv64")]
mod onchain {
    use crate::{OrderingHead, Transition, ValidationError, HEAD_LEN};
    ckb_std::default_alloc!();

    use ckb_std::ckb_constants::Source;
    use ckb_std::ckb_types::prelude::Unpack;
    use ckb_std::high_level::{load_cell_data, load_script, load_witness_args};

    ckb_std::entry!(script);

    fn error_code(e: ValidationError) -> i8 {
        match e {
            ValidationError::BadLength => 1,
            ValidationError::BadWitness => 2,
            ValidationError::PreservedFieldMutated => 3,
            ValidationError::BadEnqueue => 4,
            ValidationError::BadAppendBatch => 5,
            ValidationError::IdentityMismatch => 6,
        }
    }

    fn script() -> i8 {
        match run() {
            Ok(()) => 0,
            Err(e) => error_code(e),
        }
    }

    fn run() -> Result<(), ValidationError> {
        // Script args carry the rollup identity this deployment serves.
        let args = load_script()
            .map_err(|_| ValidationError::BadWitness)?
            .args();
        let args: alloc::vec::Vec<u8> = ckb_std::ckb_types::prelude::Unpack::unpack(&args);
        let head_args: [u8; 32] = args
            .as_slice()
            .try_into()
            .map_err(|_| ValidationError::IdentityMismatch)?;

        // The type-group's first input witness carries the 32-byte
        // commitment in WitnessArgs::input_type (devnet-tier convention).
        let witness_args =
            load_witness_args(0, Source::GroupInput).map_err(|_| ValidationError::BadWitness)?;
        let packed = witness_args
            .input_type()
            .to_opt()
            .ok_or(ValidationError::BadWitness)?;
        let commitment_bytes: alloc::vec::Vec<u8> =
            ckb_std::ckb_types::prelude::Unpack::unpack(&packed);
        if commitment_bytes.len() != 32 {
            return Err(ValidationError::BadWitness);
        }
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&commitment_bytes);

        let in_bytes =
            load_cell_data(0, Source::GroupInput).map_err(|_| ValidationError::BadWitness)?;
        let out_bytes =
            load_cell_data(0, Source::GroupOutput).map_err(|_| ValidationError::BadWitness)?;
        if in_bytes.len() != HEAD_LEN || out_bytes.len() != HEAD_LEN {
            return Err(ValidationError::BadLength);
        }
        let current = OrderingHead::from_bytes(&in_bytes)?;
        let next = OrderingHead::from_bytes(&out_bytes)?;
        if current.rollup_id != head_args {
            return Err(ValidationError::IdentityMismatch);
        }

        // ENQUEUE iff the inbox tail advances; both branches are then fully
        // validated, so a misclassified transition still fails.
        let transition = if next.inbox_tail == current.inbox_tail.wrapping_add(1) {
            Transition::Enqueue {
                message_commitment: commitment,
            }
        } else {
            Transition::AppendBatch {
                batch_commitment: commitment,
            }
        };
        current.validate_transition(&next, transition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn genesis() -> OrderingHead {
        OrderingHead {
            rollup_id: [7u8; 32],
            protocol_version: 1,
            next_batch_number: 0,
            batch_accumulator_root: [0u8; 32],
            inbox_root: [1u8; 32],
            inbox_tail: 0,
            processed_inbox_cursor: 0,
            execution_rules_hash: [9u8; 32],
            da_policy_id: [11u8; 32],
        }
    }

    #[test]
    fn encoding_roundtrip_is_canonical() {
        let g = genesis();
        assert_eq!(OrderingHead::from_bytes(&g.to_bytes()).unwrap(), g);
    }

    #[test]
    fn enqueue_updates_only_inbox() {
        let g = genesis();
        let msg = [5u8; 32];
        let mut n = g;
        n.inbox_root = chain_hash(&g.inbox_root, &msg);
        n.inbox_tail = 1;
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::Enqueue {
                    message_commitment: msg
                }
            ),
            Ok(())
        );
        // Wrong commitment → rejected.
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::Enqueue {
                    message_commitment: [6u8; 32]
                }
            ),
            Err(ValidationError::BadEnqueue)
        );
    }

    #[test]
    fn append_batch_updates_only_accumulator() {
        let g = genesis();
        let batch = [8u8; 32];
        let mut n = g;
        n.batch_accumulator_root = chain_hash(&g.batch_accumulator_root, &batch);
        n.next_batch_number = 1;
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::AppendBatch {
                    batch_commitment: batch
                }
            ),
            Ok(())
        );
        // Inbox mutated alongside the batch → rejected.
        n.inbox_tail = 1;
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::AppendBatch {
                    batch_commitment: batch
                }
            ),
            Err(ValidationError::BadAppendBatch)
        );
    }

    #[test]
    fn genesis_bound_fields_are_frozen() {
        let g = genesis();
        let mut n = g;
        n.da_policy_id = [12u8; 32];
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::AppendBatch {
                    batch_commitment: [0u8; 32]
                }
            ),
            Err(ValidationError::PreservedFieldMutated)
        );
    }

    #[test]
    fn cursor_may_advance_but_not_regress_or_pass_tail() {
        let mut g = genesis();
        g.inbox_tail = 5;
        g.processed_inbox_cursor = 2;
        let mut n = g;
        n.next_batch_number = 1;
        n.processed_inbox_cursor = 5; // may advance up to the tail
        n.batch_accumulator_root = chain_hash(&g.batch_accumulator_root, &[3u8; 32]);
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::AppendBatch {
                    batch_commitment: [3u8; 32]
                }
            ),
            Ok(())
        );
        n.processed_inbox_cursor = 6; // past the tail
        assert_eq!(
            g.validate_transition(
                &n,
                Transition::AppendBatch {
                    batch_commitment: [3u8; 32]
                }
            ),
            Err(ValidationError::BadAppendBatch)
        );
    }
}
