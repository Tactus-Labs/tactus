//! Creation-only receipts of a Groth16-verified public journal.
//! This component does not authenticate canonical ordering or advance SettlementTip.
#![cfg_attr(target_arch = "riscv64", no_std)]
#[cfg(target_arch = "riscv64")]
extern crate alloc;

pub const JOURNAL_BYTES: usize = 768;
pub const MAX_WITNESS_BYTES: usize = 8192;
pub const MAX_PROOF_BYTES: usize = 4096;
const MAGIC: &[u8; 8] = b"TO1G1601";

/// Canonical witness input_type framing; excludes the public journal, which
/// must be read from the receipt's actual output data.
pub fn proof_bytes(encoded: &[u8]) -> Result<&[u8], i8> {
    if encoded.len() < 12 || &encoded[..8] != MAGIC {
        return Err(5);
    }
    let length = u32::from_le_bytes(encoded[8..12].try_into().unwrap()) as usize;
    if length == 0 || length > MAX_PROOF_BYTES || length != encoded.len() - 12 {
        return Err(5);
    }
    Ok(&encoded[12..])
}

pub fn key_hex(args: &[u8]) -> Result<[u8; 66], i8> {
    if args.len() != 32 {
        return Err(6);
    }
    let mut key = [0; 66];
    key[..2].copy_from_slice(b"0x");
    for (i, byte) in args.iter().enumerate() {
        key[2 + 2 * i] = b"0123456789abcdef"[(byte >> 4) as usize];
        key[3 + 2 * i] = b"0123456789abcdef"[(byte & 15) as usize];
    }
    Ok(key)
}

#[cfg(target_arch = "riscv64")]
mod onchain {
    use super::*;
    use alloc::vec;
    use ckb_std::{
        ckb_constants::Source,
        ckb_types::{packed::WitnessArgs, prelude::*},
        error::SysError,
        high_level::{load_cell_capacity, load_script},
        syscalls,
    };
    ckb_std::default_alloc!();
    ckb_std::entry!(entrypoint);

    fn entrypoint() -> i8 {
        run().map_or_else(|e| e, |()| 0)
    }
    fn run() -> Result<(), i8> {
        // Receipts cannot be consumed or merged. A repeated proof may create
        // another receipt, but cannot move or release any settlement/custody state.
        if load_cell_capacity(0, Source::GroupInput) != Err(SysError::IndexOutOfBound)
            || load_cell_capacity(1, Source::GroupOutput) != Err(SysError::IndexOutOfBound)
        {
            return Err(8);
        }
        let mut journal = [0; JOURNAL_BYTES];
        if syscalls::load_cell_data(&mut journal, 0, 0, Source::GroupOutput) != Ok(JOURNAL_BYTES) {
            return Err(9);
        }
        let len = match syscalls::load_witness(&mut [], 0, 0, Source::Input) {
            Ok(n) | Err(SysError::LengthNotEnough(n)) => n,
            Err(_) => return Err(2),
        };
        if len > MAX_WITNESS_BYTES {
            return Err(3);
        }
        let mut raw = vec![0; len];
        syscalls::load_witness(&mut raw, 0, 0, Source::Input).map_err(|_| 4)?;
        let witness = WitnessArgs::from_slice(&raw).map_err(|_| 5)?;
        let bytes = witness.input_type().to_opt().ok_or(5)?.raw_data();
        let proof = proof_bytes(&bytes)?;
        let script = load_script().map_err(|_| 6)?;
        let key = key_hex(&script.args().raw_data())?;
        let key = core::str::from_utf8(&key).map_err(|_| 6)?;
        sp1_verifier::Groth16Verifier::verify(proof, &journal, key, &sp1_verifier::GROTH16_VK_BYTES)
            .map_err(|_| 7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ambiguous_or_unbounded_proof_framing() {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);
        assert_eq!(proof_bytes(&bytes), Ok(&[1, 2, 3][..]));
        // Parser tests only: these bytes are not a cryptographic proof.
        for end in 0..bytes.len() {
            assert!(proof_bytes(&bytes[..end]).is_err());
        }
        bytes.push(4);
        assert!(proof_bytes(&bytes).is_err());
        bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(proof_bytes(&bytes).is_err());
        let mut oversized = MAGIC.to_vec();
        oversized.extend_from_slice(&((MAX_PROOF_BYTES + 1) as u32).to_le_bytes());
        oversized.resize(12 + MAX_PROOF_BYTES + 1, 0);
        assert!(proof_bytes(&oversized).is_err());
        assert_eq!(key_hex(&[0; 31]), Err(6));
        assert_eq!(key_hex(&[0; 33]), Err(6));
        let mut key = [0; 32];
        key[0] = 0xab;
        key[31] = 0xef;
        assert_eq!(
            core::str::from_utf8(&key_hex(&key).unwrap()).unwrap(),
            "0xab000000000000000000000000000000000000000000000000000000000000ef"
        );
    }
}
