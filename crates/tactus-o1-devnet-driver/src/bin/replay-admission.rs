//! Matched admission-vs-admission races with independent fee inputs.
use serde_json::{json, Value};
use tactus_o1_devnet_driver::{
    lab::{self, Lab},
    molecule, rpc, sealed_lab as sealed,
    tx::{wire_bytes, CellOutPoint, OutSpec, TX_FEE},
};
use tactus_o1_ordering_script::{ckb_blake2b, genesis_identity};
use tactus_o1_protocol::priority::Message;

fn live(point: CellOutPoint) -> Result<bool, String> {
    Ok(rpc::call(
        "get_live_cell",
        json!([{"tx_hash":rpc::bytes_to_hex(&point.tx_hash),"index":format!("0x{:x}",point.index)},false]),
    )?["status"]
        == "live")
}
fn admitted(
    lab: &Lab,
    net: &sealed::Network,
    code: &[u8; 32],
    actor: usize,
    payload: Vec<u8>,
    fee: u64,
) -> Result<Value, String> {
    let p = lab.wallets[actor].point;
    let seed = molecule::cell_input(0, &molecule::out_point(&p.tx_hash, p.index))
        .try_into()
        .unwrap();
    let id = genesis_identity(&seed, 0);
    let script = sealed::role(code, 0, &id, None);
    let lock = sealed::role(code, 1, &ckb_blake2b(&script), None);
    let data = Message::admitted(
        id,
        net.anchor.state.rollup_id,
        ckb_blake2b(&net.anchor.script),
        payload,
    )
    .map_err(|e| format!("{e:?}"))?
    .encode()
    .map_err(|e| format!("{e:?}"))?;
    let capacity = OutSpec::required_capacity(&lock, Some(&script), data.len()) + TX_FEE;
    sealed::shape_with_fee(
        lab,
        actor,
        &[],
        vec![OutSpec {
            capacity,
            lock,
            type_script: Some(script),
            data,
        }],
        &[],
        &[],
        fee,
    )
}
fn metrics(t: &Value, fee: u64) -> Result<Value, String> {
    let cycles = rpc::call("estimate_cycles", json!([t]))?;
    Ok(json!({"fee_shannons":fee,"wire_bytes":wire_bytes(t)?,"cycles":cycles,"transaction":t}))
}
fn race(
    lab: &mut Lab,
    label: &str,
    txs: [Value; 2],
    victim_state: Option<CellOutPoint>,
    conflict: bool,
    delay: u64,
    ratio: u64,
) -> Result<Value, String> {
    let candidates = [metrics(&txs[0], TX_FEE)?, metrics(&txs[1], TX_FEE * ratio)?];
    let start = rpc::get_tip_block_number()?;
    let first = lab.attempt(&format!("{label}/adversary-first"), &txs[0])??;
    rpc::mine_blocks(delay)?;
    let state_live = victim_state.map(live).transpose()?;
    if !live(lab.wallets[1].point)? {
        return Err("victim funding consumed by adversary".into());
    }
    let second = lab.attempt(&format!("{label}/delayed-victim"), &txs[1])?;
    let mut committed = [false; 2];
    let hashes = [
        Some(first.as_str()),
        second.as_ref().ok().map(String::as_str),
    ];
    for _ in 0..20 {
        for i in 0..2 {
            committed[i] = hashes[i]
                .map(|h| rpc::get_transaction_status(h).map(|s| s.0 == "committed"))
                .transpose()?
                .unwrap_or(false);
        }
        if conflict && committed.iter().all(|v| *v) {
            return Err("conflicting transactions both committed".into());
        }
        if (conflict && committed.iter().any(|v| *v)) || (!conflict && committed.iter().all(|v| *v))
        {
            break;
        }
        rpc::mine_blocks(1)?;
    }
    if (!conflict && committed != [true, true]) || !committed.iter().any(|v| *v) {
        return Err(format!(
            "{label}: missing canonical admission {committed:?}"
        ));
    }
    let mut records = Vec::new();
    for i in 0..2 {
        if committed[i] {
            let hash = hashes[i].unwrap();
            lab.record_committed(&format!("{label}/actor-{i}-canonical"), hash)?;
            let packed = rpc::call("get_transaction", json!([hash, "0x0"]))?;
            let actual_bytes =
                rpc::decode_hex(packed["transaction"].as_str().ok_or("packed transaction")?)?.len();
            if actual_bytes != wire_bytes(&txs[i])? {
                return Err(format!("{label}: node wire size differs from candidate"));
            }

            records.push(
                json!({"actor":i,"hash":hash,"node_wire_bytes":actual_bytes,"view":rpc::call("get_transaction",json!([hash]))?}),
            );
            let outputs = txs[i]["outputs"].as_array().ok_or("outputs")?;
            lab.wallets[i].point = lab::point(hash, (outputs.len() - 1) as u32)?;
            lab.wallets[i].capacity = u64::from_str_radix(
                outputs.last().unwrap()["capacity"]
                    .as_str()
                    .ok_or("capacity")?
                    .trim_start_matches("0x"),
                16,
            )
            .map_err(|e| e.to_string())?;
        } else if !live(lab.wallets[i].point)? {
            return Err("loser funding is not live".into());
        }
    }
    let classification = if committed[1] {
        "victim_committed"
    } else if state_live == Some(false) {
        "stale_input"
    } else if second.is_err() {
        "live_pool_rejection"
    } else {
        "accepted_lost_conflict"
    };
    println!("{label}: {classification}, canonical={committed:?}");
    Ok(
        json!({"label":label,"delay_blocks":delay,"victim_fee_ratio":ratio,"shared_state_input":conflict,"victim_state_live_at_submit":state_live,"victim_funding_live_at_submit":true,"classification":classification,"adversary_committed":committed[0],"victim_committed":committed[1],"victim_submit_error":second.err(),"start_height":start,"end_height":rpc::get_tip_block_number()?,"candidates":candidates,"canonical":records}),
    )
}
fn run() -> Result<(), String> {
    let path = std::env::var("TACTUS_EVIDENCE_PATH").map_err(|_| "use isolated launcher")?;
    let mut lab = Lab::connect()?;
    let mut results = json!({"suite":"a123-matched-admission-v1","complete":false,"production_ready":false,"G2":"OPEN","scope":"admission-only; deterministic adversary-first schedule, two repeats; not stochastic fairness, forced execution or settlement"});
    let outcome = (|| {
        let anchor =
            std::fs::read("artifacts/tactus_o1_anchor_script.elf").map_err(|e| e.to_string())?;
        let priority =
            std::fs::read("artifacts/tactus_o1_priority_script.elf").map_err(|e| e.to_string())?;
        let gate =
            std::fs::read("artifacts/tactus_o1_sealed_script.elf").map_err(|e| e.to_string())?;
        let priority_code = ckb_blake2b(&priority);
        let gate_code = ckb_blake2b(&gate);
        let deps = lab.publish_cells(
            "admission/deploy comparator programs",
            &[anchor.clone(), priority, gate],
            0,
            true,
        )?;
        lab.deps.extend(deps);
        let mut rows = Vec::new();
        for arm in [
            "A1-shared-head",
            "A2-independent-messages",
            "A3-one-lane",
            "A3-four-lanes-targeted",
            "A3-four-lanes-disjoint",
        ] {
            for delay in [0, 1, 3, 6] {
                for ratio in [1, 2, 10] {
                    for repeat in 0..2 {
                        let label = format!("{arm}/delay-{delay}/fee-{ratio}/repeat-{repeat}");
                        // Identical payload bytes and absolute fees across arms. A1
                        // stores a commitment; the authenticated A2/A3 arms store bytes.
                        let payloads = [vec![0xa0; 64], vec![0xb1; 64]];
                        let (transactions, state, conflict) = if arm == "A1-shared-head" {
                            let head = lab.create_head(0)?;
                            let mut ts = Vec::new();
                            for (actor, payload) in payloads.iter().enumerate() {
                                let hash = ckb_blake2b(payload);
                                ts.push(lab.transition_tx(
                                    &head,
                                    lab::enqueue(&head, &hash),
                                    &hash,
                                    actor,
                                    TX_FEE * if actor == 0 { 1 } else { ratio },
                                    &[],
                                )?);
                            }
                            (ts.try_into().unwrap(), Some(head.point), true)
                        } else {
                            let n = if arm.contains("four-lanes") { 4 } else { 1 };
                            let net =
                                sealed::bootstrap_with_program(&mut lab, gate_code, n, &anchor)?;
                            if arm == "A2-independent-messages" {
                                (
                                    [
                                        admitted(
                                            &lab,
                                            &net,
                                            &priority_code,
                                            0,
                                            payloads[0].clone(),
                                            TX_FEE,
                                        )?,
                                        admitted(
                                            &lab,
                                            &net,
                                            &priority_code,
                                            1,
                                            payloads[1].clone(),
                                            TX_FEE * ratio,
                                        )?,
                                    ],
                                    None,
                                    false,
                                )
                            } else {
                                let other = usize::from(arm.ends_with("disjoint"));
                                let mut ts = Vec::new();
                                for (actor, payload) in payloads.iter().enumerate() {
                                    let (cell, lane) =
                                        &net.lanes[if actor == 0 { 0 } else { other }];
                                    let next = lane
                                        .append(payload.clone())
                                        .map_err(|e| format!("{e:?}"))?;
                                    ts.push(sealed::shape_with_fee(
                                        &lab,
                                        actor,
                                        &[(cell.point, cell.capacity)],
                                        vec![cell.output(next.encode().unwrap())],
                                        &[],
                                        &[],
                                        TX_FEE * if actor == 0 { 1 } else { ratio },
                                    )?);
                                }
                                (
                                    ts.try_into().unwrap(),
                                    Some(net.lanes[other].0.point),
                                    other == 0,
                                )
                            }
                        };
                        let mut row = race(
                            &mut lab,
                            &label,
                            transactions,
                            state,
                            conflict,
                            delay,
                            ratio,
                        )?;
                        row["arm"] = json!(arm);
                        row["repeat"] = json!(repeat);
                        rows.push(row);
                        // Retain partial evidence if a later case fails.
                        results["rows"] = json!(rows);
                    }
                }
            }
        }
        results["complete"] = json!(true);
        Ok::<_, String>(())
    })();
    results["error"] = json!(outcome.as_ref().err());
    lab.save(&path, results)?;
    outcome
}
fn main() {
    if let Err(e) = run() {
        eprintln!("ADMISSION EXPERIMENT FAILED: {e}");
        std::process::exit(1);
    }
}
