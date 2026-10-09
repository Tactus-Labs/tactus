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

/// A complete published input whose commitment and predecessor were verified
/// against canonical anchor states. Opaque bytes are not executed here.
#[derive(Debug)]
pub struct RecoveredBatch {
    pub anchor_transaction: String,
    pub publication_output: usize,
    pub input_bytes: Vec<u8>,
    pub summary: tactus_o1_protocol::batch::BatchSummary,
}

#[derive(Debug)]
pub struct RecoveredAnchor {
    pub point: CellOutPoint,
    pub state: tactus_o1_protocol::batch::AnchorState,
    pub batches: Vec<RecoveredBatch>,
}

/// Reconstruct the complete committed input sequence from canonical CKB blocks.
/// No operator database, indexer or unpublished witness cache is consulted.
pub fn recover_published_batches(type_script: &[u8]) -> Result<RecoveredAnchor, String> {
    use tactus_o1_protocol::batch::{self, AnchorState};
    let pinned = rpc::call("get_tip_header", json!([]))?;
    let height = u64::from_str_radix(
        pinned["number"]
            .as_str()
            .ok_or("tip number missing")?
            .trim_start_matches("0x"),
        16,
    )
    .map_err(|e| e.to_string())?;
    let expected_type = molecule::script_to_json(type_script);
    let mut immutable = expected_type.clone();
    immutable["args"] = json!("0x");
    let mut current: Option<(CellOutPoint, AnchorState)> = None;
    let mut batches = Vec::new();
    let mut previous_hash = None;
    for number in 0..=height {
        let block = rpc::get_block_detailed(number)?;
        if previous_hash
            .as_ref()
            .is_some_and(|hash| *hash != block["header"]["parent_hash"])
        {
            return Err("chain changed during batch recovery".into());
        }
        previous_hash = Some(block["header"]["hash"].clone());
        for transaction in block["transactions"]
            .as_array()
            .ok_or("block transactions missing")?
        {
            let outputs = transaction["outputs"].as_array().ok_or("outputs missing")?;
            let heads: Vec<usize> = outputs
                .iter()
                .enumerate()
                .filter(|(_, o)| o["type"] == expected_type)
                .map(|(i, _)| i)
                .collect();
            if heads.is_empty() {
                continue;
            }
            if heads.len() != 1 {
                return Err("ambiguous anchor succession".into());
            }
            let output_index = heads[0];
            let next = AnchorState::decode(&rpc::decode_hex(
                transaction["outputs_data"][output_index]
                    .as_str()
                    .ok_or("state data missing")?,
            )?)
            .map_err(|e| format!("anchor decoding: {e:?}"))?;
            let transaction_hash = transaction["hash"]
                .as_str()
                .ok_or("transaction hash missing")?;
            if let Some((point, state)) = current {
                let expected = json!({"tx_hash":rpc::bytes_to_hex(&point.tx_hash),"index":format!("0x{:x}",point.index)});
                if !transaction["inputs"]
                    .as_array()
                    .ok_or("inputs missing")?
                    .iter()
                    .any(|i| i["previous_output"] == expected)
                {
                    return Err("disconnected anchor predecessor".into());
                }
                let mut recovered = None;
                for (index, output) in outputs.iter().enumerate() {
                    if !output["type"].is_null() || output["lock"] != immutable {
                        continue;
                    }
                    let input_bytes = rpc::decode_hex(
                        transaction["outputs_data"][index]
                            .as_str()
                            .ok_or("publication data missing")?,
                    )?;
                    if let Ok(summary) = batch::validate_batch(&input_bytes, &state) {
                        if summary.next == next {
                            recovered = Some(RecoveredBatch {
                                anchor_transaction: transaction_hash.into(),
                                publication_output: index,
                                input_bytes,
                                summary,
                            });
                            break;
                        }
                    }
                }
                batches
                    .push(recovered.ok_or("anchor has no matching immutable batch publication")?);
            } else {
                next.validate_genesis()
                    .map_err(|e| format!("bad anchor genesis: {e:?}"))?;
            }
            current = Some((lab::point(transaction_hash, output_index as u32)?, next));
        }
    }
    if previous_hash.as_ref() != Some(&pinned["hash"]) {
        return Err("chain changed before pinned tip".into());
    }
    let (point, state) = current.ok_or("anchor domain not found")?;
    let live = rpc::call(
        "get_live_cell",
        json!([{"tx_hash":rpc::bytes_to_hex(&point.tx_hash),"index":format!("0x{:x}",point.index)},false]),
    )?;
    if live["status"] != "live" || rpc::call("get_tip_header", json!([]))?["hash"] != pinned["hash"]
    {
        return Err("canonical head changed; retry recovery".into());
    }
    Ok(RecoveredAnchor {
        point,
        state,
        batches,
    })
}
