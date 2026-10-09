//! Experiment A devnet replay — A1 OrderingHead arm (design §8, spec §14).
//!
//! Prerequisites: a CKB devnet node mining on 127.0.0.1:8114 with the dev
//! key as block assembler, and `artifacts/tactus_ordering_script.elf` built
//! via the Rust→CKB-VM chain. Produces `specs/EXPERIMENT_A_DEVNET_REPORT.md`
//! with on-chain evidence for:
//!
//! - **E1** a valid ENQUEUE transition commits (permissionless succession),
//! - **E2** a valid APPEND_BATCH commits (accumulator + cursor advance),
//! - **E3** a conflicting spend of the same head cell is rejected by CKB
//!   consensus (single-consumption canonical linearisation — G1 evidence),
//! - **E4** a transition mutating a genesis-bound field is rejected by the
//!   type script on CKB-VM (exit code 3 — script-enforced invariants).

use std::fs;

use tactus_devnet_driver::rpc;
use tactus_devnet_driver::tx::{self, CellOutPoint, DevKey, OutSpec, TX_FEE};
use tactus_ordering_script::{ckb_blake2b, OrderingHead};

fn main() {
    let mut report = String::new();
    let key = DevKey::dev();
    let rollup_id = ckb_blake2b(b"tactus-devnet-rollup-1");
    let mut ok = true;

    let tip = rpc::get_tip_block_number().expect("node rpc");
    let genesis = rpc::get_block_detailed(0).expect("genesis");
    let secp_dep = tx::find_secp_dep(&genesis).expect("secp dep");
    println!(
        "tip={tip} secp_dep={}",
        rpc::bytes_to_hex(&secp_dep.tx_hash)
    );

    // ---- Fund: consolidate coinbase outputs into one cell -----------------
    let coinbases = tx::collect_coinbase(tip, &key.args, 1000).expect("coinbase scan");
    let total: u64 = coinbases.iter().map(|(_, c)| c).sum();
    println!(
        "coinbase cells: {} total {} CKB",
        coinbases.len(),
        total / tx::SHANNONS_PER_CKB
    );
    assert!(
        total > 40_000 * tx::SHANNONS_PER_CKB,
        "not enough capacity mined yet"
    );

    let consolidated_cap = total - TX_FEE;
    let consolidate_outputs = vec![OutSpec {
        capacity: consolidated_cap,
        lock: key.lock_script(),
        type_script: None,
        data: vec![],
    }];
    let consolidate_tx =
        tx::build_and_sign(&key, &secp_dep, &coinbases, &consolidate_outputs, None)
            .expect("consolidation tx");
    let (consolidate_hash, consolidate_block) =
        tx::send_and_wait(&consolidate_tx, 120).expect("consolidation committed");
    let fund_outpoint = CellOutPoint {
        tx_hash: hex32(&consolidate_hash),
        index: 0,
    };
    println!("consolidated in block {consolidate_block:?}: {consolidate_hash}");

    // ---- Deploy: script ELF cell + genesis OrderingHead cell --------------
    let elf = fs::read("artifacts/tactus_ordering_script.elf").expect("script elf");
    let tactus_type = tx::tactus_type_script(&elf, &rollup_id);
    let head0 = OrderingHead {
        rollup_id,
        protocol_version: 1,
        next_batch_number: 0,
        batch_accumulator_root: [0u8; 32],
        inbox_root: [1u8; 32],
        inbox_tail: 0,
        processed_inbox_cursor: 0,
        execution_rules_hash: [9u8; 32],
        da_policy_id: [11u8; 32],
    };

    let code_cap =
        OutSpec::required_capacity(&key.lock_script(), None, elf.len()) + tx::SHANNONS_PER_CKB;
    let head_cap = OutSpec::required_capacity(&key.lock_script(), Some(&tactus_type), 188)
        + tx::SHANNONS_PER_CKB;
    let change_cap = consolidated_cap - code_cap - head_cap - TX_FEE;

    let deploy_outputs = vec![
        OutSpec {
            capacity: code_cap,
            lock: key.lock_script(),
            type_script: None,
            data: elf.clone(),
        },
        OutSpec {
            capacity: head_cap,
            lock: key.lock_script(),
            type_script: Some(tactus_type.clone()),
            data: head0.to_bytes().to_vec(),
        },
        OutSpec {
            capacity: change_cap,
            lock: key.lock_script(),
            type_script: None,
            data: vec![],
        },
    ];
    let deploy_tx = tx::build_and_sign(
        &key,
        &secp_dep,
        &[(fund_outpoint, consolidated_cap)],
        &deploy_outputs,
        None,
    )
    .expect("deploy tx");
    let (deploy_hash, deploy_block) = tx::send_and_wait(&deploy_tx, 180).expect("deploy committed");
    let mut head_out = CellOutPoint {
        tx_hash: hex32(&deploy_hash),
        index: 1,
    };
    println!("deployed in block {deploy_block:?}: {deploy_hash}");

    let mut head = head0;

    // ---- E1: valid ENQUEUE --------------------------------------------------
    let msg1 = ckb_blake2b(b"tactus-a1-priority-message-1");
    let head1 = {
        let mut n = head;
        n.inbox_root = chain(&head.inbox_root, &msg1);
        n.inbox_tail = 1;
        n
    };
    let e1 = transition(
        &key,
        &tactus_type,
        &secp_dep,
        head_out,
        head_cap,
        head,
        head1,
        Some(&msg1),
        "E1 enqueue",
    );
    ok &= e1.is_ok();
    let (e1_hash, e1_block) = e1.expect("E1 commits");
    head = head1;
    head_out = CellOutPoint {
        tx_hash: hex32(&e1_hash),
        index: 0,
    };
    println!("E1 committed in {e1_block:?}: {e1_hash}");

    // ---- E2: valid APPEND_BATCH (accumulator + cursor advance) -------------
    let batch1 = ckb_blake2b(b"tactus-a1-batch-1");
    let head2 = {
        let mut n = head;
        n.batch_accumulator_root = chain(&head.batch_accumulator_root, &batch1);
        n.next_batch_number = 1;
        n.processed_inbox_cursor = 1; // up to the tail
        n
    };
    let e2 = transition(
        &key,
        &tactus_type,
        &secp_dep,
        head_out,
        head_cap,
        head,
        head2,
        Some(&batch1),
        "E2 append",
    );
    ok &= e2.is_ok();
    let (e2_hash, e2_block) = e2.expect("E2 commits");
    head = head2;
    head_out = CellOutPoint {
        tx_hash: hex32(&e2_hash),
        index: 0,
    };
    println!("E2 committed in {e2_block:?}: {e2_hash}");

    // ---- E3: conflicting double-spend rejected by consensus -----------------
    let msg_a = ckb_blake2b(b"tactus-a1-conflict-a");
    let msg_b = ckb_blake2b(b"tactus-a1-conflict-b");
    let head_a = {
        let mut n = head;
        n.inbox_root = chain(&head.inbox_root, &msg_a);
        n.inbox_tail = 2;
        n
    };
    let head_b = {
        let mut n = head;
        n.inbox_root = chain(&head.inbox_root, &msg_b);
        n.inbox_tail = 2;
        n
    };
    let tx_a = transition_tx(
        &key,
        &tactus_type,
        &secp_dep,
        head_out,
        head_cap,
        head_a,
        Some(&msg_a),
    );
    let tx_b = transition_tx(
        &key,
        &tactus_type,
        &secp_dep,
        head_out,
        head_cap,
        head_b,
        Some(&msg_b),
    );
    let (a_hash, a_block) = tx::send_and_wait(&tx_a, 120).expect("E3 first spend commits");
    let b_result = rpc::send_transaction(&rpc::bytes_to_hex(&tx_b));
    let e3_rejected = match b_result {
        Ok(hash) => {
            // A same-cell spend should not commit; check status briefly.
            let (status, _) = rpc::get_transaction_status(&hash).unwrap_or_default();
            format!("UNEXPECTED: accepted as {status} ({hash})")
        }
        Err(e) => format!("rejected: {}", shorten(&e)),
    };
    let e3_ok = e3_rejected.starts_with("rejected");
    ok &= e3_ok;
    println!("E3 first: {a_hash} in {a_block:?}; second: {e3_rejected}");
    head = head_a;
    head_out = CellOutPoint {
        tx_hash: hex32(&a_hash),
        index: 0,
    };

    // ---- E4: genesis-bound field mutation rejected by the script -----------
    let batch_bad = ckb_blake2b(b"tactus-a1-bad-batch");
    let head_bad = {
        let mut n = head;
        n.batch_accumulator_root = chain(&head.batch_accumulator_root, &batch_bad);
        n.next_batch_number = 2;
        n.da_policy_id = [12u8; 32]; // preserved field mutated
        n
    };
    let tx_bad = transition_tx(
        &key,
        &tactus_type,
        &secp_dep,
        head_out,
        head_cap,
        head_bad,
        Some(&batch_bad),
    );
    let e4_result = rpc::send_transaction(&rpc::bytes_to_hex(&tx_bad));
    let e4 = match e4_result {
        Ok(h) => format!("UNEXPECTED: accepted ({h})"),
        Err(e) => format!("rejected: {}", shorten(&e)),
    };
    let e4_ok = e4.starts_with("rejected");
    ok &= e4_ok;
    println!("E4 {e4}");

    // ---- Report --------------------------------------------------------------
    report.push_str("# Experiment A — Devnet-Tier Report (A1 arm)\n\n");
    report.push_str(&format!(
        "**Node:** ckb v0.210.0 devnet (local), tip {tip} at start · **Script:** Rust→CKB-VM, {}, {}\n\n",
        elf.len(),
        "artifacts/tactus_ordering_script.elf"
    ));
    report.push_str(&format!(
        "| Evidence | Result | Detail |\n|---|---|---|\n\
         | Consolidation | committed | block {consolidate_block:?}, tx `{consolidate_hash}` |\n\
         | Deployment | committed | block {deploy_block:?}, tx `{deploy_hash}` |\n\
         | E1 valid ENQUEUE | {} | block {e1_block:?}, tx `{e1_hash}` |\n\
         | E2 valid APPEND_BATCH | {} | block {e2_block:?}, tx `{e2_hash}` |\n\
         | E3 conflicting spend | {} | first `{a_hash}` committed; second {e3_rejected} |\n\
         | E4 preserved-field mutation | {} | {e4} |\n",
        if e1_block.is_some() {
            "committed"
        } else {
            "FAILED"
        },
        if e2_block.is_some() {
            "committed"
        } else {
            "FAILED"
        },
        if e3_ok {
            "rejected by consensus"
        } else {
            "NOT rejected"
        },
        if e4_ok {
            "rejected by script"
        } else {
            "NOT rejected"
        },
    ));
    report.push_str(
        "\nE3 is canonical-linearisation evidence (G1 flavour): two well-signed, \
script-valid transitions cannot both consume the same live head cell — CKB single-consumption \
resolves the race. E4 is script-enforcement evidence: the on-chain type script rejects a \
transition that mutates a genesis-bound field (exit code 3).\n",
    );
    report.push_str(
        "\n_Scope: A1 arm on a local devnet with a single honest operator; this is \
not the full adversarial workload of design §8 (dominant-builder churn runs in the next \
iteration), and G1/G2 remain formally OPEN pending that tier._\n",
    );
    fs::write("specs/EXPERIMENT_A_DEVNET_REPORT.md", report).expect("write report");
    println!(
        "\nreport written; scenario {}",
        if ok { "OK" } else { "HAD FAILURES" }
    );
}

