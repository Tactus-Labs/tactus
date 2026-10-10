//! Real deposit transactions only; no synthetic proof is submitted to CKB.
use serde_json::{json, Value};
use tactus_o1_devnet_driver::{
    lab::{self, Lab},
    molecule, rpc,
    tx::{self, CellOutPoint, OutSpec, TX_FEE},
};
use tactus_o1_native_vault_script::{hash as digest, Config, State};
fn number(v: &Value) -> u64 {
    u64::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
}
fn output(lock: Vec<u8>, script: Vec<u8>, data: Vec<u8>, capacity: Option<u64>) -> OutSpec {
    OutSpec {
        capacity: capacity
            .unwrap_or_else(|| OutSpec::required_capacity(&lock, Some(&script), data.len())),
        lock,
        type_script: Some(script),
        data,
    }
}
fn build(
    lab: &Lab,
    actor: usize,
    vault: Option<(CellOutPoint, u64)>,
    mut outputs: Vec<OutSpec>,
) -> Result<Value, String> {
    let w = &lab.wallets[actor];
    let mut inputs = vec![];
    if let Some(v) = vault {
        inputs.push(v)
    }
    inputs.push((w.point, w.capacity));
    let total = inputs
        .iter()
        .try_fold(0u64, |a, (_, c)| a.checked_add(*c))
        .ok_or("inputs overflow")?;
    let used = outputs
        .iter()
        .try_fold(TX_FEE, |a, o| a.checked_add(o.capacity))
        .ok_or("outputs overflow")?;
    outputs.push(OutSpec {
        capacity: total.checked_sub(used).ok_or("insufficient funds")?,
        lock: w.key.lock_script(),
        type_script: None,
        data: vec![],
    });
    tx::build_with_permissionless_prefix(
        &w.key,
        &lab.secp,
        &lab.deps,
        &inputs,
        &outputs,
        None,
        usize::from(vault.is_some()),
    )
    .map(|(_, t)| t)
}
fn commit(lab: &mut Lab, actor: usize, label: &str, tx: &Value) -> Result<String, String> {
    let hash = lab.commit(label, tx)?;
    let index = tx["outputs"].as_array().unwrap().len() - 1;
    lab.wallets[actor].point = lab::point(&hash, index as u32)?;
    lab.wallets[actor].capacity = number(&tx["outputs"][index]["capacity"]);
    Ok(hash)
}
fn reject(lab: &mut Lab, label: &str, tx: &Value, code: i8) -> Result<(), String> {
    let reason = format!("error code {code}");
    let program = digest(b"", &lab.ordering_elf);
    // Both the vault input and its receipt output validate the same deposit.
    // CKB may report either exact type group, depending on execution order.
    let sources = [
        "Inputs[0].Type",
        "Inputs[1].Type",
        "Outputs[0].Type",
        "Outputs[1].Type",
    ];
    match rpc::send_transaction_json(tx) {
        Err(error)
            if sources
                .iter()
                .any(|s| lab::rejection_matches_at(&error, &reason, &program, s)) =>
        {
            lab.evidence.push(json!({"label":label,"result":"rejected","expected_reason":reason,"error":error,"transaction":tx}));
            println!("{label}: rejected ({reason})");
            Ok(())
        }
        other => Err(format!("{label}: expected {reason}, got {other:?}")),
    }
}

