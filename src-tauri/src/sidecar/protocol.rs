use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,

    pub method: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i32,

    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

fn jsonrpc_version() -> String {
    "2.0".to_string()
}

pub mod codes {

    pub const METHOD_NOT_FOUND: i32 = -32601;

    pub const INVALID_PARAMS: i32 = -32602;

    pub const INTERNAL: i32 = -32603;

    pub const INVALID_STATE: i32 = -32000;
}

impl Request {
    pub fn call(id: u64, method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: jsonrpc_version(),
            method: method.to_string(),
            params,
            id: Some(Value::from(id)),
        }
    }

    pub fn notify(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: jsonrpc_version(),
            method: method.to_string(),
            params,
            id: None,
        }
    }
}

impl Response {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: jsonrpc_version(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: jsonrpc_version(),
            result: None,
            error: Some(Error {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    pub connected: bool,

    pub approved: bool,

    pub phase: String,

    pub listening: bool,

    pub speaking: bool,

    pub muted: bool,

    pub volume: i32,

    pub error: Option<String>,
}

pub mod methods {

    pub const PING: &str = "ping";

    pub const STATE: &str = "state";

    pub const CAPABILITIES: &str = "capabilities";

    pub const RECONNECT: &str = "reconnect";

    pub const START_LISTENING: &str = "startListening";

    pub const STOP_LISTENING: &str = "stopListening";

    pub const STATE_CHANGED: &str = "stateChanged";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_request_serializes_without_an_empty_id() {
        let request = Request::call(1, "ping", None);
        let text = serde_json::to_string(&request).expect("serialize");
        assert!(!text.contains("\"id\":null"));
        assert!(text.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn a_notification_has_no_id_at_all() {
        let request = Request::notify("stateChanged", Some(json!({"a": 1})));
        let text = serde_json::to_string(&request).expect("serialize");
        assert!(!text.contains("\"id\""));
    }

    #[test]
    fn a_response_round_trips() {
        let response = Response::ok(Some(json!(7)), json!({"ok": true}));
        let text = serde_json::to_string(&response).expect("serialize");
        let parsed: Response = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed.id, Some(json!(7)));
        assert_eq!(parsed.result, Some(json!({"ok": true})));
        assert!(parsed.error.is_none());
    }

    #[test]
    fn an_error_response_carries_the_code() {
        let response = Response::err(Some(json!(1)), codes::METHOD_NOT_FOUND, "no such method");
        let text = serde_json::to_string(&response).expect("serialize");
        let parsed: Response = serde_json::from_str(&text).expect("parse");
        let error = parsed.error.expect("error");
        assert_eq!(error.code, codes::METHOD_NOT_FOUND);
        assert_eq!(error.message, "no such method");
        assert!(parsed.result.is_none());
    }

    #[test]
    fn a_request_with_no_id_parses_as_a_notification() {
        let parsed: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"ping"}"#).expect("parse");
        assert_eq!(parsed.method, "ping");
        assert!(parsed.id.is_none());
    }
}
