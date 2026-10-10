//! Actual CKB 0.210.0 blocks, with deliberate corruptions for fail-closed tests.
use replay_native_vault::{number, recover, Cell, Limits, Tracker};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use tactus_o1_devnet_driver::rpc;

fn fixtures() -> (Vec<Value>, Value) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/evidence/native-vault-recovery/0.210.0");
    (
        serde_json::from_slice(&std::fs::read(base.join("canonical-blocks.json")).unwrap())
            .unwrap(),
        serde_json::from_slice(&std::fs::read(base.join("recovery.json")).unwrap()).unwrap(),
    )
}
fn tracker(report: &Value, limits: Limits) -> Tracker {
    Tracker::new(
        report["ckb_genesis"].as_str().unwrap(),
        &rpc::decode_hex(report["vault_script"].as_str().unwrap()).unwrap(),
        limits,
    )
    .unwrap()
}
fn cells(blocks: &[Value]) -> BTreeMap<String, Value> {
    blocks
        .iter()
        .flat_map(|b| b["transactions"].as_array().unwrap())
        .map(|tx| (tx["hash"].as_str().unwrap().to_owned(), tx.clone()))
        .collect()
}
fn scan(blocks: &[Value], reports: &Value, limits: Limits) -> Result<Value, String> {
    let txs = cells(blocks);
    let mut t = tracker(&reports["cold_final"], limits);
    for block in blocks {
        let result = t.apply(block, &mut |p| {
            let tx = txs
                .get(p["tx_hash"].as_str().ok_or("hash")?)
                .ok_or("parent")?;
            let i = number(&p["index"])? as usize;
            Ok(Cell {
                output: tx["outputs"][i].clone(),
                data: tx["outputs_data"][i].clone(),
            })
        });
        if let Err(e) = result {
            assert!(t.report().is_err(), "partial state must never escape");
            assert!(t
                .apply(block, &mut |_| panic!("poisoned tracker called resolver"))
                .is_err());
            return Err(e);
        }
    }
    t.report()
}
#[test]
fn actual_canonical_prefixes_recover_exactly_without_operator_state() {
    let (blocks, reports) = fixtures();
    for label in [
        "cold_genesis",
        "cold_after_deposit_0",
        "cold_after_deposit_1",
        "cold_final",
    ] {
        let mut expected = reports[label].clone();
        expected
            .as_object_mut()
            .unwrap()
            .remove("canonical_pin_rechecked");
        expected
            .as_object_mut()
            .unwrap()
            .remove("current_live_rechecked");
        let end = expected["pinned_height"].as_u64().unwrap() as usize;
        assert_eq!(
            scan(&blocks[..=end], &reports, Limits::default()).unwrap(),
            expected
        );
    }
}
fn flip(value: &mut Value, offset: usize) {
    let mut raw = rpc::decode_hex(value.as_str().unwrap()).unwrap();
    raw[offset] ^= 1;
    *value = json!(rpc::bytes_to_hex(&raw));
}
#[test]
fn malformed_or_incomplete_history_cannot_publish_partial_state() {
    let (base, reports) = fixtures();
    type Mutation = fn(&mut Vec<Value>);
    let cases: &[(&str, Mutation)] = &[
        ("wrong genesis block", |b| {
            flip(&mut b[0]["header"]["hash"], 0)
        }),
        ("disconnected canonical blocks", |b| {
            flip(&mut b[12]["header"]["parent_hash"], 0)
        }),
        ("scan height/limit", |b| {
            b[12]["header"]["number"] = json!("0xff")
        }),
        ("forged vault genesis", |b| {
            flip(
                &mut b[8]["transactions"][1]["inputs"][0]["previous_output"]["tx_hash"],
                0,
            )
        }),
        ("canonical predecessor", |b| {
            flip(
                &mut b[12]["transactions"][1]["inputs"][0]["previous_output"]["tx_hash"],
                0,
            )
        }),
        ("receipt without canonical", |b| {
            b[12]["transactions"][1]["outputs"][0]["type"] = Value::Null;
            flip(
                &mut b[12]["transactions"][1]["inputs"][0]["previous_output"]["tx_hash"],
                0,
            );
        }),
        ("receipt count", |b| {
            b[12]["transactions"][1]["outputs"][1]["type"] = Value::Null
        }),
        ("deposit transcript", |b| {
            flip(&mut b[12]["transactions"][1]["outputs_data"][1], 16)
        }),
        ("deposit transcript", |b| {
            flip(&mut b[16]["transactions"][1]["outputs_data"][1], 44)
        }),
        ("vault capacity", |b| {
            b[12]["transactions"][1]["outputs"][0]["capacity"] = json!("0x1")
        }),
        ("custody lock changed", |b| {
            flip(
                &mut b[12]["transactions"][1]["outputs"][0]["lock"]["args"],
                0,
            )
        }),
        ("deposit transcript", |b| {
            b[12]["transactions"][1]["outputs"][1]["lock"]["args"] = json!("0x00")
        }),
        ("immutable deposit receipt consumed", |b| {
            let hash = b[12]["transactions"][1]["hash"].clone();
            b[16]["transactions"][1]["inputs"][1]["previous_output"] =
                json!({"tx_hash":hash,"index":"0x1"});
        }),
        ("output/data count", |b| {
            b[12]["transactions"][1]["outputs_data"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
    ];
    for (error, mutate) in cases {
        let mut blocks = base.clone();
        mutate(&mut blocks);
        let got = scan(&blocks, &reports, Limits::default()).unwrap_err();
        assert!(got.contains(error), "expected {error}, got {got}");
    }
    assert!(scan(
        &base,
        &reports,
        Limits {
            blocks: 12,
            ..Limits::default()
        }
    )
    .unwrap_err()
    .contains("limit"));
    assert!(scan(
        &base,
        &reports,
        Limits {
            deposits: 1,
            ..Limits::default()
        }
    )
    .unwrap_err()
    .contains("record limit"));
    let r = &reports["cold_final"];
    let script = rpc::decode_hex(r["vault_script"].as_str().unwrap()).unwrap();
    assert!(Tracker::new(
        &format!("0x{}", "00".repeat(32)),
        &script,
        Limits::default()
    )
    .is_err());
    assert!(Tracker::new(
        r["ckb_genesis"].as_str().unwrap(),
        &script,
        Limits {
            blocks: 0,
            ..Limits::default()
        }
    )
    .is_err());
}

// A local fault-injection RPC, never a replacement for the actual-node evidence.
struct Server {
    address: String,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(&self.address);
        self.worker.take().unwrap().join().unwrap();
    }
}
fn server(fault: &'static str) -> Server {
    let (blocks, reports) = fixtures();
    let txs = cells(&blocks);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let worker = thread::spawn(move || {
        let mut hashes = 0;
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            if done.load(Ordering::SeqCst) {
                break;
            }
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut data = Vec::new();
            let request: Value = loop {
                let mut buf = [0; 4096];
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0);
                data.extend_from_slice(&buf[..n]);
                if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&data[..i]).unwrap();
                    let size: usize = headers
                        .lines()
                        .find_map(|s| s.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.len() >= i + 4 + size {
                        break serde_json::from_slice(&data[i + 4..i + 4 + size]).unwrap();
                    }
                }
            };
            let p = &request["params"];
            let result = match request["method"].as_str().unwrap() {
                "get_tip_header" => {
                    let mut h = blocks.last().unwrap()["header"].clone();
                    if fault == "pin" {
                        flip(&mut h["hash"], 0);
                    }
                    h
                }
                "get_block_hash" => {
                    hashes += 1;
                    if fault == "network" || (fault == "reorg" && hashes > 1) {
                        json!("0x00")
                    } else {
                        blocks[number(&p[0]).unwrap() as usize]["header"]["hash"].clone()
                    }
                }
                "get_block_by_number" => {
                    if fault == "missing block" {
                        Value::Null
                    } else {
                        blocks[number(&p[0]).unwrap() as usize].clone()
                    }
                }
                "get_transaction" => {
                    let mut t = txs[p[0].as_str().unwrap()].clone();
                    if fault == "parent hash" {
                        flip(&mut t["hash"], 0);
                    }
                    json!({"transaction": t, "tx_status":{"status": if fault == "parent pending" {"pending"} else {"committed"}}})
                }
                "get_live_cell" => {
                    let t = &txs[p[0]["tx_hash"].as_str().unwrap()];
                    let i = number(&p[0]["index"]).unwrap() as usize;
                    let is_vault = p[0] == reports["cold_final"]["point"];
                    let mut live = json!({"status":"live","cell":{"output":t["outputs"][i],"data":{"content":t["outputs_data"][i]}}});
                    if fault == "vault spent" && is_vault || fault == "receipt spent" && !is_vault {
                        live["status"] = json!("dead");
                    }
                    if fault == "vault data" && is_vault || fault == "receipt data" && !is_vault {
                        flip(&mut live["cell"]["data"]["content"], 0);
                    }
                    live
                }
                method => panic!("unexpected RPC {method}"),
            };
            let body = if fault == "RPC error" {
                json!({"id":1,"error":{"code":-1,"message":"outage"}})
            } else {
                json!({"id":1,"result":result})
            }
            .to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    Server {
        address,
        stop,
        worker: Some(worker),
    }
}
#[test]
fn rpc_recovery_rechecks_pin_live_custody_and_immutable_receipts() {
    let (_, reports) = fixtures();
    let r = &reports["cold_final"];
    for (fault, error) in [
        ("none", ""),
        ("network", "network mismatch"),
        ("pin", "pinned header"),
        ("reorg", "pin invalidated"),
        ("missing block", "missing number"),
        ("parent hash", "canonical dependency"),
        ("parent pending", "canonical dependency"),
        ("vault spent", "vault changed"),
        ("vault data", "vault changed"),
        ("receipt spent", "receipt unavailable"),
        ("receipt data", "receipt unavailable"),
        ("RPC error", "rpc error"),
    ] {
        let s = server(fault);
        let got = recover(
            &s.address,
            r["ckb_genesis"].as_str().unwrap(),
            &rpc::decode_hex(r["vault_script"].as_str().unwrap()).unwrap(),
            Limits::default(),
            |_| Ok(()),
        );
        if error.is_empty() {
            assert_eq!(got.unwrap(), *r);
        } else {
            let e = got.unwrap_err();
            assert!(e.contains(error), "{fault}: {e}");
        }
    }
}

#[test]
fn cli_publishes_capture_only_on_success_and_never_overwrites() {
    let (_, reports) = fixtures();
    let r = &reports["cold_final"];
    let dir =
        std::env::temp_dir().join(format!("tactus-vault-recovery-test-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("blocks.json");
    for fault in ["reorg", "none", "none"] {
        let s = server(fault);
        let existed = path.exists();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_recover-native-vault"))
            .args([
                r["ckb_genesis"].as_str().unwrap(),
                r["vault_script"].as_str().unwrap(),
            ])
            .env("TACTUS_CKB_RPC_ADDR", &s.address)
            .env("TACTUS_VAULT_CAPTURE", &path)
            .env_remove("TACTUS_VAULT_MAX_BLOCKS")
            .env_remove("TACTUS_VAULT_MAX_DEPOSITS")
            .output()
            .unwrap();
        assert_eq!(out.status.success(), fault == "none" && !existed);
        assert!(!path.with_extension("pending").exists());
        if out.status.success() {
            assert_eq!(serde_json::from_slice::<Value>(&out.stdout).unwrap(), *r);
        } else {
            assert!(out.stdout.is_empty());
        }
        if path.exists() {
            assert_eq!(
                serde_json::from_slice::<Vec<Value>>(&std::fs::read(&path).unwrap()).unwrap(),
                fixtures().0
            );
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}
