use alloy_consensus::{transaction::SignerRecoverable, Block, BlockBody, Transaction, TxEnvelope};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::{Address, B256, U256};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Instant};
use tactus_o1_devnet_driver::{recovery, rpc, settlement_recovery};
use tactus_o1_execution::{Executor, Genesis};

pub struct Snapshot {
    pub created: Instant,
    pub height: u64,
    pub pin: String,
    pub chain_id: u64,
    pub status: Value,
    pub head: u64,
    blocks: BTreeMap<u64, Value>,
    transactions: BTreeMap<B256, Value>,
    receipts: BTreeMap<B256, Value>,
    genesis: Executor,
    latest: Executor,
}
impl Snapshot {
    pub fn recover(config: &crate::Config) -> Result<Self, String> {
        let anchor = rpc::decode_hex(&config.anchor_type_script)?;
        let tip = rpc::decode_hex(&config.settlement_type_script)?;
        let (settlement, published) =
            settlement_recovery::recover_snapshot(&config.ckb_genesis, &anchor, &tip)?;
        if published.batches.len() > config.max_batches {
            return Err("configured publication replay limit exceeded".into());
        }
        let mut snapshot = Self::replay(settlement, published)?;
        recovery::assert_canonical(snapshot.height, &snapshot.pin)?;
        snapshot.created = Instant::now();
        Ok(snapshot)
    }
    fn replay(settlement: Value, published: recovery::RecoveredAnchor) -> Result<Self, String> {
        let genesis = Genesis::from_allocation(
            published.genesis.rollup_id.into(),
            published.genesis.chain_id,
            &published.genesis_allocation,
        )
        .map_err(|e| e.to_string())?;
        let mut engine = Executor::new(&genesis).map_err(|e| e.to_string())?;
        let initial = engine.clone();
        let mut blocks = BTreeMap::new();
        let mut first = json!(engine.head());
        first["hash"] = json!(engine.head().hash_slow());
        first["transactions"] = json!([]);
        first["uncles"] = json!([]);
        first["withdrawals"] = json!([]);
        first["size"] = json!(quantity(Block::<TxEnvelope>::rlp_length_for(
            engine.head(),
            &BlockBody {
                transactions: vec![],
                ommers: vec![],
                withdrawals: Some(Default::default())
            }
        ) as u64));
        blocks.insert(0, first);
        let mut transactions = BTreeMap::new();
        let mut receipts = BTreeMap::new();
        for batch in &published.batches {
            for block in engine
                .apply_batch(&batch.input_bytes)
                .map_err(|e| e.to_string())?
            {
                let number = block.header.number;
                let body = BlockBody {
                    transactions: block
                        .transactions
                        .iter()
                        .map(|wire| TxEnvelope::decode_2718_exact(wire).map_err(|e| e.to_string()))
                        .collect::<Result<Vec<_>, _>>()?,
                    ommers: vec![],
                    withdrawals: Some(Default::default()),
                };
                let mut view = json!(block.header);
                view["hash"] = json!(block.hash);
                view["size"] = json!(quantity(Block::<TxEnvelope>::rlp_length_for(
                    &block.header,
                    &body
                ) as u64));
                view["uncles"] = json!([]);
                view["withdrawals"] = json!([]);
                let mut hashes = Vec::new();
                let mut log_index = 0u64;
                for (index, (wire, receipt)) in
                    block.transactions.iter().zip(&block.receipts).enumerate()
                {
                    let tx = TxEnvelope::decode_2718_exact(wire).map_err(|e| e.to_string())?;
                    let hash = alloy_primitives::keccak256(wire);
                    let sender = tx.recover_signer().map_err(|e| e.to_string())?;
                    let outcome = block
                        .outcomes
                        .iter()
                        .find(|o| o.transaction_index == Some(index as u32))
                        .ok_or("missing execution outcome")?;
                    let mut tx_view = json!(tx);
                    tx_view["hash"] = json!(hash);
                    tx_view["gasPrice"] = json!(format!(
                        "0x{:x}",
                        tx.effective_gas_price(block.header.base_fee_per_gas)
                    ));
                    tx_view["from"] = json!(sender);
                    tx_view["blockHash"] = json!(block.hash);
                    tx_view["blockNumber"] = json!(quantity(number));
                    tx_view["transactionIndex"] = json!(quantity(index as u64));
                    let mut receipt_view = json!(receipt);
                    receipt_view["transactionHash"] = json!(hash);
                    receipt_view["transactionIndex"] = json!(quantity(index as u64));
                    receipt_view["blockHash"] = json!(block.hash);
                    receipt_view["blockNumber"] = json!(quantity(number));
                    receipt_view["from"] = json!(sender);
                    receipt_view["to"] = tx_view["to"].clone();
                    receipt_view["contractAddress"] = json!(outcome.created_address);
                    receipt_view["gasUsed"] = json!(quantity(outcome.gas_used));
                    receipt_view["effectiveGasPrice"] = json!(format!(
                        "0x{:x}",
                        tx.effective_gas_price(block.header.base_fee_per_gas)
                    ));
                    for log in receipt_view["logs"].as_array_mut().ok_or("receipt logs")? {
                        log["blockHash"] = json!(block.hash);
                        log["blockNumber"] = json!(quantity(number));
                        log["blockTimestamp"] = json!(quantity(block.header.timestamp));
                        log["transactionHash"] = json!(hash);
                        log["transactionIndex"] = json!(quantity(index as u64));
                        log["logIndex"] = json!(quantity(log_index));
                        log["removed"] = false.into();
                        log_index += 1;
                    }
                    hashes.push(hash);
                    transactions.insert(hash, tx_view);
                    receipts.insert(hash, receipt_view);
                }
                view["transactions"] = json!(hashes);
                blocks.insert(number, view);
            }
        }
        let head = engine.head().number;
        Ok(Self {
            created: Instant::now(),
            height: published.pinned_height,
            pin: published.pinned_hash.clone(),
            chain_id: genesis.chain_id,
            status: json!({"ckbGenesis":settlement["ckb_genesis"],"ckbHeight":quantity(published.pinned_height),"ckbHash":published.pinned_hash,"publishedBatches":quantity(published.batches.len() as u64),"provedBatches":quantity(settlement["settled_batches"].as_u64().ok_or("settled count")?),"settlementTip":settlement["tip"],"latestIsProofSettled":settlement["settled_batches"].as_u64()==Some(published.batches.len() as u64),"safeFinalizedPolicy":null,"withdrawalAuthority":false,"productionReady":false}),
            head,
            blocks,
            transactions,
            receipts,
            genesis: initial,
            latest: engine,
        })
    }
    fn logs(&self, value: &Value) -> Result<Value, crate::RpcError> {
        use crate::{logs, RpcError};
        let criteria = logs::Criteria::parse(value)?;
        let field = |key| value.get(key).filter(|v| !v.is_null());
        let (start, end) = if let Some(hash) = field("blockHash") {
            if field("fromBlock").is_some() || field("toBlock").is_some() {
                return Err(RpcError(
                    -32602,
                    "blockHash cannot be combined with a block range",
                ));
            }
            let hash = parse_hash(hash)?;
            let number = self
                .blocks
                .iter()
                .find(|(_, b)| b["hash"] == json!(hash))
                .map(|(n, _)| *n)
                .ok_or(RpcError(-32000, "unknown canonical block"))?;
            (number, number)
        } else {
            let start = field("fromBlock")
                .map(|v| self.number(v))
                .transpose()?
                .unwrap_or(self.head);
            let end = field("toBlock")
                .map(|v| self.number(v))
                .transpose()?
                .unwrap_or(self.head);
            if start > end || end > self.head {
                return Err(RpcError(-32602, "invalid or future log range"));
            }
            if end - start >= logs::MAX_BLOCKS {
                return Err(RpcError(-32005, "log range exceeds 1024 blocks; paginate"));
            }
            (start, end)
        };
        let mut result = Vec::new();
        let mut bytes = 2usize;
        for (_, block) in self.blocks.range(start..=end) {
            for hash in block["transactions"]
                .as_array()
                .ok_or(RpcError(-32603, "block transactions"))?
            {
                let receipt = self
                    .receipts
                    .get(&parse_hash(hash)?)
                    .ok_or(RpcError(-32603, "missing receipt"))?;
                for log in receipt["logs"]
                    .as_array()
                    .ok_or(RpcError(-32603, "receipt logs"))?
                {
                    if criteria.matches(log) {
                        bytes += serde_json::to_vec(log)
                            .map_err(|_| RpcError(-32603, "log serialization"))?
                            .len()
                            + 1;
                        if result.len() == logs::MAX_LOGS || bytes > logs::MAX_BYTES {
                            return Err(RpcError(
                                -32005,
                                "log results exceed limit; narrow the filter",
                            ));
                        }
                        result.push(log.clone());
                    }
                }
            }
        }
        Ok(json!(result))
    }
    fn number(&self, value: &Value) -> Result<u64, crate::RpcError> {
        match value.as_str() {
            Some("latest") => Ok(self.head),
            Some("earliest") => Ok(0),
            Some("safe" | "finalized" | "pending") => Err(crate::RpcError(
                -32000,
                "requested block tag has no defined observer policy",
            )),
            Some(s) => parse_quantity(s),
            None => Err(crate::RpcError(-32602, "block tag must be a string")),
        }
    }
    pub fn query(&self, method: &str, params: &[Value]) -> Result<Value, crate::RpcError> {
        use crate::RpcError;
        let arity = |n| {
            if params.len() == n {
                Ok(())
            } else {
                Err(RpcError(-32602, "wrong parameter count"))
            }
        };
        match method {
            "web3_clientVersion" => {
                arity(0)?;
                Ok(json!("tactus-o1-observer/0.1.0"))
            }
            "eth_chainId" => {
                arity(0)?;
                Ok(json!(quantity(self.chain_id)))
            }
            "net_version" => {
                arity(0)?;
                Ok(json!(self.chain_id.to_string()))
            }
            "eth_blockNumber" => {
                arity(0)?;
                Ok(json!(quantity(self.head)))
            }
            "tactus_getStatus" => {
                arity(0)?;
                Ok(self.status.clone())
            }
            "eth_getBlockByNumber" | "eth_getBlockByHash" => {
                arity(2)?;
                let full = params[1]
                    .as_bool()
                    .ok_or(RpcError(-32602, "transaction flag must be boolean"))?;
                let block = if method == "eth_getBlockByNumber" {
                    self.blocks.get(&self.number(&params[0])?)
                } else {
                    let hash = parse_hash(&params[0])?;
                    self.blocks.values().find(|v| v["hash"] == json!(hash))
                };
                let Some(block) = block else {
                    return Ok(Value::Null);
                };
                let mut result = block.clone();
                if full {
                    result["transactions"] = json!(block["transactions"]
                        .as_array()
                        .ok_or(RpcError(-32603, "block transactions"))?
                        .iter()
                        .map(|hash| self
                            .transactions
                            .get(&parse_hash(hash).expect("internally encoded hash"))
                            .expect("indexed transaction"))
                        .collect::<Vec<_>>());
                }
                Ok(result)
            }
            "eth_getLogs" => {
                arity(1)?;
                self.logs(&params[0])
            }
            "eth_getTransactionByHash" | "eth_getTransactionReceipt" => {
                arity(1)?;
                let hash = parse_hash(&params[0])?;
                Ok(if method == "eth_getTransactionByHash" {
                    self.transactions.get(&hash)
                } else {
                    self.receipts.get(&hash)
                }
                .cloned()
                .unwrap_or(Value::Null))
            }
            "eth_getBalance" | "eth_getTransactionCount" | "eth_getCode" | "eth_getStorageAt" => {
                let storage = method == "eth_getStorageAt";
                arity(if storage { 3 } else { 2 })?;
                let address = params[0]
                    .as_str()
                    .filter(|s| s.starts_with("0x") && s.len() == 42)
                    .and_then(|s| s.parse::<Address>().ok())
                    .ok_or(RpcError(-32602, "invalid address"))?;
                let number = self.number(&params[if storage { 2 } else { 1 }])?;
                let engine = if number == self.head {
                    &self.latest
                } else if number == 0 {
                    &self.genesis
                } else {
                    return Err(RpcError(
                        -32000,
                        "historical state is not retained by this observer",
                    ));
                };
                let account = engine.account(address);
                Ok(match method {
                    "eth_getBalance" => json!(format!(
                        "0x{:x}",
                        account.map(|a| a.balance).unwrap_or_default()
                    )),
                    "eth_getTransactionCount" => {
                        json!(quantity(account.map(|a| a.nonce).unwrap_or_default()))
                    }
                    "eth_getCode" => json!(account.map(|a| a.code).unwrap_or_default()),
                    _ => {
                        let key = params[1]
                            .as_str()
                            .and_then(|s| {
                                let digits = s.strip_prefix("0x")?;
                                if digits.is_empty()
                                    || !digits.bytes().all(|b| b.is_ascii_hexdigit())
                                    || digits.len() > 64
                                    || (digits.len() > 1 && digits.starts_with('0'))
                                {
                                    return None;
                                }
                                s.parse::<U256>().ok()
                            })
                            .ok_or(RpcError(-32602, "invalid storage position"))?;
                        let value = account
                            .and_then(|a| a.storage.get(&key).copied())
                            .unwrap_or_default();
                        json!(format!("0x{value:064x}"))
                    }
                })
            }
            _ => Err(RpcError(-32601, "method not found")),
        }
    }
}
pub fn quantity(value: u64) -> String {
    format!("0x{value:x}")
}
fn parse_quantity(value: &str) -> Result<u64, crate::RpcError> {
    let s = value
        .strip_prefix("0x")
        .ok_or(crate::RpcError(-32602, "invalid quantity"))?;
    if s.is_empty()
        || !s.bytes().all(|b| b.is_ascii_hexdigit())
        || (s.len() > 1 && s.starts_with('0'))
    {
        return Err(crate::RpcError(-32602, "noncanonical quantity"));
    }
    u64::from_str_radix(s, 16).map_err(|_| crate::RpcError(-32602, "invalid quantity"))
}
pub(crate) fn parse_hash(value: &Value) -> Result<B256, crate::RpcError> {
    value
        .as_str()
        .filter(|s| s.starts_with("0x") && s.len() == 66)
        .and_then(|s| s.parse().ok())
        .ok_or(crate::RpcError(-32602, "invalid hash"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> Value {
        serde_json::from_str(include_str!(
            "../../../specs/test-vectors/execution-v1/geth-1.17.8.json"
        ))
        .unwrap()
    }
    fn replay_fixture(case: &Value) -> Snapshot {
        use tactus_o1_devnet_driver::tx::CellOutPoint;
        use tactus_o1_protocol::batch;
        let genesis: Genesis = serde_json::from_value(case["genesis"].clone()).unwrap();
        let engine = Executor::new(&genesis).unwrap();
        let input = rpc::decode_hex(case["batch"].as_str().unwrap()).unwrap();
        let summary = batch::validate_batch(&input, engine.anchor()).unwrap();
        Snapshot::replay(
            json!({"settled_batches":0,"ckb_genesis":"fixture","tip":null}),
            recovery::RecoveredAnchor {
                pinned_height: 1,
                pinned_hash: "fixture".into(),
                genesis: *engine.anchor(),
                genesis_allocation: genesis.allocation_bytes().unwrap(),
                point: CellOutPoint {
                    tx_hash: [0; 32],
                    index: 0,
                },
                state: summary.next,
                batches: vec![recovery::RecoveredBatch {
                    anchor_transaction: "fixture".into(),
                    publication_output: 0,
                    input_bytes: input,
                    summary,
                }],
            },
        )
        .unwrap()
    }
    #[test]
    fn reconstructed_rpc_receipts_and_logs_match_independent_geth_vectors() {
        let mut blocks = 0;
        let mut logs = 0;
        let fixture = fixtures();
        for case in fixture["cases"].as_array().unwrap() {
            let snapshot = replay_fixture(case);
            for (index, oracle) in case["geth"].as_array().unwrap().iter().enumerate() {
                let number = quantity(index as u64 + 1);
                let block = snapshot
                    .query("eth_getBlockByNumber", &[json!(number), json!(false)])
                    .unwrap();
                for (actual, expected) in [
                    ("stateRoot", "stateRoot"),
                    ("transactionsRoot", "txRoot"),
                    ("receiptsRoot", "receiptsRoot"),
                    ("logsBloom", "logsBloom"),
                    ("gasUsed", "gasUsed"),
                ] {
                    assert_eq!(
                        block[actual], oracle[expected],
                        "{} {number} {actual}",
                        case["name"]
                    );
                }
                let filtered = snapshot
                    .query("eth_getLogs", &[json!({"blockHash":block["hash"]})])
                    .unwrap();
                let mut expected_logs = Vec::new();
                for receipt in oracle["receipts"].as_array().unwrap() {
                    let actual = snapshot
                        .query(
                            "eth_getTransactionReceipt",
                            &[receipt["transactionHash"].clone()],
                        )
                        .unwrap();
                    for key in [
                        "transactionHash",
                        "transactionIndex",
                        "status",
                        "gasUsed",
                        "cumulativeGasUsed",
                        "logsBloom",
                    ] {
                        assert_eq!(actual[key], receipt[key]);
                    }
                    if receipt["contractAddress"] == json!(Address::ZERO) {
                        assert!(actual["contractAddress"].is_null());
                    } else {
                        assert_eq!(actual["contractAddress"], receipt["contractAddress"]);
                    }
                    let mut expected = receipt["logs"].as_array().unwrap().clone();
                    // The standalone Geth transition tool uses a dummy block hash.
                    for log in &mut expected {
                        log["blockHash"] = block["hash"].clone();
                    }
                    assert_eq!(actual["logs"], json!(expected));
                    expected_logs.extend(expected);
                }
                assert_eq!(filtered, json!(expected_logs));
                logs += expected_logs.len();
                blocks += 1;
            }
            let expected: Vec<_> = snapshot
                .blocks
                .iter()
                .filter(|(n, _)| **n > 0)
                .flat_map(|(_, b)| {
                    snapshot
                        .query("eth_getLogs", &[json!({"blockHash":b["hash"]})])
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .clone()
                })
                .collect();
            assert_eq!(
                snapshot
                    .query("eth_getLogs", &[json!({"fromBlock":"earliest"})])
                    .unwrap(),
                json!(expected)
            );
        }
        assert_eq!(blocks, 14);
        assert_eq!(logs, 2);
    }
    #[test]
    fn log_selection_defaults_and_errors_are_explicit() {
        let fixture = fixtures();
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "deploy-write-clear")
            .unwrap();
        let snapshot = replay_fixture(case);
        let latest = snapshot.query("eth_getLogs", &[json!({})]).unwrap();
        assert_eq!(latest.as_array().unwrap().len(), 1);
        assert_eq!(latest[0]["blockNumber"], "0x2");
        assert_eq!(
            snapshot
                .query(
                    "eth_getLogs",
                    &[json!({"fromBlock":"0x1","toBlock":"0x2","address":latest[0]["address"]})]
                )
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            snapshot
                .query(
                    "eth_getLogs",
                    &[json!({"fromBlock":"earliest","address":Address::ZERO})]
                )
                .unwrap(),
            json!([])
        );
        assert_eq!(
            snapshot
                .query("eth_getLogs", &[json!({"topics":[null]})])
                .unwrap(),
            json!([])
        );
        for filter in [
            json!({"blockHash":snapshot.blocks[&1]["hash"],"fromBlock":"0x1"}),
            json!({"fromBlock":"0x2","toBlock":"0x1"}),
            json!({"toBlock":"0x3"}),
            json!({"fromBlock":"finalized"}),
            json!({"blockHash":B256::ZERO}),
        ] {
            assert!(snapshot.query("eth_getLogs", &[filter]).is_err());
        }
    }
    #[test]
    fn oversized_log_responses_fail_instead_of_truncating() {
        let fixture = fixtures();
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "deploy-write-clear")
            .unwrap();
        let mut snapshot = replay_fixture(case);
        snapshot.head = 2048;
        assert_eq!(
            snapshot
                .query(
                    "eth_getLogs",
                    &[json!({"fromBlock":"0x0","toBlock":"0x400"})]
                )
                .unwrap_err()
                .0,
            -32005
        );
        snapshot.head = 2;
        let hash = parse_hash(&snapshot.blocks[&2]["transactions"][0]).unwrap();
        let log = snapshot.receipts[&hash]["logs"][0].clone();
        snapshot.receipts.get_mut(&hash).unwrap()["logs"] =
            json!(vec![log.clone(); crate::logs::MAX_LOGS + 1]);
        assert_eq!(
            snapshot.query("eth_getLogs", &[json!({})]).unwrap_err().0,
            -32005
        );
        let mut large = log;
        large["data"] = json!("00".repeat(crate::logs::MAX_BYTES / 2));
        snapshot.receipts.get_mut(&hash).unwrap()["logs"] = json!([large]);
        assert_eq!(
            snapshot.query("eth_getLogs", &[json!({})]).unwrap_err().0,
            -32005
        );
    }

    #[test]
    fn quantities_and_hashes_require_wire_encoding() {
        for invalid in [
            "0x",
            "0x00",
            "0x01",
            "0x+1",
            "1",
            "0X1",
            "0x10000000000000000",
            "0x 1",
        ] {
            assert!(parse_quantity(invalid).is_err(), "{invalid}");
        }
        assert_eq!(parse_quantity("0x0").unwrap(), 0);
        assert_eq!(parse_quantity("0xffffffffffffffff").unwrap(), u64::MAX);
        assert!(parse_hash(&json!("00".repeat(32))).is_err());
        assert!(parse_hash(&json!(format!("0x{}", "00".repeat(31)))).is_err());
        assert_eq!(
            parse_hash(&json!(format!("0x{}", "00".repeat(32)))).unwrap(),
            B256::ZERO
        );
    }
}
