//! Read-only canonical custody reconstruction. The selected CKB node supplies
//! consensus/script validity; this is not a CKB light client or an L2 credit.
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tactus_o1_devnet_driver::{molecule, rpc};
use tactus_o1_native_vault_script::{
    hash, Config, Record, Release, State, MAX_CELLS, RECORD_BYTES, STATE_BYTES,
};
const HEAD_LOCK: &str = "0x1a79ae4b82f5588e07fc0b94a0e8fbcf61f1adc06d05d753d13e2e63eec76dee";
#[derive(Clone, Copy)]
pub struct Limits {
    pub blocks: u64,
    pub deposits: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            blocks: 100_000,
            deposits: 4096,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), String> {
        if self.blocks == 0
            || self.blocks > 1_000_000
            || self.deposits == 0
            || self.deposits > 65_536
        {
            Err("recovery limits out of range".into())
        } else {
            Ok(())
        }
    }
}
pub fn number(value: &Value) -> Result<u64, String> {
    u64::from_str_radix(
        value
            .as_str()
            .ok_or("missing number")?
            .strip_prefix("0x")
            .ok_or("number prefix")?,
        16,
    )
    .map_err(|_| "invalid number".into())
}
fn bytes(value: &Value, max: usize) -> Result<Vec<u8>, String> {
    let value = value.as_str().ok_or("missing hex")?;
    if value.len() > 2 + 2 * max || !value.starts_with("0x") {
        return Err("hex size/prefix".into());
    }
    rpc::decode_hex(value)
}
fn hash32(value: &Value) -> Result<[u8; 32], String> {
    bytes(value, 32)?.try_into().map_err(|_| "hash size".into())
}
fn script(value: &Value) -> Result<Vec<u8>, String> {
    let tag = match value["hash_type"].as_str() {
        Some("data") => 0,
        Some("type") => 1,
        Some("data1") => 2,
        Some("data2") => 4,
        _ => return Err("script hash type".into()),
    };
    Ok(molecule::script(
        &hash32(&value["code_hash"])?,
        tag,
        &bytes(&value["args"], 16 * 1024 * 1024)?,
    ))
}
fn key(point: &Value) -> Result<(String, u32), String> {
    let h = rpc::bytes_to_hex(&hash32(&point["tx_hash"])?);
    let i = u32::try_from(number(&point["index"])?).map_err(|_| "outpoint index")?;
    Ok((h, i))
}
fn point(tx: &Value, index: usize) -> Result<Value, String> {
    Ok(json!({"tx_hash":rpc::bytes_to_hex(&hash32(&tx["hash"])?),"index":format!("0x{index:x}")}))
}
fn state(output: &Value, data: &Value) -> Result<State, String> {
    let s = State::decode(&bytes(data, STATE_BYTES)?).map_err(|e| format!("vault state {e}"))?;
    if number(&output["capacity"])? != s.capacity().map_err(|e| format!("capacity {e}"))? {
        return Err("vault capacity differs from state".into());
    }
    Ok(s)
}
fn witness(tx: &Value, index: usize) -> Result<Vec<u8>, String> {
    let raw = bytes(&tx["witnesses"][index], 140_000)?;
    if raw.len() < 16 {
        return Err("short WitnessArgs".into());
    }
    let words: Vec<_> = (0..4)
        .map(|i| u32::from_le_bytes(raw[4 * i..4 * i + 4].try_into().unwrap()) as usize)
        .collect();
    if words[0] != raw.len()
        || words[1] != 16
        || words[1] > words[2]
        || words[2] > words[3]
        || words[3] > raw.len()
    {
        return Err("WitnessArgs offsets".into());
    }
    let b = &raw[words[2]..words[3]];
    if b.len() < 4 || u32::from_le_bytes(b[..4].try_into().unwrap()) as usize != b.len() - 4 {
        return Err("input_type length".into());
    }
    Ok(b[4..].to_vec())
}
#[derive(Clone)]
pub struct Cell {
    pub output: Value,
    pub data: Value,
}
struct Current {
    point: Value,
    state: State,
    output: Value,
}
pub struct Tracker {
    failed: bool,
    cfg: Config,
    expected: Value,
    receipt: Value,
    immutable: Value,
    lock: Value,
    limits: Limits,
    previous: Option<String>,
    height: u64,
    current: Option<Current>,
    records: Vec<Value>,
    receipt_points: BTreeSet<(String, u32)>,
    releases: u64,
    genesis: Option<Value>,
}
impl Tracker {
    pub fn new(
        expected_genesis: &str,
        vault_script: &[u8],
        limits: Limits,
    ) -> Result<Self, String> {
        limits.validate()?;
        let expected = molecule::try_script_to_json(vault_script)?;
        let cfg =
            Config::decode(&bytes(&expected["args"], 164)?).map_err(|e| format!("config {e}"))?;
        if expected["hash_type"] != "data1"
            || rpc::bytes_to_hex(&cfg.ckb_genesis) != expected_genesis
        {
            return Err("trusted vault/chain binding mismatch".into());
        }
        let identity = hash(b"", vault_script);
        let mut receipt = expected.clone();
        receipt["args"] = json!(rpc::bytes_to_hex(
            &[b"TO1REC01".as_slice(), &identity].concat()
        ));
        let mut immutable = expected.clone();
        immutable["args"] = json!("0x");
        let lock =
            json!({"code_hash":HEAD_LOCK,"hash_type":"data1","args":rpc::bytes_to_hex(&identity)});
        Ok(Self {
            failed: false,
            cfg,
            expected,
            receipt,
            immutable,
            lock,
            limits,
            previous: None,
            height: 0,
            current: None,
            records: vec![],
            receipt_points: BTreeSet::new(),
            releases: 0,
            genesis: None,
        })
    }
    pub fn apply(
        &mut self,
        block: &Value,
        resolve: &mut impl FnMut(&Value) -> Result<Cell, String>,
    ) -> Result<(), String> {
        if self.failed {
            return Err("recovery tracker is invalid after an error".into());
        }
        self.failed = true;
        let result = self.apply_inner(block, resolve);
        if result.is_ok() {
            self.failed = false;
        }
        result
    }
    fn apply_inner(
        &mut self,
        block: &Value,
        resolve: &mut impl FnMut(&Value) -> Result<Cell, String>,
    ) -> Result<(), String> {
        if self.height >= self.limits.blocks || number(&block["header"]["number"])? != self.height {
            return Err("scan height/limit".into());
        }
        let block_hash = rpc::bytes_to_hex(&hash32(&block["header"]["hash"])?);
        if self.height == 0 && block_hash != rpc::bytes_to_hex(&self.cfg.ckb_genesis) {
            return Err("wrong genesis block".into());
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|v| block["header"]["parent_hash"] != *v)
        {
            return Err("disconnected canonical blocks".into());
        }
        for (tx_index, tx) in block["transactions"]
            .as_array()
            .ok_or("block transactions")?
            .iter()
            .enumerate()
        {
            let outputs = tx["outputs"].as_array().ok_or("outputs")?;
            let inputs = tx["inputs"].as_array().ok_or("inputs")?;
            let data = tx["outputs_data"].as_array().ok_or("outputs data")?;
            if outputs.len() != data.len() {
                return Err("output/data count".into());
            }
            for input in inputs {
                if self
                    .receipt_points
                    .contains(&key(&input["previous_output"])?)
                {
                    return Err("immutable deposit receipt consumed".into());
                }
            }
            let matches: Vec<_> = outputs
                .iter()
                .enumerate()
                .filter(|(_, o)| o["type"] == self.expected)
                .collect();
            let receipts: Vec<_> = outputs
                .iter()
                .enumerate()
                .filter(|(_, o)| o["type"] == self.receipt)
                .collect();
            let consumed: Vec<_> = if let Some(current) = &self.current {
                inputs
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i["previous_output"] == current.point)
                    .map(|(i, _)| i)
                    .collect()
            } else {
                vec![]
            };
            if matches.is_empty() && consumed.is_empty() {
                if !receipts.is_empty() {
                    return Err("receipt without canonical vault transition".into());
                }
                continue;
            }
            if matches.len() != 1 || outputs.len() > MAX_CELLS || inputs.len() > MAX_CELLS {
                return Err("vault transaction shape".into());
            }
            let (index, output) = matches[0];
            if output["lock"] != self.lock {
                return Err("custody lock changed".into());
            }
            let next = state(output, &data[index])?;
            let next_point = point(tx, index)?;
            if self.current.is_none() {
                if !receipts.is_empty() {
                    return Err("deposit receipt at genesis".into());
                }
                let input = inputs.first().ok_or("genesis funding input")?;
                let p = key(&input["previous_output"])?;
                let seed = molecule::cell_input(
                    number(&input["since"])?,
                    &molecule::out_point(&hash32(&json!(p.0))?, p.1),
                );
                if hash(
                    b"",
                    &[seed.as_slice(), &(index as u64).to_le_bytes()].concat(),
                ) != self.cfg.identity
                {
                    return Err("forged vault genesis".into());
                }
                let reserve = (8
                    + 33
                    + bytes(&self.lock["args"], 32)?.len()
                    + 33
                    + bytes(&self.expected["args"], 164)?.len()
                    + STATE_BYTES) as u64
                    * 100_000_000;
                if next != State::genesis(&self.cfg, reserve) {
                    return Err("nonempty/incorrect genesis reserve".into());
                }
                self.genesis =
                    Some(json!({"point":next_point,"block_hash":block_hash,"height":self.height}));
            } else {
                if consumed.len() != 1 {
                    return Err("vault successor does not consume canonical predecessor".into());
                }
                for (i, input) in inputs.iter().enumerate() {
                    if i != consumed[0]
                        && !resolve(&input["previous_output"])?.output["type"].is_null()
                    {
                        return Err("another typed custody input".into());
                    }
                }
                let current = &self.current.as_ref().unwrap().state;
                if next.count > current.count {
                    if receipts.len() != 1 || self.records.len() >= self.limits.deposits {
                        return Err("receipt count/record limit".into());
                    }
                    let (ri, ro) = receipts[0];
                    let raw = bytes(&data[ri], RECORD_BYTES)?;
                    let record = Record::decode(&raw).map_err(|e| format!("record {e}"))?;
                    let (expected, expected_record) = current
                        .deposit(&self.cfg, record.recipient, record.amount)
                        .map_err(|e| format!("funded deposit {e}"))?;
                    if next != expected || record != expected_record || ro["lock"] != self.immutable
                    {
                        return Err("deposit transcript/funding differs".into());
                    }
                    let record_point = point(tx, ri)?;
                    if !self.receipt_points.insert(key(&record_point)?) {
                        return Err("duplicate receipt outpoint".into());
                    }
                    self.records.push(json!({"sequence":record.sequence,"deposit_id":rpc::bytes_to_hex(&self.cfg.deposit_id(record.sequence)),"recipient":rpc::bytes_to_hex(&record.recipient),"amount_shannons":record.amount.to_string(),"cumulative_shannons":record.cumulative.to_string(),"record":rpc::bytes_to_hex(&raw),"point":record_point,"output":ro,"height":self.height,"block_hash":block_hash,"transaction_index":tx_index}));
                } else {
                    if !receipts.is_empty() {
                        return Err("receipt on withdrawal".into());
                    }
                    let release = Release::decode(&witness(tx, consumed[0])?)
                        .map_err(|e| format!("release wire {e}"))?;
                    let deps = tx["cell_deps"].as_array().ok_or("cell deps")?;
                    if deps.len() > MAX_CELLS {
                        return Err("dependency limit".into());
                    }
                    let mut tip = None;
                    for dep in deps {
                        if dep["dep_type"] != "code" {
                            continue;
                        }
                        let cell = resolve(&dep["out_point"])?;
                        if !cell.output["type"].is_null()
                            && hash(b"", &script(&cell.output["type"])?) == self.cfg.settlement
                        {
                            if tip.is_some() {
                                return Err("duplicate settlement Tip".into());
                            }
                            tip = Some(bytes(&cell.data, 280)?);
                        }
                    }
                    let expected = release
                        .verify(
                            &self.cfg,
                            current,
                            &tip.ok_or("missing authenticated settlement Tip")?,
                        )
                        .map_err(|e| format!("release verification {e}"))?;
                    let payout = outputs
                        .get(release.payout as usize)
                        .ok_or("missing payout")?;
                    if next != expected
                        || !payout["type"].is_null()
                        || data[release.payout as usize] != "0x"
                        || number(&payout["capacity"])? < release.amount
                        || hash(b"", &script(&payout["lock"])?) != release.recipient
                    {
                        return Err("release/payment mismatch".into());
                    }
                    self.releases = self.releases.checked_add(1).ok_or("release count")?;
                }
            }
            self.current = Some(Current {
                point: next_point,
                state: next,
                output: output.clone(),
            });
        }
        self.previous = Some(block_hash);
        self.height += 1;
        Ok(())
    }
    pub fn report(&self) -> Result<Value, String> {
        if self.failed {
            return Err("cannot report invalid recovery state".into());
        }
        let c = self
            .current
            .as_ref()
            .ok_or("canonical vault genesis not found")?;
        if self.records.len() as u64 != c.state.count {
            return Err("incomplete deposit sequence".into());
        }
        Ok(
            json!({"schema":1,"kind":"canonical-native-vault-v1","ckb_genesis":rpc::bytes_to_hex(&self.cfg.ckb_genesis),"vault_script":rpc::bytes_to_hex(&script(&self.expected)?),"pinned_height":self.height.checked_sub(1).ok_or("empty scan")?,"pinned_hash":self.previous,"genesis":self.genesis,"point":c.point,"output":c.output,"state":rpc::bytes_to_hex(&c.state.encode()),"deposit_count":c.state.count,"deposited_shannons":c.state.deposited.to_string(),"released_shannons":c.state.released.to_string(),"reserve_shannons":c.state.reserve.to_string(),"capacity_shannons":c.state.capacity().map_err(|e|format!("capacity {e}"))?.to_string(),"release_transitions":self.releases,"deposits":self.records,"authenticated_l2_credit":false,"production_ready":false,"consensus_source":"selected CKB node; no independent consensus or Groth16 verification"}),
        )
    }
}

