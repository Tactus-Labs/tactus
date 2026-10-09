//! Immutable, transition-authenticated Anchor history checkpoints.
//! Consumers must pin the full checkpoint type and its Anchor type-hash argument.
#![cfg_attr(target_arch = "riscv64", no_std)]
#[cfg(target_arch = "riscv64")]
extern crate alloc;

pub const ANCHOR_BYTES: usize = 200;
pub const MAX_CELLS: usize = 64;

/// Defense in depth: the pinned Anchor program enforces the full transition.
pub fn advanced(before: &[u8; ANCHOR_BYTES], after: &[u8; ANCHOR_BYTES]) -> bool {
    before[..8] == *b"TO1ANC01"
        && after[..8] == *b"TO1ANC01"
        && before[8..40] == after[8..40]
        && before[96..] == after[96..]
        && u64::from_le_bytes(before[40..48].try_into().unwrap()).checked_add(1)
            == Some(u64::from_le_bytes(after[40..48].try_into().unwrap()))
}

#[cfg(target_arch = "riscv64")]
mod onchain {
    use super::*;
    use ckb_std::{
        ckb_constants::Source,
        error::SysError,
        high_level::{load_cell_capacity, load_cell_type_hash, load_script},
        syscalls,
    };
    ckb_std::default_alloc!();
    ckb_std::entry!(entrypoint);

    fn entrypoint() -> i8 {
        run().map_or_else(|error| error, |()| 0)
    }

    fn unique_anchor(source: Source, identity: &[u8; 32]) -> Result<usize, i8> {
        let mut found = None;
        for index in 0..=MAX_CELLS {
            match load_cell_type_hash(index, source) {
                Err(SysError::IndexOutOfBound) => return found.ok_or(4),
                Ok(value) if index < MAX_CELLS => {
                    if value == Some(*identity) && found.replace(index).is_some() {
                        return Err(4);
                    }
                }
                _ => return Err(4),
            }
        }
        Err(4)
    }

    fn data(index: usize, source: Source) -> Result<[u8; ANCHOR_BYTES], i8> {
        let mut data = [0; ANCHOR_BYTES];
        if syscalls::load_cell_data(&mut data, 0, index, source) != Ok(ANCHOR_BYTES) {
            return Err(5);
        }
        Ok(data)
    }

    fn run() -> Result<(), i8> {
        let script = load_script().map_err(|_| 1)?;
        let identity: [u8; 32] = script
            .args()
            .raw_data()
            .as_ref()
            .try_into()
            .map_err(|_| 1)?;
        if load_cell_capacity(0, Source::GroupInput) != Err(SysError::IndexOutOfBound)
            || load_cell_capacity(1, Source::GroupOutput) != Err(SysError::IndexOutOfBound)
        {
            return Err(2);
        }
        let checkpoint = data(0, Source::GroupOutput)?;
        // A cell_dep is intentionally insufficient: the named Anchor must be
        // consumed and recreated in this transaction, so its type program runs.
        let input = unique_anchor(Source::Input, &identity)?;
        let output = unique_anchor(Source::Output, &identity)?;
        let before = data(input, Source::Input)?;
        let after = data(output, Source::Output)?;
        if !advanced(&before, &after) {
            return Err(6);
        }
        if checkpoint != after {
            return Err(7);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_exact_successor_and_stable_domain_without_overflow() {
        let mut before = [0; ANCHOR_BYTES];
        before[..8].copy_from_slice(b"TO1ANC01");
        let mut after = before;
        after[40..48].copy_from_slice(&1u64.to_le_bytes());
        assert!(advanced(&before, &after));
        assert!(!advanced(&before, &before));
        for index in (0..40).chain(96..ANCHOR_BYTES) {
            let mut changed = after;
            changed[index] ^= 1;
            assert!(!advanced(&before, &changed));
        }
        after[40..48].copy_from_slice(&2u64.to_le_bytes());
        assert!(!advanced(&before, &after));
        before[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        after[40..48].fill(0);
        assert!(!advanced(&before, &after));
    }
}