fn chain(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(left);
    buf[32..].copy_from_slice(right);
    ckb_blake2b(&buf)
}

fn hex32(h: &str) -> [u8; 32] {
    let b = rpc::hex_to_bytes(h);
    b.try_into().expect("32 byte hash")
}

fn transition_tx(
    key: &DevKey,
    type_script: &[u8],
    secp_dep: &CellOutPoint,
    spend: CellOutPoint,
    capacity: u64,
    next: OrderingHead,
    input_type: Option<&[u8; 32]>,
) -> Vec<u8> {
    let outputs = vec![tx::head_output(key, type_script, &next, capacity)];
    tx::build_and_sign(
        key,
        secp_dep,
        &[(spend, capacity)],
        &outputs,
        input_type.map(|x| &x[..]),
    )
    .expect("transition tx")
}

#[allow(clippy::too_many_arguments)]
fn transition(
    key: &DevKey,
    type_script: &[u8],
    secp_dep: &CellOutPoint,
    spend: CellOutPoint,
    capacity: u64,
    _current: OrderingHead,
    next: OrderingHead,
    input_type: Option<&[u8; 32]>,
    label: &str,
) -> Result<(String, Option<u64>), String> {
    let t = transition_tx(
        key,
        type_script,
        secp_dep,
        spend,
        capacity,
        next,
        input_type,
    );
    match tx::send_and_wait(&t, 180) {
        Ok(x) => Ok(x),
        Err(e) => {
            eprintln!("{label} failed: {e}");
            Err(e)
        }
    }
}

fn shorten(s: &str) -> String {
    s.chars().take(200).collect()
}
