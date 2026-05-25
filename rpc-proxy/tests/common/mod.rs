use parapet_rpc_proxy::rpc_handler::{JsonRpcRequest, JsonRpcResponse};

#[allow(dead_code)]
pub fn sample_request(method: &str) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: serde_json::json!(1),
        method: method.to_string(),
        params: vec![],
    }
}

#[allow(dead_code)]
pub fn sample_success_response(id: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({"ok": true})),
        error: None,
    }
}
