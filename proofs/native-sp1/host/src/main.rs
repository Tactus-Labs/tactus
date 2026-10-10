//! Local execution of the native custody guest. This runner creates no proof.
use sp1_sdk::{Elf, HashableKey, Prover, ProverClient, ProvingKey, SP1Stdin};
use std::{fs, io::Read, path::Path, time::Instant};
use tactus_o1_native_proof_journal::{execute, Domain, Journal};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn err(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
async fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: tactus-o1-native-sp1-host GUEST_ELF INPUT_JSON NEW_OUTPUT_DIR".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&args[2])
        .map_err(err)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("input exceeds 64 MiB".into());
    }
    let input: serde_json::Value = serde_json::from_slice(&bytes).map_err(err)?;
    if input["schema"] != "native-proof-input-v2" {
        return Err("wrong input schema".into());
    }
    let decode = |v: &serde_json::Value| -> Result<Vec<u8>, String> {
        hex::decode(
            v.as_str()
                .and_then(|s| s.strip_prefix("0x"))
                .ok_or("expected 0x hex")?,
        )
        .map_err(err)
    };
    let domain = Domain::decode(&decode(&input["domain_hex"])?).map_err(err)?;
    let allocation = decode(&input["allocation_hex"])?;
    let batches = input["batches"]
        .as_array()
        .ok_or("missing batches")?
        .iter()
        .map(decode)
        .collect::<Result<Vec<_>, _>>()?;
    let prefix = input["prefix_batches"].as_u64().ok_or("missing prefix")?;
    let interval = (batches.len() as u64)
        .checked_sub(prefix)
        .filter(|n| *n > 0)
        .ok_or("invalid interval")?;
    let mut iter = batches.iter();
    let expected = execute(domain.clone(), &allocation, prefix, interval, || {
        iter.next().unwrap().clone()
    })
    .map_err(err)?;
    if decode(&input["expected_journal_hex"])? != expected.encode() {
        return Err("native replay differs from expected journal".into());
    }
    let elf = Elf::from(fs::read(&args[1]).map_err(err)?);
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(domain.encode().to_vec());
    stdin.write_vec(allocation);
    stdin.write(&prefix);
    stdin.write(&interval);
    for batch in batches {
        stdin.write_vec(batch);
    }
    let output = Path::new(&args[3]);
    fs::create_dir(output).map_err(err)?;
    let start = Instant::now();
    let prover = ProverClient::builder().cpu().build().await;
    let pk = prover.setup(elf.clone()).await.map_err(err)?;
    let key = pk.verifying_key().bytes32();
    let (public, report) = prover.execute(elf, stdin).await.map_err(err)?;
    if Journal::decode(public.as_slice()).map_err(err)? != expected {
        return Err("zkVM differs from native replay".into());
    }
    fs::write(output.join("public-values.bin"), public.as_slice()).map_err(err)?;
    fs::write(output.join("execution-report.txt"), format!("{report:?}\n")).map_err(err)?;
    let result = serde_json::json!({"schema": "native-guest-execution-v2", "sp1":"6.8.1", "backend":"local-cpu", "guest_verifying_key":key, "public_values_hex":hex::encode(public.as_slice()), "native_replay_match":true, "prefix_batches":prefix, "interval_batches":interval, "execution_seconds_including_setup":start.elapsed().as_secs_f64(), "proof_generated":false, "authenticated_publication":false, "ckb_settlement":false, "custody_release":false, "production_ready":false});
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&result).map_err(err)?,
    )
    .map_err(err)?;
    println!("Native guest matched full journal; key {key}; no proof generated");
    Ok(())
}
