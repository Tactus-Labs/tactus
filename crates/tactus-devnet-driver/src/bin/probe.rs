//! Devnet diagnostics: local re-verification of a signed spend and an
//! on-chain message-rule probe. Current status (2026-10-09, ckb v0.210.0
//! devnet): even ckb-cli-signed transactions are rejected with secp error
//! -31 (pubkey/args mismatch) on this chain, so the blocker is environmental,
//! not in this driver. See specs/EXPERIMENT_A_DEVNET_REPORT.md.
// Message-rule enumeration with spendable cells — the chain judges.
fn main() {
    use secp256k1::{ecdsa, Message, Secp256k1};
    use tactus_devnet_driver::molecule;
    use tactus_devnet_driver::rpc;
    use tactus_devnet_driver::tx::{self, DevKey, OutSpec};
    use tactus_ordering_script::ckb_blake2b;

    let key = DevKey::dev();
    let secp = Secp256k1::new();
    let genesis = rpc::get_block_detailed(0).unwrap();
    let secp_dep = tx::find_secp_dep(&genesis).unwrap();
    let tip = rpc::get_tip_block_number().unwrap();
    let cells: Vec<_> = tx::collect_coinbase(tip, &key.args, 300)
        .unwrap()
        .into_iter()
        .take(2)
        .collect();
    let total: u64 = cells.iter().map(|(_, c)| c).sum();
    let outputs = vec![OutSpec {
        capacity: total - tx::TX_FEE,
        lock: key.lock_script(),
        type_script: None,
        data: vec![],
    }];
    let (bytes, mut json) = tx::build_and_sign(&key, &secp_dep, &cells, &outputs, None).unwrap();
    let off_w = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let raw = &bytes[12..off_w];
    let tx_hash = ckb_blake2b(raw);
    let witnesses: Vec<Vec<u8>> = json["witnesses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| rpc::hex_to_bytes(w.as_str().unwrap()))
        .collect();
    let hdr = u32::from_le_bytes(witnesses[0][4..8].try_into().unwrap()) as usize;
    let llen = u32::from_le_bytes(witnesses[0][hdr..hdr + 4].try_into().unwrap()) as usize;
    let mut blank0 = witnesses[0].clone();
    for b in &mut blank0[hdr + 4..hdr + 4 + llen] {
        *b = 0;
    }
    let empties = &witnesses[1..];
    let push = |buf: &mut Vec<u8>, w: &[u8]| {
        buf.extend_from_slice(&(w.len() as u64).to_le_bytes());
        buf.extend_from_slice(w);
    };

    let variants: Vec<(&str, Vec<u8>)> = vec![
        ("A C-source rule", {
            let mut b = tx_hash.to_vec();
            push(&mut b, &blank0);
            for w in empties {
                push(&mut b, w);
            }
            b
        }),
        ("B blank0 only", {
            let mut b = tx_hash.to_vec();
            push(&mut b, &blank0);
            b
        }),
        ("C raw tx hash only", tx_hash.to_vec()),
        ("D witness0 zeroed-fully", {
            let e = molecule::witness_args(None, None, None);
            let mut b = tx_hash.to_vec();
            push(&mut b, &e);
            for w in empties {
                push(&mut b, w);
            }
            b
        }),
        ("E hash full tx (raw+wit)", ckb_blake2b(&bytes).to_vec()),
        ("F hash raw ‖ witnesses (no lens)", {
            let mut b = raw.to_vec();
            b.extend_from_slice(&blank0);
            for w in empties {
                b.extend_from_slice(w);
            }
            b
        }),
        ("G hash tx ‖ blank0 (no lens)", {
            let mut b = tx_hash.to_vec();
            b.extend_from_slice(&blank0);
            b
        }),
        ("H hash tx ‖ raw ‖ blank0", {
            let mut b = tx_hash.to_vec();
            b.extend_from_slice(raw);
            push(&mut b, &blank0);
            for w in empties {
                push(&mut b, w);
            }
            b
        }),
    ];

    for (name, buf) in &variants {
        let m = ckb_blake2b(buf);
        let sig: ecdsa::RecoverableSignature =
            secp.sign_ecdsa_recoverable(&Message::from_digest_slice(&m).unwrap(), &key.secret);
        let (rid, data) = sig.serialize_compact();
        let mut s65 = data.to_vec();
        s65.push(rid.to_i32() as u8);
        let w0 = molecule::witness_args(Some(&s65), None, None);
        json["witnesses"][0] = serde_json::Value::String(rpc::bytes_to_hex(&w0));
        match rpc::send_transaction_json(&json) {
            Ok(h) => {
                println!("{name}: ACCEPTED {h}");
                break;
            }
            Err(e) => println!("{name}: no ({})", e.contains("-101")),
        }
    }
}
