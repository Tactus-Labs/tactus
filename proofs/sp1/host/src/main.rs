//! Local CPU execution/proof experiment. No remote proving backend is enabled.
use sp1_sdk::{
    Elf, HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1ProofWithPublicValues,
    SP1PublicValues, SP1Stdin,
};
use std::{fs, path::Path, time::Instant};
use tactus_o1_execution::{Executor, Genesis};
use tactus_o1_proof_journal::{execute, Domain, Journal};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 || !["execute", "prove", "verify"].contains(&args[1].as_str()) {
        return Err("usage: tactus-o1-sp1-host execute|prove|verify GUEST_ELF GETH_FIXTURE CASE_INDEX OUTPUT_DIR [WRONG_GUEST_ELF]".into());
    }
    let fixture: serde_json::Value =
        serde_json::from_slice(&fs::read(&args[3]).map_err(err)?).map_err(err)?;
    let index = args[4].parse::<usize>().map_err(err)?;
    let case = fixture["cases"].get(index).ok_or("missing fixture case")?;
    let genesis: Genesis = serde_json::from_value(case["genesis"].clone()).map_err(err)?;
    let batch = hex::decode(
        case["batch"]
            .as_str()
            .ok_or("missing batch")?
            .trim_start_matches("0x"),
    )
    .map_err(err)?;
    // Explicit laboratory deployment context; these are not CKB network claims.
    let domain = Domain {
        ckb_genesis: [1; 32],
        ordering_type_hash: [2; 32],
        settlement_type_hash: [3; 32],
        rollup_id: genesis.rollup_id.0,
        chain_id: genesis.chain_id,
    };
    let allocation = genesis.allocation_bytes().map_err(err)?;
    let expected = execute(domain.clone(), &allocation, 0, 1, || batch.clone()).map_err(err)?;
    let mut host = Executor::new(&genesis).map_err(err)?;
    let blocks = host.apply_batch(&batch).map_err(err)?;
    let oracles = case["geth"].as_array().ok_or("missing Geth results")?;
    if blocks.len() != oracles.len() {
        return Err("Geth block count differs".into());
    }
    for (block, oracle) in blocks.iter().zip(oracles) {
        for (actual, field) in [
            (block.header.state_root, "stateRoot"),
            (block.header.transactions_root, "txRoot"),
            (block.header.receipts_root, "receiptsRoot"),
        ] {
            if format!("{actual}") != oracle[field].as_str().ok_or("missing Geth root")? {
                return Err(format!("Geth {field} mismatch"));
            }
        }
    }
    let elf = Elf::from(fs::read(&args[2]).map_err(err)?);
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(domain.encode().to_vec());
    stdin.write_vec(allocation);
    stdin.write(&0u64);
    stdin.write(&1u64);
    stdin.write_vec(batch);
    let output = Path::new(&args[5]);
    if args[1] != "verify" {
        fs::create_dir(output).map_err(err)?;
    }
    let started = Instant::now();
    let prover = ProverClient::builder().cpu().build().await;
    eprintln!("Local CPU prover initialized after {:?}", started.elapsed());
    let pk = prover.setup(elf.clone()).await.map_err(err)?;
    let key = pk.verifying_key().bytes32();
    if args[1] == "verify" {
        let proof = SP1ProofWithPublicValues::load(output.join("proof.bin")).map_err(err)?;
        prover
            .verify(&proof, pk.verifying_key(), None)
            .map_err(err)?;
        if proof.public_values.as_slice() != expected.encode() {
            return Err("valid proof does not match expected deployment/interval/state".into());
        }
        println!("Fresh process verified real proof and exact expected journal; key {key}");
        return Ok(());
    }
    let (public, report) = prover.execute(elf, stdin.clone()).await.map_err(err)?;
    let decoded = Journal::decode(public.as_slice()).map_err(err)?;
    if decoded != expected {
        return Err("zkVM result differs from native execution".into());
    }
    fs::write(output.join("execution-report.txt"), format!("{report:?}\n")).map_err(err)?;
    fs::write(output.join("public-values.bin"), public.as_slice()).map_err(err)?;
    let execution_seconds = started.elapsed().as_secs_f64();
    eprintln!("Guest matched native and independent Geth roots after {execution_seconds:.3}s");
    let mut result = serde_json::json!({"schema":1,"case":case["name"],"fixture_case_index":index,"backend":"local-cpu","sp1":"6.8.1","guest_verifying_key":key,"public_values_hex":hex::encode(public.as_slice()),"native_and_geth_roots_match":true,"execution_seconds_including_setup":execution_seconds,"proof_generated":false,"ckb_settlement":false,"production_ready":false,"domain":"explicit laboratory identities 01/02/03; no CKB authentication"});
    if args[1] == "prove" {
        let proof = prover.prove(&pk, stdin).core().await.map_err(err)?;
        if proof.public_values.as_slice() != expected.encode() {
            return Err("proof public values differ".into());
        }
        prover
            .verify(&proof, pk.verifying_key(), None)
            .map_err(err)?;
        proof.save(output.join("proof.bin")).map_err(err)?;
        let mut rejected = Vec::new();
        for (label, offset) in [
            ("profile", 8),
            ("ckb-network", 40),
            ("ordering-script", 72),
            ("settlement-script", 104),
            ("rollup", 136),
            ("chain", 168),
            ("allocation", 176),
            ("predecessor", 248),
            ("end-history", 456),
            ("previous-state", 608),
            ("next-state", 640),
            ("previous-header", 672),
            ("next-header", 704),
            ("interval-data", 736),
        ] {
            let mut altered = proof.clone();
            let mut bytes = public.as_slice().to_vec();
            bytes[offset] ^= 1;
            altered.public_values = SP1PublicValues::from(&bytes);
            if prover.verify(&altered, pk.verifying_key(), None).is_ok() {
                return Err(format!("accepted tampered {label}"));
            }
            rejected.push(label);
        }
        if let Some(other) = args.get(6) {
            let other_key = prover
                .setup(Elf::from(fs::read(other).map_err(err)?))
                .await
                .map_err(err)?;
            if prover
                .verify(&proof, other_key.verifying_key(), None)
                .is_ok()
            {
                return Err("accepted wrong guest key".into());
            }
            rejected.push("wrong-guest-key");
        }
        result["proof_generated"] = true.into();
        result["proof_kind"] = "SP1 real core STARK".into();
        result["negative_controls"] = serde_json::json!(rejected);
        result["proof_bytes"] = fs::metadata(output.join("proof.bin"))
            .map_err(err)?
            .len()
            .into();
        result["total_seconds"] = started.elapsed().as_secs_f64().into();
        eprintln!(
            "Real CPU proof verified; {} negative controls passed",
            rejected.len()
        );
    }
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&result).map_err(err)?,
    )
    .map_err(err)?;
    Ok(())
}
fn err(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
