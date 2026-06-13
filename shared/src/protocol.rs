use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcEvent {
    pub event: String,
    pub data: serde_json::Value,
}

pub mod error_codes {
    pub const PORT_OCCUPIED: i32 = 1001;
    pub const CONFIG_INVALID: i32 = 1002;
    pub const BINARY_NOT_FOUND: i32 = 1003;
    pub const PROCESS_EXITED: i32 = 1004;
    pub const HEALTH_CHECK_FAILED: i32 = 1005;
    pub const HOSTS_PERMISSION_DENIED: i32 = 1006;
    pub const RUNTIME_CORRUPTED: i32 = 1007;
    pub const DATABASE_START_FAILED: i32 = 1008;
}

impl JsonRpcResponse {
    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: impl Into<String>, code: i32, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    pub fn error_with_data(
        id: impl Into<String>,
        code: i32,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
        }
    }
}
