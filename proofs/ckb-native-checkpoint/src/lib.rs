//! Immutable snapshots of a consumed-and-recreated native publication anchor.
//! A consumer must pin this program and the exact trusted anchor type hash.
#![cfg_attr(target_arch = "riscv64", no_std)]
#[cfg(target_arch = "riscv64")]
extern crate alloc;
use tactus_o1_protocol::native_bridge::{Anchor, Config, MAX_DEPOSITS};
pub const STATE_BYTES: usize = 428;
pub const MAX_CELLS: usize = 64;

fn state(b: &[u8; STATE_BYTES]) -> Option<(Config, Anchor)> {
    if &b[..8] != b"TO1NAP02" {
        return None;
    }
    let c = Config::decode(&b[8..172]).ok()?;
    let a = Anchor::decode(&b[172..]).ok()?;
    if a.ordering.rollup_id != c.rollup || a.ordering.chain_id != c.chain {
        return None;
    }
    Some((c, a))
}
/// Defense in depth; the named anchor's type program enforces the full update
/// including authenticated funded receipts. Hashes are never accepted as data.
pub fn advanced(before: &[u8; STATE_BYTES], after: &[u8; STATE_BYTES]) -> bool {
    let Some((bc, b)) = state(before) else {
        return false;
    };
    let Some((ac, a)) = state(after) else {
        return false;
    };
    bc == ac
        && b.ordering.execution_rules_hash == a.ordering.execution_rules_hash
        && b.ordering.next_batch_number.checked_add(1) == Some(a.ordering.next_batch_number)
        && b.ordering.last_block_number < a.ordering.last_block_number
        && b.ordering.last_timestamp < a.ordering.last_timestamp
        && a.deposits
            .count
            .checked_sub(b.deposits.count)
            .is_some_and(|n| n <= MAX_DEPOSITS as u64)
        && ((a.deposits.count == b.deposits.count && a.deposits == b.deposits)
            || (a.deposits.count > b.deposits.count
                && a.deposits.cumulative > b.deposits.cumulative))
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
        run().map_or_else(|e| e, |()| 0)
    }
    fn unique(source: Source, hash: &[u8; 32]) -> Result<usize, i8> {
        let mut found = None;
        for i in 0..=MAX_CELLS {
            match load_cell_type_hash(i, source) {
                Err(SysError::IndexOutOfBound) => return found.ok_or(4),
                Ok(h) if i < MAX_CELLS => {
                    if h == Some(*hash) && found.replace(i).is_some() {
                        return Err(4);
                    }
                }
                _ => return Err(4),
            }
        }
        Err(4)
    }
    fn data(i: usize, s: Source) -> Result<[u8; STATE_BYTES], i8> {
        let mut b = [0; STATE_BYTES];
        if syscalls::load_cell_data(&mut b, 0, i, s) != Ok(STATE_BYTES) {
            return Err(5);
        }
        Ok(b)
    }
    fn run() -> Result<(), i8> {
        let script = load_script().map_err(|_| 1)?;
        let args = script.args().raw_data();
        if args.len() != 40 || &args[..8] != b"TO1NCP02" || script.hash_type() != 2u8.into() {
            return Err(1);
        }
        let identity: [u8; 32] = args[8..].try_into().map_err(|_| 1)?;
        if identity == [0; 32] {
            return Err(1);
        }
        if load_cell_capacity(0, Source::GroupInput) != Err(SysError::IndexOutOfBound)
            || load_cell_capacity(1, Source::GroupOutput) != Err(SysError::IndexOutOfBound)
        {
            return Err(2);
        }
        let checkpoint = data(0, Source::GroupOutput)?;
        // An input AND output are mandatory, so the trusted anchor program runs.
        // A live dependency, even with perfect data, provides no transition authority.
        let before = data(unique(Source::Input, &identity)?, Source::Input)?;
        let after = data(unique(Source::Output, &identity)?, Source::Output)?;
        if !advanced(&before, &after) {
            return Err(6);
        }
        if checkpoint != after {
            return Err(7);
        }
        Ok(())
    }
}
