//! Transaction construction and signing for the devnet replay.
//!
//! All amounts are shannons (1 CKB = 1e8). Transactions are protocol version
//! 0, signed with the dev key's SECP256K1 lock; the first input's witness
//! carries both the recoverable signature (lock group) and the OrderingHead
//! commitment (type group, `WitnessArgs::input_type`).

use secp256k1::{ecdsa::RecoverableSignature, Message, PublicKey, Secp256k1, SecretKey};
use serde_json::{json, Value};

use crate::molecule;
use crate::rpc;
use tactus_ordering_script::{ckb_blake2b, ckb_blakeb160, OrderingHead};

/// SECP256K1/blake160 sighash-all script hash (hash type `type`).
pub const SECP_CODE_HASH: [u8; 32] = [
    0x9b, 0xd7, 0xe0, 0x6f, 0x3e, 0xcf, 0x4b, 0xe0, 0xf2, 0xfc, 0xd2, 0x18, 0x8b, 0x23, 0xf1, 0xb9,
    0xfc, 0xc8, 0x8e, 0x5d, 0x4b, 0x65, 0xa8, 0x63, 0x7b, 0x17, 0x72, 0x3b, 0xbd, 0xa3, 0xcc, 0xe8,
];

pub const SHANNONS_PER_CKB: u64 = 100_000_000;
/// One CKB fee per transaction — devnet tier.
pub const TX_FEE: u64 = SHANNONS_PER_CKB;

#[derive(Clone)]
pub struct DevKey {
    pub secret: SecretKey,
    pub public: PublicKey,
    pub args: [u8; 20],
}

impl DevKey {
    /// The fixed devnet-only key (0x21 × 32). Never use outside dev chains.
    #[must_use]
    pub fn dev() -> Self {
        let secp = Secp256k1::new();
        let secret = SecretKey::from_slice(&[0x21u8; 32]).expect("fixed dev key is valid");
        let public = PublicKey::from_secret_key(&secp, &secret);
        let args = ckb_blakeb160(&public.serialize());
        Self {
            secret,
            public,
            args,
        }
    }

