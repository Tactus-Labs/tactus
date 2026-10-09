//! Ordering-state recovery using canonical CKB blocks only, with a pinned tip.
//! This recovers experimental head cells, not EVM state or settled withdrawals.
use crate::{lab, molecule, rpc, tx::CellOutPoint};
use serde_json::json;
use tactus_o1_ordering_script::OrderingHead;

pub fn recover_head(type_script: &[u8]) -> Result<(CellOutPoint, OrderingHead), String> {
    let tip = rpc::call("get_tip_header", json!([]))?;
    let number = u64::from_str_radix(
        tip["number"]
            .as_str()
            .ok_or("missing tip number")?
            .trim_start_matches("0x"),
        16,
    )
    .map_err(|e| e.to_string())?;
    let expected_type = molecule::script_to_json(type_script);
    let mut current: Option<(CellOutPoint, OrderingHead)> = None;
    let mut previous_block = None;
    for height in 0..=number {
        let block = rpc::get_block_detailed(height)?;
        if let Some(parent) = &previous_block {
            if block["header"]["parent_hash"] != *parent {
                return Err("chain changed during recovery".into());
            }
        }
        previous_block = Some(block["header"]["hash"].clone());
        for transaction in block["transactions"]
            .as_array()
            .ok_or("block transactions missing")?
        {
            let outputs = transaction["outputs"]
                .as_array()
                .ok_or("transaction outputs missing")?;
            for (index, output) in outputs.iter().enumerate() {
                if output["type"] != expected_type {
                    continue;
                }
                let state = OrderingHead::from_bytes(&rpc::decode_hex(
                    transaction["outputs_data"][index]
                        .as_str()
                        .ok_or("missing head data")?,
                )?)
                .map_err(|e| format!("head decode: {e:?}"))?;
                if let Some((prior, _)) = &current {
                    let expected = json!({"tx_hash":rpc::bytes_to_hex(&prior.tx_hash),"index":format!("0x{:x}",prior.index)});
                    if !transaction["inputs"]
                        .as_array()
                        .ok_or("inputs missing")?
                        .iter()
                        .any(|i| i["previous_output"] == expected)
                    {
                        return Err("head succession disconnected from recovered identity".into());
                    }
                } else {
                    state
                        .validate_genesis()
                        .map_err(|e| format!("invalid genesis: {e:?}"))?;
                }
                current = Some((
                    lab::point(
                        transaction["hash"]
                            .as_str()
                            .ok_or("transaction hash missing")?,
                        index as u32,
                    )?,
                    state,
                ));
            }
        }
    }
    if previous_block.as_ref() != Some(&tip["hash"])
        || rpc::call("get_tip_header", json!([]))?["hash"] != tip["hash"]
    {
        return Err("canonical tip changed; retry recovery".into());
    }
    let (point, state) = current.ok_or("head identity not found in canonical blocks")?;
    let live = rpc::call(
        "get_live_cell",
        json!([{"tx_hash":rpc::bytes_to_hex(&point.tx_hash),"index":format!("0x{:x}",point.index)},false]),
    )?;
    if live["status"] != "live" {
        return Err("recovered head is not live".into());
    }
    Ok((point, state))
}
