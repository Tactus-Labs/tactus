//! Hand-rolled CKB molecule serialization (RFC 0008) for the transaction
//! shapes the driver needs. Layout rules implemented here:
//!
//! - fixed structs: raw little-endian concatenation, no header;
//! - fixed vectors (`FixVec`): `u32` count + raw items;
//! - dynamic vectors and tables (`DynVec`/`Table`): `u32` total size,
//!   `u32` field/item count, `u32` offsets, then fields — an absent field
//!   occupies zero bytes (identical adjacent offsets).

fn u32_le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

fn u64_le(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

fn dyn_collection(items: &[Vec<u8>]) -> Vec<u8> {
    let header_len = 8 + 4 * items.len();
    let payload: usize = items.iter().map(|i| i.len()).sum();
    let total = header_len + payload;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&u32_le(total as u32));
    out.extend_from_slice(&u32_le(items.len() as u32));
    let mut offset = header_len as u32;
    for item in items {
        out.extend_from_slice(&u32_le(offset));
        offset += item.len() as u32;
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// Molecule `Table` over optional fields: `None` serializes as empty.
fn table(fields: &[Option<Vec<u8>>]) -> Vec<u8> {
    dyn_collection(
        &fields
            .iter()
            .map(|f| f.clone().unwrap_or_default())
            .collect::<Vec<_>>(),
    )
}

/// `Bytes`: u32 length + payload.
pub fn bytes(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&u32_le(payload.len() as u32));
    out.extend_from_slice(payload);
    out
}

/// `Script` table: code hash (32 raw), hash type (1 raw), args (`Bytes`).
#[must_use]
pub fn script(code_hash: &[u8; 32], hash_type: u8, args: &[u8]) -> Vec<u8> {
    let mut code = Vec::with_capacity(32);
    code.extend_from_slice(code_hash);
    table(&[Some(code), Some(vec![hash_type]), Some(bytes(args))])
}

/// `CellOutput` table: capacity (u64 raw), lock `Script`, optional type `Script`.
#[must_use]
pub fn cell_output(capacity: u64, lock: &[u8], type_script: Option<&[u8]>) -> Vec<u8> {
    table(&[
        Some(u64_le(capacity).to_vec()),
        Some(lock.to_vec()),
        type_script.map(|s| s.to_vec()),
    ])
}

/// `OutPoint` fixed struct: tx hash (32) + index (u32).
#[must_use]
pub fn out_point(tx_hash: &[u8; 32], index: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(36);
    out.extend_from_slice(tx_hash);
    out.extend_from_slice(&u32_le(index));
    out
}

/// `CellInput` fixed struct: since (u64) + out point (36).
#[must_use]
pub fn cell_input(since: u64, previous_output: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(44);
    out.extend_from_slice(&u64_le(since));
    out.extend_from_slice(previous_output);
    out
}

/// `CellDep` fixed struct: out point (36) + dep type (1).
#[must_use]
pub fn cell_dep(out_point: &[u8], dep_type: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(37);
    out.extend_from_slice(out_point);
    out.push(dep_type);
    out
}

/// `WitnessArgs` table: optional lock / input_type / output_type `Bytes`.
#[must_use]
pub fn witness_args(
    lock: Option<&[u8]>,
    input_type: Option<&[u8]>,
    output_type: Option<&[u8]>,
) -> Vec<u8> {
    table(&[
        lock.map(bytes),
        input_type.map(bytes),
        output_type.map(bytes),
    ])
}

fn fix_vec(item_size: usize, raw_items: &[u8]) -> Vec<u8> {
    debug_assert_eq!(raw_items.len() % item_size, 0);
    let mut out = Vec::with_capacity(4 + raw_items.len());
    out.extend_from_slice(&u32_le((raw_items.len() / item_size) as u32));
    out.extend_from_slice(raw_items);
    out
}

/// `RawTransaction` table, protocol version 0.
#[must_use]
pub fn raw_transaction(
    cell_deps: &[Vec<u8>],    // serialized CellDep (37 bytes each)
    inputs: &[Vec<u8>],       // serialized CellInput (44 bytes each)
    outputs: &[Vec<u8>],      // serialized CellOutput tables
    outputs_data: &[Vec<u8>], // serialized Bytes
) -> Vec<u8> {
    let deps_raw: Vec<u8> = cell_deps.concat();
    let inputs_raw: Vec<u8> = inputs.concat();
    table(&[
        Some(u32_le(0).to_vec()),           // version
        Some(fix_vec(37, &deps_raw)),       // cell_deps
        Some(fix_vec(32, &[])),             // header_deps
        Some(fix_vec(44, &inputs_raw)),     // inputs
        Some(dyn_collection(outputs)),      // outputs
        Some(dyn_collection(outputs_data)), // outputs_data
    ])
}

/// `Transaction` table: raw + witnesses (`DynVec<Bytes>`).
#[must_use]
pub fn transaction(raw: &[u8], witnesses: &[Vec<u8>]) -> Vec<u8> {
    table(&[Some(raw.to_vec()), Some(dyn_collection(witnesses))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_script_roundtrips_shape() {
        // Table header: total = 8 + 3 offsets + fields.
        let s = script(&[1u8; 32], 1, &[]);
        let total = u32::from_le_bytes(s[0..4].try_into().unwrap()) as usize;
        assert_eq!(total, s.len());
        let count = u32::from_le_bytes(s[4..8].try_into().unwrap());
        assert_eq!(count, 3);
        // args is an empty Bytes: u32(0)
        let args_off = u32::from_le_bytes(s[16..20].try_into().unwrap()) as usize;
        assert_eq!(&s[args_off..args_off + 4], &0u32.to_le_bytes());
    }

    #[test]
    fn cell_input_layout_is_44_bytes() {
        let i = cell_input(0, &out_point(&[7u8; 32], 3));
        assert_eq!(i.len(), 44);
    }

    #[test]
    fn witness_args_offsets_are_monotonic() {
        let w = witness_args(Some(&[0u8; 65]), Some(&[2u8; 32]), None);
        let total = u32::from_le_bytes(w[0..4].try_into().unwrap()) as usize;
        assert_eq!(total, w.len());
        let o1 = u32::from_le_bytes(w[8..12].try_into().unwrap());
        let o2 = u32::from_le_bytes(w[12..16].try_into().unwrap());
        let o3 = u32::from_le_bytes(w[16..20].try_into().unwrap());
        // Third field absent: its offset equals the table's total size.
        assert!(o1 >= 20 && o2 > o1 && o3 as usize == total);
    }
}