    #[must_use]
    pub fn lock_script(&self) -> Vec<u8> {
        molecule::script(&SECP_CODE_HASH, 1, &self.args)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellOutPoint {
    pub tx_hash: [u8; 32],
    pub index: u32,
}

/// Locates the SECP code cell in the genesis block: the cell whose **type
/// script hash** (ckbhash of the serialized type script, RFC 0022) equals
/// [`SECP_CODE_HASH`]. Verified against this devnet's genesis.
pub fn find_secp_dep(genesis: &Value) -> Result<CellOutPoint, String> {
    let tx0 = genesis
        .get("transactions")
        .and_then(|t| t.get(0))
        .ok_or("genesis has no transactions")?;
    let tx_hash_hex = tx0
        .get("hash")
        .and_then(Value::as_str)
        .ok_or("no tx hash")?;
    let tx_hash = rpc::hex_to_bytes(tx_hash_hex);
    let outputs = genesis
        .get("transactions")
        .and_then(|t| t.get(0))
        .and_then(|t| t.get("outputs"))
        .and_then(Value::as_array)
        .ok_or("no outputs")?;
    for (index, out) in outputs.iter().enumerate() {
        let Some(t) = out.get("type") else { continue };
        let code_hash_hex = t
            .get("code_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let ch = rpc::hex_to_bytes(code_hash_hex);
        if ch.len() != 32 {
            continue;
        }
        let mut code_hash = [0u8; 32];
        code_hash.copy_from_slice(&ch);
        let hash_type = match t.get("hash_type").and_then(Value::as_str) {
            Some("type") => 1u8,
            _ => 0u8,
        };
        let args = rpc::hex_to_bytes(t.get("args").and_then(Value::as_str).unwrap_or("0x"));
        let script = molecule::script(&code_hash, hash_type, &args);
        if ckb_blake2b(&script) == SECP_CODE_HASH {
            let mut h = [0u8; 32];
            h.copy_from_slice(&tx_hash);
            return Ok(CellOutPoint {
                tx_hash: h,
                index: index as u32,
            });
        }
    }
    Err("secp code cell not found in genesis".into())
}

/// Collects mature cellbase outputs paying to `args` (devnet maturity is 0).
pub fn collect_coinbase(
    tip: u64,
    args: &[u8; 20],
    max_blocks: u64,
) -> Result<Vec<(CellOutPoint, u64)>, String> {
    let args_hex = rpc::bytes_to_hex(args);
    let mut cells = Vec::new();
    let start = tip.saturating_sub(max_blocks) + 1;
    for block_number in start..=tip {
        let block = rpc::get_block_detailed(block_number)?;
        let Some(cellbase) = block.get("transactions").and_then(|t| t.get(0)).cloned() else {
            continue;
        };
        let tx_hash_hex = cellbase
            .get("hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let outputs = cellbase
            .get("outputs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (index, out) in outputs.iter().enumerate() {
            let lock_args = out
                .get("lock")
                .and_then(|l| l.get("args"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if lock_args.eq_ignore_ascii_case(&args_hex) {
                let capacity = u64::from_str_radix(
                    out.get("capacity")
                        .and_then(Value::as_str)
                        .unwrap_or("0x0")
                        .trim_start_matches("0x"),
                    16,
                )
                .unwrap_or(0);
                let tx_hash = rpc::hex_to_bytes(tx_hash_hex);
                let mut h = [0u8; 32];
                h.copy_from_slice(&tx_hash);
                cells.push((
                    CellOutPoint {
                        tx_hash: h,
                        index: index as u32,
                    },
                    capacity,
                ));
            }
        }
    }
    Ok(cells)
}

/// An output being built.
pub struct OutSpec {
    pub capacity: u64,
    pub lock: Vec<u8>,
    pub type_script: Option<Vec<u8>>,
    pub data: Vec<u8>,
}

impl OutSpec {
    /// Minimum capacity covering the occupied storage (bytes × 1 CKB).
    #[must_use]
    pub fn required_capacity(lock: &[u8], type_script: Option<&[u8]>, data_len: usize) -> u64 {
        let serialized = molecule::cell_output(0, lock, type_script).len() + data_len;
        serialized as u64 * SHANNONS_PER_CKB
    }
}

/// Builds, signs and serializes one transaction. The first input's witness
/// carries `WitnessArgs { lock: signature, input_type }`; remaining inputs
/// get empty witnesses.
pub fn build_and_sign(
    key: &DevKey,
    secp_dep: &CellOutPoint,
    inputs: &[(CellOutPoint, u64)],
    outputs: &[OutSpec],
    input_type: Option<&[u8]>,
) -> Result<(Vec<u8>, Value), String> {
    let dep = molecule::cell_dep(&molecule::out_point(&secp_dep.tx_hash, secp_dep.index), 0);
    let input_cells: Vec<Vec<u8>> = inputs
        .iter()
        .map(|(o, _)| molecule::cell_input(0, &molecule::out_point(&o.tx_hash, o.index)))
        .collect();
    let out_cells: Vec<Vec<u8>> = outputs
        .iter()
        .map(|o| molecule::cell_output(o.capacity, &o.lock, o.type_script.as_deref()))
        .collect();
    let out_data: Vec<Vec<u8>> = outputs.iter().map(|o| molecule::bytes(&o.data)).collect();

    let raw = molecule::raw_transaction(&[dep], &input_cells, &out_cells, &out_data);
    let tx_hash = ckb_blake2b(&raw);

    // SECP256K1/blake160 sighash-all message (per the system script source):
    // ckbhash( tx_hash ‖ u64le(len)‖blank_witness0 ‖ Σ u64le(len)‖w_i ),
    // where blank_witness0 keeps input_type/output_type but zeroes the
    // 65-byte lock; remaining same-group witnesses hash as submitted.
    let blank0 = molecule::witness_args(Some(&[0u8; 65]), input_type, None);
    let mut message_buf = tx_hash.to_vec();
    message_buf.extend_from_slice(&(blank0.len() as u64).to_le_bytes());
    message_buf.extend_from_slice(&blank0);
    for _ in 1..inputs.len() {
        let empty = molecule::witness_args(None, None, None);
        message_buf.extend_from_slice(&(empty.len() as u64).to_le_bytes());
        message_buf.extend_from_slice(&empty);
    }
    let message_hash = ckb_blake2b(&message_buf);

    let secp = Secp256k1::new();
    let message = Message::from_digest_slice(&message_hash).map_err(|e| e.to_string())?;
    let sig: RecoverableSignature = secp.sign_ecdsa_recoverable(&message, &key.secret);
    let (rec_id, data) = sig.serialize_compact();
    let mut signature = Vec::with_capacity(65);
    signature.extend_from_slice(&data);
    signature.push(rec_id.to_i32() as u8);

    let first = molecule::witness_args(Some(&signature), input_type, None);
    let mut witnesses = vec![first];
    for _ in 1..inputs.len() {
        witnesses.push(molecule::witness_args(None, None, None));
    }

    let bytes = molecule::transaction(&raw, &witnesses);
    let json = transaction_to_json(outputs, inputs, secp_dep, &witnesses);
    Ok((bytes, json))
}

/// JSON form for RPC submission (ckb 0.210 requires the object form). The
/// binary form remains authoritative for the tx hash and signature.
fn transaction_to_json(
    outputs: &[OutSpec],
    inputs: &[(CellOutPoint, u64)],
    secp_dep: &CellOutPoint,
    witnesses: &[Vec<u8>],
) -> Value {
    let out_json: Vec<Value> = outputs
        .iter()
        .map(|o| {
            let mut v = json!({
                "capacity": format!("0x{:x}", o.capacity),
                "lock": molecule::script_to_json(&o.lock),
            });
            if let Some(t) = &o.type_script {
                v["type"] = molecule::script_to_json(t);
            }
            v
        })
        .collect();
    json!({
        "version": "0x0",
        "cell_deps": [{
            "out_point": {
                "tx_hash": rpc::bytes_to_hex(&secp_dep.tx_hash),
                "index": format!("0x{:x}", secp_dep.index),
            },
            "dep_type": "code",
        }],
        "header_deps": [],
        "inputs": inputs.iter().map(|(o, _)| json!({
            "previous_output": {
                "tx_hash": rpc::bytes_to_hex(&o.tx_hash),
                "index": format!("0x{:x}", o.index),
            },
            "since": "0x0",
        })).collect::<Vec<_>>(),
        "outputs": out_json,
        "outputs_data": outputs.iter()
            .map(|o| rpc::bytes_to_hex(&o.data))
            .collect::<Vec<_>>(),
        "witnesses": witnesses.iter()
            .map(|w| rpc::bytes_to_hex(w))
            .collect::<Vec<_>>(),
    })
}

/// Tactus type script referencing the deployed ELF by data hash.
#[must_use]
pub fn tactus_type_script(elf: &[u8], rollup_id: &[u8; 32]) -> Vec<u8> {
    molecule::script(&ckb_blake2b(elf), 0, rollup_id)
}

/// Builds the successor head cell spec (same lock and type, new state).
#[must_use]
pub fn head_output(
    key: &DevKey,
    type_script: &[u8],
    next: &OrderingHead,
    capacity: u64,
) -> OutSpec {
    OutSpec {
        capacity,
        lock: key.lock_script(),
        type_script: Some(type_script.to_vec()),
        data: next.to_bytes().to_vec(),
    }
}

/// Sends a transaction (JSON form) and waits until it is committed.
pub fn send_and_wait(tx: &Value, timeout_secs: u64) -> Result<(String, Option<u64>), String> {
    let tx_hash = rpc::send_transaction_json(tx)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let (status, block_number) = rpc::get_transaction_status(&tx_hash)?;
        if status == "committed" {
            return Ok((tx_hash, block_number));
        }
        if std::time::Instant::now() > deadline {
            return Err(format!("timeout in status {status}: {tx_hash}"));
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
