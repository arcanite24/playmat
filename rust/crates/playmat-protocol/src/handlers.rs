//! Calls the gateway answers in Rust instead of passing them to Java (routes `rust` and `shadow`). The platform
//! service (migration phase 2) provides them; the gateway itself provides none.

use crate::jsonrpc::{Outcome, Request};
use async_trait::async_trait;

/// Who is calling, as the gateway knows it from the logins it saw on the connection.
#[derive(Debug, Clone, Default)]
pub struct Caller {
    /// the logged in user, None before login
    pub user: Option<String>,
    /// logged in with the admin password
    pub admin: bool,
    /// the user signed in to an account (the server has accounts on)
    pub account: bool,
    /// the client's address
    pub ip: String,
    /// the gateway's number for the connection, which outlives its logins (Java's per-session limits)
    pub connection: u64,
}

#[async_trait]
pub trait Handlers: Send + Sync {
    /// true when this method has a Rust implementation
    fn handles(&self, method: &str) -> bool;

    async fn call(&self, caller: &Caller, request: &Request) -> Outcome;

    /// true for a call that can take seconds (it fetches from another site): the gateway answers it on its own task,
    /// so the connection's other calls don't wait for it
    fn slow(&self, _method: &str) -> bool {
        false
    }

    /// The answer Rust would give, for comparing with Java's (`shadow`): Java makes the change, so a call that
    /// writes must leave everything as it was. It runs before the call goes to Java.
    async fn shadow(&self, caller: &Caller, request: &Request) -> Outcome {
        self.call(caller, request).await
    }

    /// Where Rust's answer differs from Java's in `shadow`, or None. A method can allow for what differs by design
    /// (the time a write was made, a random code).
    fn difference(&self, _method: &str, java: &serde_json::Value, rust: &serde_json::Value) -> Option<String> {
        crate::compare::difference(java, rust)
    }

    /// a session ended on the Java side (logout, expiry, signed in elsewhere): drop what is kept for it
    fn session_ended(&self, _caller: &Caller) {}
}

/// No method is handled in Rust: every call goes to Java whatever the routes say.
pub struct NoHandlers;

#[async_trait]
impl Handlers for NoHandlers {
    fn handles(&self, _method: &str) -> bool {
        false
    }

    async fn call(&self, _caller: &Caller, _request: &Request) -> Outcome {
        Outcome::error(crate::jsonrpc::METHOD_NOT_FOUND, "Not handled by the gateway")
    }
}
