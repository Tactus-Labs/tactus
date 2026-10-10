use super::*;
use alloc::vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{
        packed::{Script, WitnessArgs},
        prelude::*,
    },
    error::SysError,
    high_level::*,
    syscalls,
};
ckb_std::default_alloc!();
ckb_std::entry!(entrypoint);
fn entrypoint() -> i8 {
    run().map_or_else(|e| e, |()| 0)
}
const HEAD_LOCK: [u8; 32] = [
    0x1a, 0x79, 0xae, 0x4b, 0x82, 0xf5, 0x58, 0x8e, 0x07, 0xfc, 0x0b, 0x94, 0xa0, 0xe8, 0xfb, 0xcf,
    0x61, 0xf1, 0xad, 0xc0, 0x6d, 0x05, 0xd7, 0x53, 0xd1, 0x3e, 0x2e, 0x63, 0xee, 0xc7, 0x6d, 0xee,
];
fn data(index: usize, source: Source, limit: usize) -> Result<Vec<u8>, i8> {
    let size = match syscalls::load_cell_data(&mut [], 0, index, source) {
        Ok(n) | Err(SysError::LengthNotEnough(n)) => n,
        _ => return Err(3),
    };
    if size > limit {
        return Err(3);
    }
    let mut out = vec![0; size];
    if syscalls::load_cell_data(&mut out, 0, index, source) != Ok(size) {
        return Err(3);
    }
    Ok(out)
}
fn unique(identity: [u8; 32], source: Source) -> Result<usize, i8> {
    let mut found = None;
    for i in 0..=MAX_CELLS {
        match load_cell_type_hash(i, source) {
            Err(SysError::IndexOutOfBound) => return found.ok_or(2),
            Ok(value) if i < MAX_CELLS => {
                if value == Some(identity) {
                    if found.is_some() {
                        return Err(2);
                    }
                    found = Some(i);
                }
            }
            _ => return Err(2),
        }
    }
    Err(2)
}
fn no_second(source: Source) -> Result<(), i8> {
    if load_cell_capacity(1, source) != Err(SysError::IndexOutOfBound) {
        return Err(2);
    }
    Ok(())
}

