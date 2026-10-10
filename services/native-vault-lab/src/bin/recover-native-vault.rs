use replay_native_vault::{recover, Limits};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
use tactus_o1_devnet_driver::rpc;
struct Capture {
    path: PathBuf,
    temporary: PathBuf,
    file: std::fs::File,
    first: bool,
    done: bool,
}
impl Capture {
    fn new(path: PathBuf) -> Result<Self, String> {
        if path.exists() {
            return Err("capture already exists".into());
        }
        let temporary = path.with_extension("pending");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(b"[\n").map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            temporary,
            file,
            first: true,
            done: false,
        })
    }
    fn push(&mut self, value: &serde_json::Value) -> Result<(), String> {
        if !self.first {
            self.file.write_all(b",\n").map_err(|e| e.to_string())?
        }
        self.first = false;
        serde_json::to_writer(&mut self.file, value).map_err(|e| e.to_string())
    }
    fn commit(&mut self) -> Result<(), String> {
        self.file.write_all(b"\n]\n").map_err(|e| e.to_string())?;
        self.file.sync_all().map_err(|e| e.to_string())?;
        fs::hard_link(&self.temporary, &self.path).map_err(|e| e.to_string())?;
        fs::remove_file(&self.temporary).map_err(|e| e.to_string())?;
        self.done = true;
        Ok(())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if !self.done {
            let _ = fs::remove_file(&self.temporary);
        }
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: recover-native-vault EXPECTED_GENESIS VAULT_SCRIPT_HEX".into());
    }
    let address = std::env::var("TACTUS_CKB_RPC_ADDR")
        .map_err(|_| "explicit TACTUS_CKB_RPC_ADDR required")?;
    let mut limits = Limits::default();
    if let Ok(value) = std::env::var("TACTUS_VAULT_MAX_BLOCKS") {
        limits.blocks = value.parse().map_err(|_| "block limit")?
    }
    if let Ok(value) = std::env::var("TACTUS_VAULT_MAX_DEPOSITS") {
        limits.deposits = value.parse().map_err(|_| "deposit limit")?
    }
    let mut capture = std::env::var_os("TACTUS_VAULT_CAPTURE")
        .map(|p| Capture::new(p.into()))
        .transpose()?;
    let report = recover(
        &address,
        &args[1],
        &rpc::decode_hex(&args[2])?,
        limits,
        |block| {
            if let Some(out) = &mut capture {
                out.push(block)?
            }
            Ok(())
        },
    )?;
    if let Some(out) = &mut capture {
        out.commit()?
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("VAULT RECOVERY FAILED: {error}");
        std::process::exit(1)
    }
}
