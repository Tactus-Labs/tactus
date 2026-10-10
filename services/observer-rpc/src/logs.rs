//! Stateless event filtering over one already authenticated snapshot.
use crate::{snapshot::parse_hash, RpcError};
use alloy_primitives::{Address, B256};
use serde_json::Value;

pub const MAX_BLOCKS: u64 = 1024;
pub const MAX_LOGS: usize = 10_000;
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ALTERNATIVES: usize = 256;

pub fn address(value: &Value) -> Result<Address, RpcError> {
    value
        .as_str()
        .filter(|s| s.starts_with("0x") && s.len() == 42)
        .and_then(|s| s.parse().ok())
        .ok_or(RpcError(-32602, "invalid address"))
}

pub struct Criteria {
    addresses: Vec<Address>,
    topics: Vec<Vec<B256>>,
}
impl Criteria {
    pub fn parse(value: &Value) -> Result<Self, RpcError> {
        let object = value
            .as_object()
            .ok_or(RpcError(-32602, "log filter must be an object"))?;
        let addresses = match object.get("address").filter(|v| !v.is_null()) {
            None => vec![],
            Some(Value::Array(items)) => {
                if items.len() > MAX_ALTERNATIVES {
                    return Err(RpcError(-32005, "too many log addresses"));
                }
                items.iter().map(address).collect::<Result<_, _>>()?
            }
            Some(v) => vec![address(v)?],
        };
        let mut topics = Vec::new();
        if let Some(value) = object.get("topics").filter(|v| !v.is_null()) {
            let items = value
                .as_array()
                .ok_or(RpcError(-32602, "topics must be an array"))?;
            if items.len() > 4 {
                return Err(RpcError(-32602, "at most four topic positions"));
            }
            for item in items {
                let alternatives = match item {
                    Value::Null => vec![],
                    Value::Array(items) => {
                        if items.len() > MAX_ALTERNATIVES {
                            return Err(RpcError(-32005, "too many topic alternatives"));
                        }
                        let mut parsed = Vec::new();
                        for value in items {
                            if value.is_null() {
                                parsed.clear();
                                break;
                            }
                            parsed.push(parse_hash(value)?);
                        }
                        parsed
                    }
                    _ => vec![parse_hash(item)?],
                };
                topics.push(alternatives);
            }
        }
        Ok(Self { addresses, topics })
    }
    pub fn matches(&self, log: &Value) -> bool {
        if !self.addresses.is_empty()
            && !address(&log["address"]).is_ok_and(|a| self.addresses.contains(&a))
        {
            return false;
        }
        let Some(topics) = log["topics"].as_array() else {
            return false;
        };
        self.topics.len() <= topics.len()
            && self.topics.iter().zip(topics).all(|(choices, actual)| {
                choices.is_empty() || parse_hash(actual).is_ok_and(|hash| choices.contains(&hash))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn topic(n: u8) -> Value {
        json!(B256::repeat_byte(n))
    }
    #[test]
    fn topic_positions_are_and_with_or_alternatives_and_wildcards() {
        let log = json!({"address":Address::repeat_byte(1),"topics":[topic(1),topic(2)]});
        for topics in [
            json!([]),
            json!([null]),
            json!([[topic(3), topic(1)], topic(2)]),
            json!([[], topic(2)]),
            json!([[null, topic(3)], topic(2)]),
        ] {
            assert!(Criteria::parse(&json!({"topics":topics}))
                .unwrap()
                .matches(&log));
        }
        for topics in [
            json!([topic(2)]),
            json!([topic(1), topic(3)]),
            json!([null, null, null]),
        ] {
            assert!(!Criteria::parse(&json!({"topics":topics}))
                .unwrap()
                .matches(&log));
        }
        assert!(
            !Criteria::parse(&json!({"address":Address::repeat_byte(2)}))
                .unwrap()
                .matches(&log)
        );
        assert!(Criteria::parse(
            &json!({"address":[Address::repeat_byte(2),Address::repeat_byte(1)]})
        )
        .unwrap()
        .matches(&log));
        // A wildcard position still requires that the log contains that position.
        assert!(!Criteria::parse(&json!({"topics":[null]}))
            .unwrap()
            .matches(&json!({"topics":[]})));
    }
    #[test]
    fn invalid_and_unbounded_filters_fail() {
        for filter in [
            json!(null),
            json!({"address":[false]}),
            json!({"topics":["0x01"]}),
            json!({"topics":[null,null,null,null,null]}),
            json!({"topics":{}}),
            json!({"address":vec![Address::ZERO;257]}),
            json!({"topics":[vec![B256::ZERO;257]]}),
        ] {
            assert!(Criteria::parse(&filter).is_err(), "{filter}");
        }
    }
}
