use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: InputParams,
}

#[derive(Debug, Deserialize)]
pub struct InputParams {
    pub prompt: String,
    pub pwd: String,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ResponseAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct ResponseAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, action: ResponseAction) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(action),
            error: None,
        }
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}

pub fn invalid_request(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32600, message, None)
}

pub fn method_not_found(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32601, message, None)
}

pub fn invalid_params(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32602, message, None)
}

pub fn internal_error(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32603, message, None)
}