// One custody transition per transaction prevents two vaults from counting the
// same recipient output twice. Funding inputs may use any lock but no type.
fn only_typed_input(identity: [u8; 32]) -> Result<(), i8> {
    let mut count = 0;
    for index in 0..=MAX_CELLS {
        match load_cell_type_hash(index, Source::Input) {
            Err(SysError::IndexOutOfBound) => return if count == 1 { Ok(()) } else { Err(2) },
            Ok(None) if index < MAX_CELLS => {}
            Ok(Some(value)) if index < MAX_CELLS && value == identity => {
                count += 1;
            }
            _ => return Err(2),
        }
    }
    Err(2)
}
fn script_hash(script: &Script) -> [u8; 32] {
    hash(b"", script.as_slice())
}
fn receipt_script(script: &Script, vault: [u8; 32]) -> Script {
    script
        .clone()
        .as_builder()
        .args([b"TO1REC01".as_slice(), &vault].concat().pack())
        .build()
}
fn immutable(script: &Script) -> Script {
    script
        .clone()
        .as_builder()
        .args(Vec::<u8>::new().pack())
        .build()
}
fn read_state(index: usize, source: Source) -> Result<State, i8> {
    let s = State::decode(&data(index, source, STATE_BYTES)?)?;
    if load_cell_capacity(index, source).map_err(|_| 4)? != s.capacity()? {
        return Err(4);
    }
    Ok(s)
}
fn funded_deposit(cfg: &Config, current: &State, next: &State, record: &Record) -> Result<(), i8> {
    let (expected, expected_record) = current.deposit(cfg, record.recipient, record.amount)?;
    if *next != expected || *record != expected_record {
        return Err(5);
    }
    Ok(())
}
fn record_creation(script: &Script, args: &[u8]) -> Result<(), i8> {
    no_second(Source::GroupOutput)?;
    if load_cell_capacity(0, Source::GroupInput) != Err(SysError::IndexOutOfBound) {
        return Err(2);
    }
    let identity: [u8; 32] = args[8..40].try_into().unwrap();
    let before = unique(identity, Source::Input)?;
    let after = unique(identity, Source::Output)?;
    let vault = load_cell_type(before, Source::Input)
        .map_err(|_| 2)?
        .ok_or(2)?;
    if load_cell_type(after, Source::Output).map_err(|_| 2)? != Some(vault.clone())
        || vault.code_hash() != script.code_hash()
        || vault.hash_type() != script.hash_type()
    {
        return Err(2);
    }
    let cfg = Config::decode(&vault.args().raw_data())?;
    let current = read_state(before, Source::Input)?;
    let next = read_state(after, Source::Output)?;
    let record = Record::decode(&data(0, Source::GroupOutput, RECORD_BYTES)?)?;
    if load_cell_lock(0, Source::GroupOutput).map_err(|_| 5)? != immutable(script) {
        return Err(5);
    }
    funded_deposit(&cfg, &current, &next, &record)
}
fn release_witness() -> Result<Release, i8> {
    let limit = RELEASE_PREFIX + tactus_o1_state_proof_script::MAX_PROOF_BYTES + 4096;
    let size = match syscalls::load_witness(&mut [], 0, 0, Source::GroupInput) {
        Ok(n) | Err(SysError::LengthNotEnough(n)) => n,
        _ => return Err(6),
    };
    if size > limit {
        return Err(6);
    }
    let mut out = vec![0; size];
    if syscalls::load_witness(&mut out, 0, 0, Source::GroupInput) != Ok(size) {
        return Err(6);
    }
    let witness = WitnessArgs::from_slice(&out).map_err(|_| 6)?;
    Release::decode(&witness.input_type().to_opt().ok_or(6)?.raw_data())
}
fn run() -> Result<(), i8> {
    let script = load_script().map_err(|_| 1)?;
    if script.hash_type() != 2u8.into() {
        return Err(1);
    }
    let args = script.args().raw_data();
    if args.len() == 40 && &args[..8] == b"TO1REC01" {
        return record_creation(&script, &args);
    }
    let cfg = Config::decode(&args)?;
    let identity = load_script_hash().map_err(|_| 1)?;
    no_second(Source::GroupInput)?;
    no_second(Source::GroupOutput)?;
    let next = read_state(0, Source::GroupOutput)?;
    let expected_lock = Script::new_builder()
        .code_hash(HEAD_LOCK.pack())
        .hash_type(2u8.into())
        .args(identity.to_vec().pack())
        .build();
    if load_cell_lock(0, Source::GroupOutput).map_err(|_| 4)? != expected_lock {
        return Err(4);
    }
    if load_cell_capacity(0, Source::GroupInput) == Err(SysError::IndexOutOfBound) {
        let index = unique(identity, Source::Output)?;
        let input = load_input(0, Source::Input).map_err(|_| 2)?;
        if hash(
            b"",
            &[input.as_slice(), &(index as u64).to_le_bytes()].concat(),
        ) != cfg.identity
        {
            return Err(2);
        }
        let reserve = load_cell_occupied_capacity(0, Source::GroupOutput).map_err(|_| 4)?;
        if next != State::genesis(&cfg, reserve) {
            return Err(4);
        }
        return Ok(());
    }
    if load_cell_lock(0, Source::GroupInput).map_err(|_| 4)? != expected_lock {
        return Err(4);
    }
    only_typed_input(identity)?;
    let current = read_state(0, Source::GroupInput)?;
    if next.count > current.count {
        let receipt = receipt_script(&script, identity);
        let index = unique(script_hash(&receipt), Source::Output)?;
        if load_cell_lock(index, Source::Output).map_err(|_| 5)? != immutable(&script) {
            return Err(5);
        }
        let record = Record::decode(&data(index, Source::Output, RECORD_BYTES)?)?;
        return funded_deposit(&cfg, &current, &next, &record);
    }
    let release = release_witness()?;
    let tip = unique(cfg.settlement, Source::CellDep).map_err(|_| 9)?;
    let expected = release.verify(
        &cfg,
        &current,
        &data(tip, Source::CellDep, 280).map_err(|_| 9)?,
    )?;
    if next != expected {
        return Err(7);
    }
    let payout = release.payout as usize;
    if payout >= MAX_CELLS
        || load_cell_lock_hash(payout, Source::Output).map_err(|_| 11)? != release.recipient
        || load_cell_type_hash(payout, Source::Output)
            .map_err(|_| 11)?
            .is_some()
        || !data(payout, Source::Output, 0).map_err(|_| 11)?.is_empty()
        || load_cell_capacity(payout, Source::Output).map_err(|_| 11)? < release.amount
    {
        return Err(11);
    }
    Ok(())
}