fn run() -> Result<(), String> {
    let evidence = std::env::var("TACTUS_EVIDENCE_PATH").map_err(|_| "use isolated launcher")?;
    let mut lab = Lab::connect_with_script("artifacts/tactus_o1_native_vault_script.elf")?;
    let code = digest(b"", &lab.ordering_elf);
    let point = lab.wallets[0].point;
    let input = molecule::cell_input(0, &molecule::out_point(&point.tx_hash, point.index));
    let identity = digest(b"", &[input.as_slice(), &0u64.to_le_bytes()].concat());
    let ckb_genesis = rpc::decode_hex(
        rpc::call("get_block_hash", json!(["0x0"]))?
            .as_str()
            .ok_or("genesis")?,
    )?
    .try_into()
    .map_err(|_| "genesis length")?;
    // Deliberately separate fixture domain. There is no live settlement for it yet.
    let cfg = Config {
        identity,
        ckb_genesis,
        rollup: [3; 32],
        chain: 31337,
        contract: [4; 20],
        settlement: [5; 32],
    };
    let script = molecule::script(&code, 2, &cfg.encode());
    let script_hash = digest(b"", &script);
    let lock = molecule::script(&digest(b"", &lab.lock_elf), 2, &script_hash);
    let receipt_script =
        molecule::script(&code, 2, &[b"TO1REC01".as_slice(), &script_hash].concat());
    let immutable = molecule::script(&code, 2, &[]);
    let reserve = OutSpec::required_capacity(
        &lock,
        Some(&script),
        tactus_o1_native_vault_script::STATE_BYTES,
    );
    let mut state = State::genesis(&cfg, reserve);
    let mut result = json!({"suite":"native-vault-deposits-v1","complete":false,"error":null,"production_ready":false,"authenticated_l2_credit":false,"withdrawal_executed":false,"config":rpc::bytes_to_hex(&cfg.encode()),"vault_script":rpc::bytes_to_hex(&script),"receipt_script":rpc::bytes_to_hex(&receipt_script),"runtime_code_hash":rpc::bytes_to_hex(cfg.runtime_hash().as_slice()),"domain":rpc::bytes_to_hex(cfg.domain().as_slice()),"reserve":reserve,"deposits":[]});
    let outcome = (|| {
        let genesis = output(
            lock.clone(),
            script.clone(),
            state.encode().to_vec(),
            Some(reserve),
        );
        let mut forged = genesis.clone();
        forged.data[8] ^= 1;
        let bad_genesis = build(&lab, 0, None, vec![forged])?;
        reject(&mut lab, "vault/false reserve", &bad_genesis, 4)?;
        let mut wrong_cfg = cfg.clone();
        wrong_cfg.identity[0] ^= 1;
        let wrong_script = molecule::script(&code, 2, &wrong_cfg.encode());
        let wrong_lock =
            molecule::script(&digest(b"", &lab.lock_elf), 2, &digest(b"", &wrong_script));
        let wrong_state = State::genesis(&wrong_cfg, reserve);
        let wrong = build(
            &lab,
            0,
            None,
            vec![output(
                wrong_lock,
                wrong_script,
                wrong_state.encode().to_vec(),
                Some(reserve),
            )],
        )?;
        reject(&mut lab, "vault/forged genesis identity", &wrong, 2)?;
        let tx = build(&lab, 0, None, vec![genesis])?;
        let hash = commit(&mut lab, 0, "vault/genesis", &tx)?;
        let mut vault = lab::point(&hash, 0)?;
        result["genesis"] = json!({"hash":hash,"state":rpc::bytes_to_hex(&state.encode())});
        for round in 0..2 {
            let actor = round;
            let recipient = [0x11 + round as u8; 20];
            let amount = (100 + 50 * round as u64) * 100_000_000;
            let (next, record) = state
                .deposit(&cfg, recipient, amount)
                .map_err(|e| format!("deposit {e}"))?;
            let old = Some((vault, state.capacity().map_err(|_| "capacity")?));
            let vault_out = output(
                lock.clone(),
                script.clone(),
                next.encode().to_vec(),
                Some(next.capacity().unwrap()),
            );
            let receipt = output(
                immutable.clone(),
                receipt_script.clone(),
                record.encode().to_vec(),
                None,
            );
            if round == 0 {
                let forge = build(&lab, actor, None, vec![receipt.clone()])?;
                reject(
                    &mut lab,
                    "vault/receipt without funding transition",
                    &forge,
                    2,
                )?;
                for (name, offset) in [
                    ("wrong recipient", 16),
                    ("wrong amount", 36),
                    ("wrong sequence", 8),
                    ("wrong previous accumulator", 44),
                    ("wrong next accumulator", 76),
                    ("wrong cumulative deposit", 108),
                ] {
                    let mut changed = receipt.clone();
                    changed.data[offset] ^= 1;
                    let tx = build(&lab, actor, old, vec![vault_out.clone(), changed])?;
                    reject(&mut lab, &format!("vault/{name}"), &tx, 5)?;
                }
                let tx = build(&lab, actor, old, vec![vault_out.clone()])?;
                reject(&mut lab, "vault/missing receipt", &tx, 2)?;
                let tx = build(
                    &lab,
                    actor,
                    old,
                    vec![vault_out.clone(), receipt.clone(), receipt.clone()],
                )?;
                reject(&mut lab, "vault/duplicate receipts", &tx, 2)?;
                let mut short = vault_out.clone();
                short.capacity -= 1;
                let tx = build(&lab, actor, old, vec![short, receipt.clone()])?;
                reject(&mut lab, "vault/underfunded deposit", &tx, 4)?;
                let mut stolen = vault_out.clone();
                stolen.lock = lab.wallets[0].key.lock_script();
                let tx = build(&lab, actor, old, vec![stolen, receipt.clone()])?;
                reject(&mut lab, "vault/lock takeover", &tx, 4)?;
                let mut reserve = next.clone();
                reserve.reserve += 1;
                let changed = output(
                    lock.clone(),
                    script.clone(),
                    reserve.encode().to_vec(),
                    Some(reserve.capacity().unwrap()),
                );
                let tx = build(&lab, actor, old, vec![changed, receipt.clone()])?;
                reject(&mut lab, "vault/reserve rewrite", &tx, 5)?;
                let mut ordinary = receipt.clone();
                ordinary.lock = lab.wallets[0].key.lock_script();
                ordinary.capacity = OutSpec::required_capacity(
                    &ordinary.lock,
                    ordinary.type_script.as_deref(),
                    ordinary.data.len(),
                );
                let tx = build(&lab, actor, old, vec![vault_out.clone(), ordinary])?;
                reject(&mut lab, "vault/mutable receipt lock", &tx, 5)?;
            }
            let tx = build(&lab, actor, old, vec![vault_out, receipt])?;
            let hash = commit(
                &mut lab,
                actor,
                &format!("vault/deposit actor {actor}"),
                &tx,
            )?;
            vault = lab::point(&hash, 0)?;
            state = next;
            let receipt_live = rpc::call(
                "get_live_cell",
                json!([{"tx_hash":hash,"index":"0x1"},true]),
            )?;
            if receipt_live["status"] != "live"
                || receipt_live["cell"]["data"]["content"] != rpc::bytes_to_hex(&record.encode())
            {
                return Err("receipt not live".into());
            }
            result["deposits"].as_array_mut().unwrap().push(json!({"actor":actor,"hash":hash,"record":rpc::bytes_to_hex(&record.encode()),"deposit_id":rpc::bytes_to_hex(&cfg.deposit_id(record.sequence)),"receipt_live":receipt_live}));
        }
        // A second independently initialized vault must not share a funding or
        // payout transaction with the first; both enforce the same type-input boundary.
        let seed = lab.wallets[0].point;
        let input = molecule::cell_input(0, &molecule::out_point(&seed.tx_hash, seed.index));
        let mut other_cfg = cfg.clone();
        other_cfg.identity = digest(b"", &[input.as_slice(), &0u64.to_le_bytes()].concat());
        let other_script = molecule::script(&code, 2, &other_cfg.encode());
        let other_lock =
            molecule::script(&digest(b"", &lab.lock_elf), 2, &digest(b"", &other_script));
        let other_state = State::genesis(&other_cfg, reserve);
        let tx = build(
            &lab,
            0,
            None,
            vec![output(
                other_lock.clone(),
                other_script.clone(),
                other_state.encode().to_vec(),
                Some(reserve),
            )],
        )?;
        let second = commit(&mut lab, 0, "vault/second independent vault", &tx)?;
        result["second_vault"] = json!({"hash":second,"config":rpc::bytes_to_hex(&other_cfg.encode()),"state":rpc::bytes_to_hex(&other_state.encode())});
        let (next, record) = state
            .deposit(&cfg, [0x13; 20], 100_000_000)
            .map_err(|e| format!("deposit {e}"))?;
        let inputs = vec![
            (vault, state.capacity().unwrap()),
            (lab::point(&second, 0)?, reserve),
            (lab.wallets[0].point, lab.wallets[0].capacity),
        ];
        let mut outputs = vec![
            output(
                lock.clone(),
                script.clone(),
                next.encode().to_vec(),
                Some(next.capacity().unwrap()),
            ),
            output(
                other_lock,
                other_script,
                other_state.encode().to_vec(),
                Some(reserve),
            ),
            output(
                immutable.clone(),
                receipt_script.clone(),
                record.encode().to_vec(),
                None,
            ),
        ];
        let available: u64 = inputs.iter().map(|(_, c)| *c).sum();
        let needed: u64 = outputs.iter().map(|o| o.capacity).sum();
        outputs.push(OutSpec {
            capacity: available - needed - TX_FEE,
            lock: lab.wallets[0].key.lock_script(),
            type_script: None,
            data: vec![],
        });
        let (_, combined) = tx::build_with_permissionless_prefix(
            &lab.wallets[0].key,
            &lab.secp,
            &lab.deps,
            &inputs,
            &outputs,
            None,
            2,
        )?;
        reject(
            &mut lab,
            "vault/only one typed input per transition",
            &combined,
            2,
        )?;
        let duplicated = build(
            &lab,
            0,
            None,
            vec![output(
                lock.clone(),
                script.clone(),
                State::genesis(&cfg, reserve).encode().to_vec(),
                Some(reserve),
            )],
        )?;
        reject(&mut lab, "vault/cannot recreate singleton", &duplicated, 2)?;
        let old = Some((vault, state.capacity().unwrap()));
        let mut stolen = state.clone();
        stolen.released = 1;
        let tx = build(
            &lab,
            0,
            old,
            vec![output(
                lock.clone(),
                script.clone(),
                stolen.encode().to_vec(),
                Some(stolen.capacity().unwrap()),
            )],
        )?;
        reject(&mut lab, "vault/no release without proof", &tx, 6)?;
        let tx = build(&lab, 0, old, vec![])?;
        reject(&mut lab, "vault/cannot destroy funded vault", &tx, 3)?;
        let final_live = rpc::call(
            "get_live_cell",
            json!([{"tx_hash":rpc::bytes_to_hex(&vault.tx_hash),"index":"0x0"},true]),
        )?;
        if final_live["status"] != "live"
            || final_live["cell"]["data"]["content"] != rpc::bytes_to_hex(&state.encode())
        {
            return Err("vault not live".into());
        }
        result["final_state"] = json!(rpc::bytes_to_hex(&state.encode()));
        result["final_live"] = final_live;
        result["complete"] = json!(true);
        Ok(())
    })();
    if let Err(error) = &outcome {
        result["error"] = json!(error)
    }
    lab.save(&evidence, result)?;
    outcome
}
fn main() {
    if let Err(e) = run() {
        eprintln!("NATIVE VAULT FAILED: {e}");
        std::process::exit(1)
    }
}
