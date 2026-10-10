//! Local read-only Ethereum observer backed by canonical CKB recovery.
mod snapshot;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tactus_o1_devnet_driver::recovery;
use tokio::sync::Semaphore;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    listen: SocketAddr,
    ckb_rpc: SocketAddr,
    ckb_genesis: String,
    anchor_type_script: String,
    settlement_type_script: String,
    max_batches: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct RpcError(i64, &'static str);
fn error(id: Value, e: RpcError) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":e.0,"message":e.1}})
}
fn request(
    value: Value,
    query: &impl Fn(&str, &[Value]) -> Result<Value, RpcError>,
) -> Option<Value> {
    let Some(object) = value.as_object() else {
        return Some(error(Value::Null, RpcError(-32600, "invalid request")));
    };
    let id = object.get("id").cloned().unwrap_or(Value::Null);
    if object.get("jsonrpc") != Some(&json!("2.0"))
        || !object.get("method").is_some_and(Value::is_string)
        || !(id.is_null() || id.is_string() || id.as_i64().is_some() || id.as_u64().is_some())
    {
        return Some(error(Value::Null, RpcError(-32600, "invalid request")));
    }
    let params = object.get("params").cloned().unwrap_or_else(|| json!([]));
    let reply = match params.as_array() {
        Some(params) => match query(object["method"].as_str().expect("checked method"), params) {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(e) => error(id, e),
        },
        None => error(id, RpcError(-32602, "positional parameters required")),
    };
    object.contains_key("id").then_some(reply)
}
struct App {
    snapshot: RwLock<Option<Arc<snapshot::Snapshot>>>,
    requests: Arc<Semaphore>,
}
fn dispatch(
    value: Value,
    query: &impl Fn(&str, &[Value]) -> Result<Value, RpcError>,
) -> Option<Value> {
    if let Some(batch) = value.as_array() {
        if batch.is_empty() || batch.len() > 64 {
            return Some(error(
                Value::Null,
                RpcError(-32600, "batch must contain 1 through 64 requests"),
            ));
        }
        let replies: Vec<_> = batch
            .iter()
            .cloned()
            .filter_map(|v| request(v, query))
            .collect();
        if replies.is_empty() {
            None
        } else {
            Some(json!(replies))
        }
    } else {
        request(value, query)
    }
}
fn unavailable(value: Value, failure: RpcError) -> Option<Value> {
    dispatch(value, &|_, _| Err(failure))
}
fn response(reply: Option<Value>) -> Response {
    match reply {
        Some(v) => Json(v).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
async fn handle(State(app): State<Arc<App>>, body: Bytes) -> Response {
    let value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return response(Some(error(Value::Null, RpcError(-32700, "parse error")))),
    };
    let permit = match app.requests.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return response(unavailable(value, RpcError(-32005, "observer busy"))),
    };
    let snapshot = app.snapshot.read().expect("snapshot lock").clone();
    let Some(snapshot) = snapshot else {
        return response(unavailable(
            value,
            RpcError(-32001, "canonical snapshot unavailable"),
        ));
    };
    if snapshot.created.elapsed() > Duration::from_secs(30) {
        return response(unavailable(
            value,
            RpcError(-32001, "canonical snapshot expired"),
        ));
    }
    let failure_value = value.clone();
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let check = || recovery::assert_canonical(snapshot.height, &snapshot.pin);
        if check().is_err() {
            return unavailable(
                value,
                RpcError(-32001, "canonical snapshot replaced or node unavailable"),
            );
        }
        let reply = dispatch(value.clone(), &|method, params| {
            snapshot.query(method, params)
        });
        if check().is_err() {
            unavailable(
                value,
                RpcError(-32001, "canonical snapshot changed during query"),
            )
        } else {
            reply
        }
    })
    .await;
    response(match task {
        Ok(v) => v,
        Err(_) => unavailable(failure_value, RpcError(-32603, "observer task failed")),
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: tactus-o1-observer-rpc CONFIG_JSON".into());
    }
    let bytes = std::fs::read(&args[1])?;
    if bytes.len() > 16384 {
        return Err("configuration too large".into());
    }
    let config: Config = serde_json::from_slice(&bytes)?;
    if !config.listen.ip().is_loopback()
        || !config.ckb_rpc.ip().is_loopback()
        || config.max_batches == 0
        || config.max_batches > 4096
    {
        return Err("observer requires loopback endpoints and 1..4096 max_batches".into());
    }
    std::env::set_var("TACTUS_CKB_RPC_ADDR", config.ckb_rpc.to_string());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let app = Arc::new(App {
            snapshot: RwLock::new(None),
            requests: Arc::new(Semaphore::new(8)),
        });
        let updater = app.clone();
        let settings = config.clone();
        tokio::spawn(async move {
            loop {
                let cfg = settings.clone();
                match tokio::task::spawn_blocking(move || snapshot::Snapshot::recover(&cfg)).await {
                    Ok(Ok(snapshot)) => {
                        *updater.snapshot.write().expect("snapshot lock") =
                            Some(Arc::new(snapshot));
                    }
                    failure => {
                        *updater.snapshot.write().expect("snapshot lock") = None;
                        eprintln!(
                            "canonical refresh failed: {}",
                            match failure {
                                Ok(Err(e)) => e,
                                _ => "worker failure".into(),
                            }
                        );
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        let listener = tokio::net::TcpListener::bind(config.listen).await?;
        eprintln!("Read-only observer listening on {}", config.listen);
        let router = Router::new()
            .route("/", post(handle))
            .layer(DefaultBodyLimit::max(65536))
            .with_state(app);
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_preserves_ids_and_suppresses_notifications() {
        let value = json!([
            {"jsonrpc":"2.0","id":"wallet-7","method":"eth_blockNumber"},
            {"jsonrpc":"2.0","method":"eth_blockNumber"},
            {"jsonrpc":"2.0","id":42,"method":"eth_chainId"},
            {"jsonrpc":"2.0","id":null,"method":"eth_chainId"}
        ]);
        let result = unavailable(value, RpcError(-32001, "unavailable")).unwrap();
        let replies = result.as_array().unwrap();
        assert_eq!(replies.len(), 3);
        assert_eq!(replies[0]["id"], "wallet-7");
        assert_eq!(replies[1]["id"], 42);
        assert!(replies[2]["id"].is_null());
        assert!(replies.iter().all(|r| r["error"]["code"] == -32001));
        assert!(unavailable(
            json!({"jsonrpc":"2.0","method":"x"}),
            RpcError(-32001, "down")
        )
        .is_none());
    }

    #[test]
    fn malformed_batches_and_ids_are_rejected() {
        for invalid in [
            json!([]),
            json!([1, 2]),
            json!({"jsonrpc":"2.0","method":"x","id":{}}),
            json!({"method":"x"}),
        ] {
            let response = dispatch(invalid, &|_, _| Ok(json!(1))).unwrap();
            let responses = response.as_array().cloned().unwrap_or(vec![response]);
            assert!(responses.iter().all(|r| r["error"]["code"] == -32600));
        }
        let batch = vec![json!({"jsonrpc":"2.0","method":"x","id":1}); 65];
        assert_eq!(
            dispatch(json!(batch), &|_, _| Ok(json!(1))).unwrap()["error"]["code"],
            -32600
        );
    }

    #[test]
    fn mixed_batch_reports_only_calls_in_order() {
        let value = json!([
            {"jsonrpc":"2.0","method":"x"},
            {"jsonrpc":"2.0","id":2,"method":"x","params":{}},
            {"jsonrpc":"2.0","id":3,"method":"x","params":[7]}
        ]);
        let replies = dispatch(value, &|_, p| Ok(json!(p))).unwrap();
        assert_eq!(replies.as_array().unwrap().len(), 2);
        assert_eq!(replies[0]["error"]["code"], -32602);
        assert_eq!(replies[1]["result"], json!([7]));
    }
}
