//! Real A3 mandatory gate: authenticated complete seals, immutable dependencies,
//! bounded mandatory prefixes, hostile switching and planned rollback.
use serde_json::{json, Value};
use tactus_o1_devnet_driver::{
    batch_lab::{self, Anchor},
    lab::{self, Lab},
    molecule, rpc,
    tx::{self, CellOutPoint, OutSpec, TX_FEE},
};
use tactus_o1_execution::{rules_hash, Executor, Genesis};
use tactus_o1_ordering_script::{ckb_blake2b, genesis_identity};
use tactus_o1_protocol::{
    batch::{self, AnchorState},
    sealed::{self as s, Lane, Schedule, Snapshot},
};
#[derive(Clone)]
struct Cell {
    point: CellOutPoint,
    capacity: u64,
    script: Vec<u8>,
    lock: Vec<u8>,
}
impl Cell {
    fn output(&self, data: Vec<u8>) -> OutSpec {
        OutSpec {
            capacity: self.capacity,
            lock: self.lock.clone(),
            type_script: Some(self.script.clone()),
            data,
        }
    }
}
#[derive(Clone)]
struct Network {
    code: [u8; 32],
    anchor: Anchor,
    gate: Cell,
    schedule: Schedule,
    lanes: Vec<(Cell, Lane)>,
    snapshot: Option<(CellOutPoint, Snapshot)>,
}
fn role(code: &[u8; 32], tag: u8, id: &[u8; 32], index: Option<u8>) -> Vec<u8> {
    let mut args = vec![tag];
    args.extend_from_slice(id);
    if let Some(index) = index {
        args.push(index);
    }
    molecule::script(code, 2, &args)
}
fn cell(point: CellOutPoint, code: &[u8; 32], script: Vec<u8>, reserved: usize) -> Cell {
    let lock = role(code, 0, &ckb_blake2b(&script), None);
    let capacity = OutSpec::required_capacity(&lock, Some(&script), reserved) + TX_FEE;
    Cell {
        point,
        capacity,
        script,
        lock,
    }
}
fn shape(
    lab: &Lab,
    actor: usize,
    prefix: &[(CellOutPoint, u64)],
    mut outputs: Vec<OutSpec>,
    input_types: &[(usize, Vec<u8>)],
    extra_deps: &[CellOutPoint],
) -> Result<Value, String> {
    let wallet = &lab.wallets[actor];
    let total = prefix.iter().try_fold(wallet.capacity, |n, (_, c)| {
        n.checked_add(*c).ok_or("capacity overflow")
    })?;
    let spent: u64 = outputs.iter().map(|o| o.capacity).sum();
    let change = total.checked_sub(spent + TX_FEE).ok_or("capacity")?;
    outputs.push(OutSpec {
        capacity: change,
        lock: wallet.key.lock_script(),
        type_script: None,
        data: vec![],
    });
    let mut inputs = prefix.to_vec();
    inputs.push((wallet.point, wallet.capacity));
    let mut deps = lab.deps.clone();
    deps.extend_from_slice(extra_deps);
    let (_, mut transaction) = tx::build_with_permissionless_prefix(
        &wallet.key,
        &lab.secp,
        &deps,
        &inputs,
        &outputs,
        None,
        prefix.len(),
    )?;
    for (index, data) in input_types {
        if *index >= prefix.len() {
            return Err("cannot mutate funding witness after signing".into());
        }
        transaction["witnesses"][*index] = json!(rpc::bytes_to_hex(&molecule::witness_args(
            None,
            Some(data),
            None
        )));
    }
    Ok(transaction)
}
fn commit(lab: &mut Lab, actor: usize, label: &str, transaction: &Value) -> Result<String, String> {
    let hash = lab.commit(label, transaction)?;
    let outputs = transaction["outputs"].as_array().ok_or("outputs")?;
    let change = outputs.last().ok_or("change")?;
    lab.wallets[actor].point = lab::point(&hash, (outputs.len() - 1) as u32)?;
    lab.wallets[actor].capacity = u64::from_str_radix(
        change["capacity"]
            .as_str()
            .ok_or("capacity")?
            .trim_start_matches("0x"),
        16,
    )
    .map_err(|e| e.to_string())?;
    Ok(hash)
}
fn reject(
    lab: &mut Lab,
    code: &[u8; 32],
    label: &str,
    transaction: &Value,
    location: &str,
    expected: i8,
) -> Result<(), String> {
    let error = match rpc::send_transaction_json(transaction) {
        Err(e) => e,
        Ok(hash) => return Err(format!("{label}: unexpectedly accepted {hash}")),
    };
    let value: Value = error
        .strip_prefix("rpc error: ")
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| error.clone())?;
    if value["code"] != -302
        || !location.split('|').any(|x| error.contains(x))
        || !error.contains(&format!("error code {expected} on page "))
        || !error.contains(&rpc::bytes_to_hex(code)[2..])
    {
        return Err(format!(
            "{label}: expected {location} code {expected}, got {error}"
        ));
    }
    lab.evidence.push(json!({"label":label,"result":"rejected","expected_location":location,"expected_code":expected,"error":error,"transaction":transaction}));
    println!("{label}: rejected ({expected})");
    Ok(())
}
fn live(point: CellOutPoint, expected: bool) -> Result<(), String> {
    let view = rpc::call(
        "get_live_cell",
        json!([{"tx_hash":rpc::bytes_to_hex(&point.tx_hash),"index":format!("0x{:x}",point.index)},false]),
    )?;
    if (view["status"] == "live") != expected {
        return Err(format!("liveness mismatch {view}"));
    }
    Ok(())
}
fn bootstrap(lab: &mut Lab, code: [u8; 32], n: u8) -> Result<Network, String> {
    let wallet = &lab.wallets[0];
    let seed: [u8; 44] = molecule::cell_input(
        0,
        &molecule::out_point(&wallet.point.tx_hash, wallet.point.index),
    )
    .try_into()
    .unwrap();
    let rollup = genesis_identity(&seed, 0);
    let id = genesis_identity(&seed, 1);
    let anchor_script = tx::tactus_o1_type_script(&lab.ordering_elf, &rollup);
    let gate_script = role(&code, 2, &id, None);
    let gate = cell(wallet.point, &code, gate_script, s::SCHEDULE_BYTES);
    let anchor_lock = role(&code, 0, &ckb_blake2b(&gate.script), None);
    let anchor = Anchor {
        point: wallet.point,
        capacity: OutSpec::required_capacity(&anchor_lock, Some(&anchor_script), batch::ANCHOR_LEN)
            + TX_FEE,
        state: AnchorState::genesis(rollup, rules_hash(), 31337).map_err(|e| format!("{e:?}"))?,
        lock: anchor_lock,
        script: anchor_script,
        immutable: molecule::script(&ckb_blake2b(&lab.ordering_elf), 2, &[]),
    };
    let schedule = Schedule::genesis(id, rollup, ckb_blake2b(&anchor.script), n)
        .map_err(|e| format!("{e:?}"))?;
    let mut outputs = vec![
        batch_lab::head_output(&anchor, anchor.state),
        gate.output(schedule.encode().unwrap()),
    ];
    let mut lanes = vec![];
    for i in 0..n {
        let c = cell(
            wallet.point,
            &code,
            role(&code, 1, &id, Some(i)),
            s::MAX_LANE_BYTES,
        );
        let lane = Lane::genesis(id, i).unwrap();
        outputs.push(c.output(lane.encode().unwrap()));
        lanes.push((c, lane));
    }
    let transaction = shape(lab, 0, &[], outputs, &[], &[])?;
    let hash = commit(lab, 0, &format!("sealed/{n} lanes/genesis"), &transaction)?;
    let mut network = Network {
        code,
        anchor,
        gate,
        schedule,
        lanes,
        snapshot: None,
    };
    network.anchor.point = lab::point(&hash, 0)?;
    network.gate.point = lab::point(&hash, 1)?;
    for (i, (c, _)) in network.lanes.iter_mut().enumerate() {
        c.point = lab::point(&hash, (i + 2) as u32)?;
    }
    Ok(network)
}
fn append_tx(
    lab: &Lab,
    net: &Network,
    index: usize,
    actor: usize,
    payload: Vec<u8>,
) -> Result<(Value, Lane), String> {
    let (c, old) = &net.lanes[index];
    let next = old.append(payload).map_err(|e| format!("{e:?}"))?;
    Ok((
        shape(
            lab,
            actor,
            &[(c.point, c.capacity)],
            vec![c.output(next.encode().unwrap())],
            &[],
            &[],
        )?,
        next,
    ))
}
fn append(
    lab: &mut Lab,
    net: &mut Network,
    index: usize,
    actor: usize,
    payload: Vec<u8>,
    label: &str,
) -> Result<(), String> {
    let (transaction, next) = append_tx(lab, net, index, actor, payload)?;
    let hash = commit(lab, actor, label, &transaction)?;
    net.lanes[index].0.point = lab::point(&hash, 0)?;
    net.lanes[index].1 = next;
    Ok(())
}
fn bytes(net: &Network, transactions: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    batch_lab::encode(
        net.anchor.state,
        vec![batch_lab::block(
            net.anchor.state.last_timestamp + 1,
            transactions,
        )],
    )
}
fn required_bytes(net: &Network) -> Result<Vec<u8>, String> {
    bytes(
        net,
        net.schedule
            .required(net.snapshot.as_ref().map(|(_, s)| s))
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .map(|m| m.payload)
            .collect(),
    )
}
fn advance_outputs(net: &Network, bytes: &[u8]) -> Result<Vec<OutSpec>, String> {
    let summary = batch::validate_batch(bytes, &net.anchor.state).map_err(|e| format!("{e:?}"))?;
    let mut next = net.schedule.clone();
    // Allows constructing a freshly signed ninth-batch attack with a validly
    // encoded but unchanged schedule; the real gate must reject it.
    if next.batches < s::BATCHES_PER_EPOCH {
        next.batches += 1;
        next.cursor =
            (u16::from(next.batches) * s::PRIORITY_PER_BATCH as u16).min(next.snapshot_messages);
    }
    Ok(vec![
        batch_lab::head_output(&net.anchor, summary.next),
        batch_lab::da_output(&net.anchor, bytes),
        net.gate.output(next.encode().unwrap()),
    ])
}
fn advance_tx(
    lab: &Lab,
    net: &Network,
    actor: usize,
    outputs: Vec<OutSpec>,
) -> Result<Value, String> {
    let deps: Vec<_> = net.snapshot.iter().map(|(point, _)| *point).collect();
    shape(
        lab,
        actor,
        &[
            (net.anchor.point, net.anchor.capacity),
            (net.gate.point, net.gate.capacity),
        ],
        outputs,
        &[(0, 1u32.to_le_bytes().to_vec()), (1, vec![1])],
        &deps,
    )
}
fn accept_advance(
    lab: &mut Lab,
    net: &mut Network,
    actor: usize,
    bytes: &[u8],
    transaction: &Value,
    label: &str,
) -> Result<(), String> {
    let (next, summary) = net
        .schedule
        .advance(
            net.snapshot.as_ref().map(|(_, s)| s),
            bytes,
            &net.anchor.state,
        )
        .map_err(|e| format!("{e:?}"))?;
    let hash = commit(lab, actor, label, transaction)?;
    net.anchor.point = lab::point(&hash, 0)?;
    net.anchor.state = summary.next;
    net.gate.point = lab::point(&hash, 2)?;
    net.schedule = next;
    Ok(())
}
fn advance(
    lab: &mut Lab,
    net: &mut Network,
    actor: usize,
    engine: &mut Executor,
    label: &str,
) -> Result<(), String> {
    let bytes = required_bytes(net)?;
    let transaction = advance_tx(lab, net, actor, advance_outputs(net, &bytes)?)?;
    accept_advance(lab, net, actor, &bytes, &transaction, label)?;
    engine.apply_batch(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}
fn seal_outputs(net: &Network) -> Result<(Vec<OutSpec>, Schedule, Vec<Lane>, Snapshot), String> {
    let lanes: Vec<_> = net.lanes.iter().map(|(_, s)| s.clone()).collect();
    let mut ready = net.schedule.clone();
    ready.batches = s::BATCHES_PER_EPOCH;
    ready.cursor = ready.snapshot_messages;
    let (next, active, sealed) = ready.seal(&lanes).map_err(|e| format!("{e:?}"))?;
    let mut outputs = vec![net.gate.output(next.encode().unwrap())];
    for ((c, _), state) in net.lanes.iter().zip(&active) {
        outputs.push(c.output(state.encode().unwrap()));
    }
    let data = sealed.encode().unwrap();
    let immutable = molecule::script(&net.code, 2, &[]);
    outputs.push(OutSpec {
        capacity: OutSpec::required_capacity(&immutable, None, data.len()),
        lock: immutable,
        type_script: None,
        data,
    });
    Ok((outputs, next, active, sealed))
}
fn seal_tx(
    lab: &Lab,
    net: &Network,
    actor: usize,
    outputs: Vec<OutSpec>,
    omit_last: bool,
) -> Result<Value, String> {
    let mut inputs = vec![(net.gate.point, net.gate.capacity)];
    let n = net.lanes.len() - usize::from(omit_last);
    inputs.extend(net.lanes[..n].iter().map(|(c, _)| (c.point, c.capacity)));
    let mut witness = vec![0];
    witness.extend_from_slice(&((outputs.len() - 1) as u32).to_le_bytes());
    shape(lab, actor, &inputs, outputs, &[(0, witness)], &[])
}
fn seal(lab: &mut Lab, net: &mut Network, actor: usize, label: &str) -> Result<(), String> {
    let (outputs, next, active, sealed) = seal_outputs(net)?;
    let transaction = seal_tx(lab, net, actor, outputs, false)?;
    let hash = commit(lab, actor, label, &transaction)?;
    net.gate.point = lab::point(&hash, 0)?;
    net.schedule = next;
    for (i, ((c, old), new)) in net.lanes.iter_mut().zip(active).enumerate() {
        c.point = lab::point(&hash, (i + 1) as u32)?;
        *old = new;
    }
    net.snapshot = Some((lab::point(&hash, (net.lanes.len() + 1) as u32)?, sealed));
    Ok(())
}
fn scenario(lab: &mut Lab, code: [u8; 32], n: u8) -> Result<Value, String> {
    let mut net = bootstrap(lab, code, n)?;
    let genesis = Genesis {
        rollup_id: net.anchor.state.rollup_id.into(),
        chain_id: 31337,
        accounts: Default::default(),
    };
    let mut engine = Executor::new(&genesis).map_err(|e| e.to_string())?;
    // Every scenario separately verifies the mandatory anchor lock boundary.
    let raw = bytes(&net, vec![])?;
    let summary = batch::validate_batch(&raw, &net.anchor.state).unwrap();
    let omitted = shape(
        lab,
        1,
        &[(net.anchor.point, net.anchor.capacity)],
        vec![
            batch_lab::head_output(&net.anchor, summary.next),
            batch_lab::da_output(&net.anchor, &raw),
        ],
        &[(0, 1u32.to_le_bytes().to_vec())],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/anchor cannot omit schedule input",
        &omitted,
        "Inputs[0].Lock",
        7,
    )?;
    let burn = shape(
        lab,
        1,
        &[(net.gate.point, net.gate.capacity)],
        vec![],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/schedule cannot be burned",
        &burn,
        "Inputs[0].Type",
        2,
    )?;
    let (outputs, _, _, _) = seal_outputs(&net)?;
    let early = seal_tx(lab, &net, 1, outputs, false)?;
    reject(
        lab,
        &code,
        "sealed/epoch cannot switch early",
        &early,
        "Inputs[0].Type",
        14,
    )?;
    let mut output = net.gate.output(net.schedule.encode().unwrap());
    output.lock = lab.wallets[1].key.lock_script();
    let tx = shape(
        lab,
        1,
        &[(net.gate.point, net.gate.capacity)],
        vec![output],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/schedule lock cannot be replaced",
        &tx,
        "Inputs[0].Type",
        5,
    )?;
    let mut output = net.gate.output(net.schedule.encode().unwrap());
    output.capacity -= 1;
    let tx = shape(
        lab,
        1,
        &[(net.gate.point, net.gate.capacity)],
        vec![output],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/schedule capacity cannot be drained",
        &tx,
        "Inputs[0].Type",
        5,
    )?;
    let tx = shape(
        lab,
        1,
        &[(net.gate.point, net.gate.capacity)],
        vec![
            net.gate.output(net.schedule.encode().unwrap()),
            net.gate.output(net.schedule.encode().unwrap()),
        ],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/schedule cannot be split",
        &tx,
        "Inputs[0].Type",
        12,
    )?;
    // A lane append signed before an unrelated batch remains valid: admission
    // does not read or consume the mutable schedule.
    let (pending, next_lane) = append_tx(lab, &net, 0, 0, vec![2, 0, 0])?;
    advance(
        lab,
        &mut net,
        1,
        &mut engine,
        "sealed/batch advances while lane admission is pending",
    )?;
    let hash = commit(
        lab,
        0,
        "sealed/pre-signed lane admission survives schedule change",
        &pending,
    )?;
    net.lanes[0] = (
        Cell {
            point: lab::point(&hash, 0)?,
            ..net.lanes[0].0.clone()
        },
        next_lane,
    );
    for i in 0..usize::from(n) {
        while net.lanes[i].1.queue.len() < 3 {
            let seq = net.lanes[i].1.next_sequence;
            append(
                lab,
                &mut net,
                i,
                0,
                vec![2, i as u8, seq as u8],
                "sealed/fill initial snapshot lanes",
            )?;
        }
    }
    let (c, _) = &net.lanes[0];
    let counterfeit = shape(
        lab,
        1,
        &[],
        vec![c.output(
            Lane::genesis(net.schedule.gate, 0)
                .unwrap()
                .encode()
                .unwrap(),
        )],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/cannot recreate a lane without fresh gate genesis",
        &counterfeit,
        "Outputs[0].Type",
        7,
    )?;
    let (c, old) = &net.lanes[0];
    let tx = shape(
        lab,
        0,
        &[(c.point, c.capacity)],
        vec![c.output(old.encode().unwrap())],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/partially filled lane cannot churn with no-op",
        &tx,
        "Inputs[0].Type",
        16,
    )?;
    let (lane_cell, old_lane) = &net.lanes[0];
    let cleared = Lane {
        epoch: old_lane.epoch + 1,
        base_root: old_lane.root,
        queue: vec![],
        ..old_lane.clone()
    };
    let tx = shape(
        lab,
        0,
        &[(lane_cell.point, lane_cell.capacity)],
        vec![lane_cell.output(cleared.encode().unwrap())],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/lane queue cannot reset without seal authorization",
        &tx,
        "Inputs[0].Type",
        7,
    )?;
    while net.schedule.batches < s::BATCHES_PER_EPOCH {
        advance(lab, &mut net, 1, &mut engine, "sealed/genesis epoch batch")?;
    }
    let raw = bytes(&net, vec![])?;
    let stale = advance_tx(lab, &net, 1, advance_outputs(&net, &raw)?)?;
    reject(
        lab,
        &code,
        "sealed/ninth batch must wait for seal",
        &stale,
        "Inputs[1].Type",
        14,
    )?;
    let (mut outputs, _, _, _) = seal_outputs(&net)?;
    outputs.remove(usize::from(n));
    let missing = seal_tx(lab, &net, 1, outputs, true)?;
    reject(
        lab,
        &code,
        "sealed/cannot omit configured lane at switch",
        &missing,
        "Inputs[0].Type",
        8,
    )?;
    let (mut outputs, _, _, _) = seal_outputs(&net)?;
    *outputs.last_mut().unwrap().data.last_mut().unwrap() ^= 1;
    let forged = seal_tx(lab, &net, 1, outputs, false)?;
    reject(
        lab,
        &code,
        "sealed/snapshot bytes must equal authentic lane history",
        &forged,
        "Inputs[0].Type",
        9,
    )?;
    let (mut outputs, _, _, _) = seal_outputs(&net)?;
    outputs.last_mut().unwrap().lock = lab.wallets[1].key.lock_script();
    let output = outputs.last_mut().unwrap();
    output.capacity = OutSpec::required_capacity(&output.lock, None, output.data.len());
    let mutable = seal_tx(lab, &net, 1, outputs, false)?;
    reject(
        lab,
        &code,
        "sealed/snapshot retention lock is mandatory",
        &mutable,
        "Inputs[0].Type",
        5,
    )?;
    let (mut outputs, _, _, _) = seal_outputs(&net)?;
    let raw = bytes(&net, vec![])?;
    let summary = batch::validate_batch(&raw, &net.anchor.state).unwrap();
    outputs.push(batch_lab::head_output(&net.anchor, summary.next));
    outputs.push(batch_lab::da_output(&net.anchor, &raw));
    let mut inputs = vec![(net.gate.point, net.gate.capacity)];
    inputs.extend(net.lanes.iter().map(|(c, _)| (c.point, c.capacity)));
    inputs.push((net.anchor.point, net.anchor.capacity));
    let mut operation = vec![0];
    operation.extend_from_slice(&(u32::from(n) + 1).to_le_bytes());
    let tx = shape(
        lab,
        1,
        &inputs,
        outputs,
        &[
            (0, operation),
            (
                usize::from(n) + 1,
                (u32::from(n) + 3).to_le_bytes().to_vec(),
            ),
        ],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/seal cannot disguise an unmetered anchor advance",
        &tx,
        "Inputs[0].Type",
        10,
    )?;
    seal(lab, &mut net, 1, "sealed/authenticated complete first seal")?;
    let first_snapshot = net.snapshot.clone().unwrap();
    let snapshot_view = rpc::call(
        "get_live_cell",
        json!([{"tx_hash":rpc::bytes_to_hex(&first_snapshot.0.tx_hash),"index":format!("0x{:x}",first_snapshot.0.index)},true]),
    )?;
    let snapshot_capacity = u64::from_str_radix(
        snapshot_view["cell"]["output"]["capacity"]
            .as_str()
            .ok_or("snapshot capacity")?
            .trim_start_matches("0x"),
        16,
    )
    .map_err(|e| e.to_string())?;
    let tx = shape(
        lab,
        0,
        &[(first_snapshot.0, snapshot_capacity)],
        vec![],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/published snapshot cannot be destroyed",
        &tx,
        "Inputs[0].Lock",
        1,
    )?;
    // A counterfeit snapshot published by anyone cannot replace the gate's hash.
    let mut wrong = first_snapshot.1.clone();
    wrong.epoch += 1;
    for lane in &mut wrong.lanes {
        lane.epoch = wrong.epoch;
    }
    let false_dep = lab.publish_cells(
        "sealed/publish unrelated snapshot bytes",
        &[wrong.encode().unwrap()],
        0,
        true,
    )?[0];
    let valid = required_bytes(&net)?;
    let mut fake = net.clone();
    fake.snapshot = Some((false_dep, wrong));
    let fake_tx = advance_tx(lab, &fake, 1, advance_outputs(&net, &valid)?)?;
    reject(
        lab,
        &code,
        "sealed/unrelated dependency is not the authenticated snapshot",
        &fake_tx,
        "Inputs[1].Type",
        9,
    )?;
    let omitted_bytes = bytes(&net, vec![])?;
    let skip = advance_tx(lab, &net, 1, advance_outputs(&net, &omitted_bytes)?)?;
    reject(
        lab,
        &code,
        "sealed/builder cannot skip mandatory prefix",
        &skip,
        "Inputs[1].Type",
        15,
    )?;
    let mut reversed = net
        .schedule
        .required(net.snapshot.as_ref().map(|(_, s)| s))
        .unwrap()
        .into_iter()
        .map(|m| m.payload)
        .collect::<Vec<_>>();
    reversed.swap(0, 1);
    let bad = bytes(&net, reversed)?;
    let reorder = advance_tx(lab, &net, 1, advance_outputs(&net, &bad)?)?;
    reject(
        lab,
        &code,
        "sealed/builder cannot reorder mandatory prefix",
        &reorder,
        "Inputs[1].Type",
        15,
    )?;
    let mut tx = advance_tx(lab, &net, 1, advance_outputs(&net, &valid)?)?;
    tx["witnesses"][1] = json!(rpc::bytes_to_hex(&vec![0; 4096]));
    reject(
        lab,
        &code,
        "sealed/witness allocation has an executable byte bound",
        &tx,
        "Inputs[1].Type",
        12,
    )?;
    let prepared = advance_tx(lab, &net, 1, advance_outputs(&net, &valid)?)?;
    let victim = vec![2, 0xfe, n, 7];
    let admitted_at = net.anchor.state.next_batch_number;
    for i in 0..usize::from(n) {
        for position in 0..s::MAX_LANE_MESSAGES {
            let payload = if i == usize::from(n) - 1 && position == s::MAX_LANE_MESSAGES - 1 {
                victim.clone()
            } else {
                vec![2, 0xfd, i as u8, position as u8]
            };
            append(
                lab,
                &mut net,
                i,
                0,
                payload,
                "sealed/churn active heads up to finite queue bound",
            )?;
        }
    }
    // Finite queues disallow another valid mutation; build an unchanged-head
    // transaction to ensure a no-op cannot sustain endless seal invalidation.
    let (full_cell, full_lane) = &net.lanes[0];
    let mut oversized = full_cell.output(full_lane.encode().unwrap());
    oversized.data[121] = 9;
    oversized.data.extend_from_slice(&[1, 0, 7]);
    let tx = shape(
        lab,
        0,
        &[(full_cell.point, full_cell.capacity)],
        vec![oversized],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/full lane refuses a ninth admission",
        &tx,
        "Inputs[0].Type",
        3,
    )?;
    let (c, lane) = &net.lanes[0];
    let noop = shape(
        lab,
        0,
        &[(c.point, c.capacity)],
        vec![c.output(lane.encode().unwrap())],
        &[],
        &[],
    )?;
    reject(
        lab,
        &code,
        "sealed/full lane cannot churn with no-op",
        &noop,
        "Inputs[0].Type",
        12,
    )?;
    accept_advance(
        lab,
        &mut net,
        1,
        &valid,
        &prepared,
        "sealed/pre-signed batch survives all active-lane churn",
    )?;
    engine.apply_batch(&valid).map_err(|e| e.to_string())?;
    live(first_snapshot.0, true)?;
    while net.schedule.batches < s::BATCHES_PER_EPOCH {
        advance(
            lab,
            &mut net,
            1,
            &mut engine,
            "sealed/process frozen epoch without changing active lanes",
        )?;
    }
    let raw = bytes(&net, vec![])?;
    let extend = advance_tx(lab, &net, 1, advance_outputs(&net, &raw)?)?;
    reject(
        lab,
        &code,
        "sealed/processed snapshot cannot be retained indefinitely",
        &extend,
        "Inputs[1].Type",
        14,
    )?;
    let stable = net.clone();
    let stable_engine = engine.clone();
    let wallets: Vec<_> = lab.wallets.iter().map(|w| (w.point, w.capacity)).collect();
    let parent = rpc::call("get_tip_header", json!([]))?;
    seal(lab, &mut net, 0, "sealed/planned orphan second seal")?;
    let orphan = net.snapshot.as_ref().unwrap().0;
    advance(
        lab,
        &mut net,
        1,
        &mut engine,
        "sealed/planned orphan first priority batch",
    )?;
    rpc::require_devnet()?;
    rpc::call("truncate", json!([parent["hash"]]))?;
    net = stable;
    engine = stable_engine;
    for (wallet, (point, capacity)) in lab.wallets.iter_mut().zip(wallets) {
        wallet.point = point;
        wallet.capacity = capacity;
    }
    for (c, _) in &net.lanes {
        live(c.point, true)?;
    }
    live(orphan, false)?;
    live(net.gate.point, true)?;
    seal(
        lab,
        &mut net,
        1,
        "sealed/other actor reseals restored active heads",
    )?;
    let second_snapshot = net.snapshot.clone().unwrap();
    let mut previous = net.clone();
    previous.snapshot = Some(first_snapshot.clone());
    let raw = required_bytes(&net)?;
    let outdated = advance_tx(lab, &previous, 1, advance_outputs(&net, &raw)?)?;
    reject(
        lab,
        &code,
        "sealed/old snapshot rejected after mandatory switch",
        &outdated,
        "Inputs[1].Type",
        9,
    )?;
    let mut processed_at = None;
    let mut observed = Vec::new();
    for _ in 0..s::BATCHES_PER_EPOCH {
        let duty = net
            .schedule
            .required(net.snapshot.as_ref().map(|(_, s)| s))
            .unwrap();
        if duty.iter().any(|m| m.payload == victim) {
            processed_at = Some(net.anchor.state.next_batch_number + 1);
        }
        observed.extend(duty.iter().map(|m| (m.lane, m.sequence)));
        advance(
            lab,
            &mut net,
            1,
            &mut engine,
            "sealed/mandatory processing of post-seal messages",
        )?;
    }
    let elapsed = processed_at.ok_or("post-seal victim never processed")? - admitted_at;
    let expected = second_snapshot
        .1
        .ordered()
        .unwrap()
        .iter()
        .map(|m| (m.lane, m.sequence))
        .collect::<Vec<_>>();
    if observed != expected || elapsed > 16 {
        return Err("mandatory processing bound/order violated".into());
    }
    live(first_snapshot.0, true)?;
    live(second_snapshot.0, true)?;
    if engine.anchor() != &net.anchor.state {
        return Err("execution/anchor diverged".into());
    }
    let mut maximum_payload_snapshot_bytes = None;
    if n == 4 {
        for index in 0..4 {
            for _ in 0..s::MAX_LANE_MESSAGES {
                append(
                    lab,
                    &mut net,
                    index,
                    0,
                    vec![0xff; s::MAX_PAYLOAD_BYTES],
                    "sealed/admit maximum-size active payload",
                )?;
            }
        }
        seal(
            lab,
            &mut net,
            1,
            "sealed/seal maximum 33385-byte authenticated snapshot",
        )?;
        let length = net.snapshot.as_ref().unwrap().1.encode().unwrap().len();
        if length != s::MAX_SNAPSHOT_BYTES {
            return Err("maximum snapshot shape mismatch".into());
        }
        maximum_payload_snapshot_bytes = Some(length);
        for _ in 0..s::BATCHES_PER_EPOCH {
            advance(
                lab,
                &mut net,
                1,
                &mut engine,
                "sealed/process maximum snapshot with bounded prefixes",
            )?;
        }
    }
    let result = json!({"lanes":n,"maximum_payload_snapshot_bytes":maximum_payload_snapshot_bytes,"canonical_batches":net.anchor.state.next_batch_number,"post_seal_victim_batches_to_inclusion":elapsed,"second_snapshot_messages":observed.len(),"consumed_snapshot_messages":net.schedule.cursor,"active_churn_mutations":usize::from(n)*s::MAX_LANE_MESSAGES,"snapshot_retention":true,"planned_seal_and_batch_reorg_recovered":true,"independent_sealer":true,"pre_signed_batch_survived_churn":true,"pre_signed_admission_survived_schedule_change":true,"execution_head":engine.head(),"G2":"OPEN","scope":"Mandatory publication under canonical batch progress; no wall-clock, admission fairness or validity-proof claim"});
    lab.evidence.push(json!({"label":format!("sealed/{n} lanes/verified outcome"),"result":"control_passed","outcome":result}));
    Ok(result)
}
fn run() -> Result<(), String> {
    let path = std::env::var("TACTUS_EVIDENCE_PATH").map_err(|_| "use isolated launcher")?;
    let mut lab = Lab::connect_with_script("artifacts/tactus_o1_anchor_script.elf")?;
    let mut results = json!({"suite":"a3-mandatory-sealed-gate-v1","complete":false,"production_ready":false,"G2":"OPEN","proof_settlement":"NOT_IMPLEMENTED"});
    let outcome = (|| {
        let elf =
            std::fs::read("artifacts/tactus_o1_sealed_script.elf").map_err(|e| e.to_string())?;
        let code = ckb_blake2b(&elf);
        lab.metadata["sealed_code_hash"] = json!(rpc::bytes_to_hex(&code));
        let dep = lab.publish_cells(
            "sealed/deploy immutable gate and lane program",
            &[elf],
            0,
            true,
        )?[0];
        lab.deps.push(dep);
        let mut scenarios = Vec::new();
        for n in [1, 2, 4] {
            scenarios.push(scenario(&mut lab, code, n)?);
        }
        results["scenarios"] = json!(scenarios);
        results["complete"] = json!(true);
        Ok::<_, String>(())
    })();
    results["error"] = json!(outcome.as_ref().err());
    lab.save(&path, results)?;
    outcome
}
fn main() {
    if let Err(error) = run() {
        eprintln!("SEALED EXPERIMENT FAILED: {error}");
        std::process::exit(1);
    }
}