pub fn recover(
    address: &str,
    genesis: &str,
    vault_script: &[u8],
    limits: Limits,
    mut capture: impl FnMut(&Value) -> Result<(), String>,
) -> Result<Value, String> {
    let mut tracker = Tracker::new(genesis, vault_script, limits)?;
    let pin = rpc::call_at(address, "get_tip_header", json!([]))?;
    let height = number(&pin["number"])?;
    if height >= limits.blocks {
        return Err("canonical height exceeds scan limit".into());
    }
    if rpc::call_at(address, "get_block_hash", json!(["0x0"]))? != genesis {
        return Err("RPC network mismatch".into());
    }
    let mut resolve = |point: &Value| -> Result<Cell, String> {
        let (hash, index) = key(point)?;
        let view = rpc::call_at(address, "get_transaction", json!([hash]))?;
        if view["tx_status"]["status"] != "committed" || view["transaction"]["hash"] != hash {
            return Err("unavailable canonical dependency".into());
        }
        let tx = &view["transaction"];
        let output = tx["outputs"]
            .as_array()
            .and_then(|o| o.get(index as usize))
            .ok_or("dependency output")?
            .clone();
        let data = tx["outputs_data"]
            .as_array()
            .and_then(|o| o.get(index as usize))
            .ok_or("dependency data")?
            .clone();
        Ok(Cell { output, data })
    };
    for number in 0..=height {
        let block = rpc::call_at(
            address,
            "get_block_by_number",
            json!([format!("0x{number:x}"), "0x2"]),
        )?;
        tracker.apply(&block, &mut resolve)?;
        capture(&block)?;
    }
    let mut report = tracker.report()?;
    if report["pinned_hash"] != pin["hash"] {
        return Err("pinned header differs from scanned history".into());
    }
    let live = rpc::call_at(address, "get_live_cell", json!([report["point"], true]))?;
    if live["status"] != "live"
        || live["cell"]["output"] != report["output"]
        || live["cell"]["data"]["content"] != report["state"]
    {
        return Err("vault changed during recovery; retry".into());
    }
    for row in report["deposits"].as_array().unwrap() {
        let live = rpc::call_at(address, "get_live_cell", json!([row["point"], true]))?;
        if live["status"] != "live"
            || live["cell"]["output"] != row["output"]
            || live["cell"]["data"]["content"] != row["record"]
        {
            return Err("immutable receipt unavailable or changed".into());
        }
    }
    if rpc::call_at(address, "get_block_hash", json!([format!("0x{height:x}")]))? != pin["hash"] {
        return Err("canonical pin invalidated during recovery".into());
    }
    report["canonical_pin_rechecked"] = json!(true);
    report["current_live_rechecked"] = json!(true);
    Ok(report)
}
