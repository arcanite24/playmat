//! Calls the gateway answers in Rust instead of passing them to Java (routes `rust` and `shadow`). The platform
//! service (migration phase 2) provides them; the gateway itself provides none.

use async_trait::async_trait;
use playmat_protocol::jsonrpc::{Outcome, Request};

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
}

#[async_trait]
pub trait Handlers: Send + Sync {
    /// true when this method has a Rust implementation
    fn handles(&self, method: &str) -> bool;

    async fn call(&self, caller: &Caller, request: &Request) -> Outcome;

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
        Outcome::error(playmat_protocol::jsonrpc::METHOD_NOT_FOUND, "Not handled by the gateway")
    }
}
