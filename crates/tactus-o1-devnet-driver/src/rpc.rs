//! Minimal JSON-RPC 2.0 client over plain HTTP for a local CKB devnet node
//! (`http://127.0.0.1:8114`). serde_json is used untyped: the driver reads a
//! handful of well-known fields and hashes everything itself.

use std::io::{Read, Write};
use std::net::TcpStream;

use serde_json::{json, Value};

pub const CKB_RPC_URL: &str = "127.0.0.1:8114";

/// One JSON-RPC call. Returns the `result` field or the error text.
pub fn call(method: &str, params: Value) -> Result<Value, String> {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let body = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let request = format!(
        "POST / HTTP/1.1\r\nHost: {CKB_RPC_URL}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream = TcpStream::connect(CKB_RPC_URL).map_err(|e| format!("connect: {e}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("read: {e}"))?;
    let body_start = response.find("\r\n\r\n").ok_or("malformed http response")? + 4;
    let parsed: Value =
        serde_json::from_str(&response[body_start..]).map_err(|e| format!("json: {e}"))?;
    if let Some(err) = parsed.get("error") {
        return Err(format!("rpc error: {err}"));
    }
    parsed
        .get("result")
        .cloned()
        .ok_or_else(|| "no result".to_string())
}

pub fn get_tip_block_number() -> Result<u64, String> {
    let v = call("get_tip_block_number", json!([]))?;
    let h = v.as_str().ok_or("tip not a string")?;
    u64::from_str_radix(h.trim_start_matches("0x"), 16).map_err(|e| e.to_string())
}

/// Detailed block (verbosity 2) — transactions carry outputs and data.
pub fn get_block_detailed(number: u64) -> Result<Value, String> {
    call(
        "get_block_by_number",
        json!([format!("0x{number:x}"), "0x2"]),
    )
}

pub fn send_transaction_json(tx: &Value) -> Result<String, String> {
    call("send_transaction", json!([tx, "passthrough"]))
        .map(|v| v.as_str().unwrap_or_default().to_string())
}

/// `(committed, block_number)` from `get_transaction`.
pub fn get_transaction_status(tx_hash: &str) -> Result<(String, Option<u64>), String> {
    let v = call("get_transaction", json!([tx_hash]))?;
    let status = v
        .get("tx_status")
        .and_then(|s| s.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let bn = v
        .get("tx_status")
        .and_then(|s| s.get("block_number"))
        .and_then(Value::as_str)
        .and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok());
    Ok((status, bn))
}

pub fn hex_to_bytes(h: &str) -> Vec<u8> {
    let h = h.trim_start_matches("0x");
    (0..h.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&h[i..i + 2], 16).ok())
        .collect()
}

pub fn bytes_to_hex(b: &[u8]) -> String {
    format!(
        "0x{}",
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    )
}
