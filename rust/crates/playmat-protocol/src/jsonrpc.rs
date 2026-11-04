//! JSON-RPC 2.0 as the web protocol uses it (docs/WebSocketAPI.md, "Message format").

use serde_json::{json, Value};

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const SERVER_REJECTED: i64 = -32000;
pub const NOT_AUTHORIZED: i64 = -32001;
pub const TOO_MANY_REQUESTS: i64 = -32003;
pub const SERVER_BUSY: i64 = -32004;

/// A client request, as far as routing needs it. Anything else (not JSON, a batch, no method) is left to the
/// Java bridge, which answers it with the right error.
#[derive(Debug, Clone)]
pub struct Request {
    /// None for a notification
    pub id: Option<Value>,
    pub method: String,
    pub params: Vec<Value>,
}

impl Request {
    pub fn parse(text: &str) -> Option<Request> {
        let value: Value = serde_json::from_str(text).ok()?;
        let object = value.as_object()?;
        let method = object.get("method")?.as_str()?.to_string();
        let params = match object.get("params") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(params)) => params.clone(),
            Some(_) => return None,
        };
        Some(Request { id: object.get("id").cloned(), method, params })
    }

    pub fn to_text(&self) -> String {
        let mut request = json!({"jsonrpc": "2.0", "method": self.method, "params": self.params});
        if let Some(id) = &self.id {
            request["id"] = id.clone();
        }
        request.to_string()
    }
}

/// What a handler answers: a result, or an error with the protocol's code.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Result(Value),
    Error { code: i64, message: String, data: Option<Value> },
}

impl Outcome {
    pub fn error(code: i64, message: impl Into<String>) -> Outcome {
        Outcome::Error { code, message: message.into(), data: None }
    }

    /// the reply to the request with this id
    pub fn reply(&self, id: &Value) -> String {
        match self {
            Outcome::Result(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string(),
            Outcome::Error { code, message, data } => {
                let mut error = json!({"code": code, "message": message});
                if let Some(data) = data {
                    error["data"] = data.clone();
                }
                json!({"jsonrpc": "2.0", "id": id, "error": error}).to_string()
            }
        }
    }

    /// a reply from the Java bridge, read back
    pub fn from_reply(reply: &Value) -> Option<Outcome> {
        if let Some(result) = reply.get("result") {
            return Some(Outcome::Result(result.clone()));
        }
        let error = reply.get("error")?;
        Some(Outcome::Error {
            code: error.get("code")?.as_i64()?,
            message: error.get("message").and_then(Value::as_str).unwrap_or_default().to_string(),
            data: error.get("data").cloned(),
        })
    }
}

/// A frame from the server: a reply (has an id and a result or error) or an event (`{"method": ..., "data": ...}`).
pub fn is_reply(frame: &Value) -> bool {
    frame.get("jsonrpc").is_some() && (frame.get("result").is_some() || frame.get("error").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_and_notifications() {
        let request = Request::parse(r#"{"jsonrpc":"2.0","id":3,"method":"ping","params":[]}"#).unwrap();
        assert_eq!(request.id, Some(json!(3)));
        assert_eq!(request.method, "ping");
        let notification = Request::parse(r#"{"jsonrpc":"2.0","method":"ping"}"#).unwrap();
        assert_eq!(notification.id, None);
        assert!(Request::parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{"a":1}}"#).is_none());
        assert!(Request::parse("[1]").is_none());
        assert!(Request::parse("{").is_none());
    }

    #[test]
    fn replies() {
        assert_eq!(Outcome::Result(json!(true)).reply(&json!(1)), r#"{"jsonrpc":"2.0","id":1,"result":true}"#);
        assert_eq!(
            Outcome::error(NOT_AUTHORIZED, "no").reply(&json!("a")),
            r#"{"jsonrpc":"2.0","id":"a","error":{"code":-32001,"message":"no"}}"#
        );
    }
}
