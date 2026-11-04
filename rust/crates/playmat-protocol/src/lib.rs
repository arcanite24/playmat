//! The web protocol as the gateway sees it: every method the Java bridge registers, the group it belongs to (one
//! `*Api` class, from `methods.json`, which `WebClientApiDocsTest` generates), and where each group is routed.

pub mod jsonrpc;
pub mod routes;

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Who may call a method (`RpcMethod.Access` in Java).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// before login
    Public,
    /// logs in: creates the connection's session
    Establish,
    /// needs a logged in connection
    Session,
    /// works with or without a login
    Optional,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Method {
    pub group: String,
    pub access: Access,
}

const METHODS_JSON: &str = include_str!("../methods.json");

/// Every method, by name.
pub fn methods() -> &'static HashMap<String, Method> {
    static METHODS: OnceLock<HashMap<String, Method>> = OnceLock::new();
    METHODS.get_or_init(|| serde_json::from_str(METHODS_JSON).expect("methods.json"))
}

pub fn method(name: &str) -> Option<&'static Method> {
    methods().get(name)
}

/// The groups, sorted.
pub fn groups() -> Vec<&'static str> {
    let mut groups: Vec<&str> = methods().values().map(|m| m.group.as_str()).collect();
    groups.sort_unstable();
    groups.dedup();
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_method_has_a_group() {
        assert!(methods().len() > 200);
        assert_eq!(method("ping").unwrap().group, "Session");
        assert_eq!(method("ping").unwrap().access, Access::Public);
        assert_eq!(method("searchCards").unwrap().group, "Catalog");
        assert_eq!(method("deckSyncList").unwrap().group, "Account");
        assert!(groups().contains(&"Admin"));
    }
}
