//! Where each call goes (docs/RustMigration.md, "Target architecture"): to the Java bridge, to a handler in Rust,
//! or to both ("shadow": Java answers, the Rust answer is compared and logged).
//!
//! The table comes from a TOML file (`XMAGE_ROUTES_FILE`) and environment overrides:
//!
//! ```toml
//! default = "java"
//! [groups]
//! Catalog = "shadow"
//! [methods]
//! searchCards = "rust"
//! [rollout]          # percent of connections that get the group's route; the others get java
//! Catalog = 25
//! ```
//!
//! `XMAGE_ROUTE_<GROUP>=java|rust|shadow` (group in capitals, e.g. `XMAGE_ROUTE_CATALOG`) and
//! `XMAGE_ROUTE_DEFAULT` override the file. A method with no Rust handler always goes to Java.

use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Route {
    Java,
    Rust,
    Shadow,
}

impl FromStr for Route {
    type Err = String;

    fn from_str(s: &str) -> Result<Route, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "java" => Ok(Route::Java),
            "rust" => Ok(Route::Rust),
            "shadow" => Ok(Route::Shadow),
            other => Err(format!("unknown route '{other}' (java, rust or shadow)")),
        }
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Route::Java => "java",
            Route::Rust => "rust",
            Route::Shadow => "shadow",
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutesFile {
    default: Option<Route>,
    #[serde(default)]
    groups: HashMap<String, Route>,
    #[serde(default)]
    methods: HashMap<String, Route>,
    #[serde(default)]
    rollout: HashMap<String, u8>,
}

#[derive(Debug, Clone)]
pub struct RouteTable {
    default: Route,
    groups: HashMap<String, Route>,
    methods: HashMap<String, Route>,
    rollout: HashMap<String, u8>,
}

impl Default for RouteTable {
    fn default() -> Self {
        RouteTable { default: Route::Java, groups: HashMap::new(), methods: HashMap::new(), rollout: HashMap::new() }
    }
}

impl RouteTable {
    pub fn from_toml(text: &str) -> Result<RouteTable, String> {
        let file: RoutesFile = toml::from_str(text).map_err(|e| e.to_string())?;
        let known = crate::groups();
        for group in file.groups.keys().chain(file.rollout.keys()) {
            if !known.contains(&group.as_str()) {
                return Err(format!("unknown group '{group}' (known: {})", known.join(", ")));
            }
        }
        for method in file.methods.keys() {
            if crate::method(method).is_none() {
                return Err(format!("unknown method '{method}'"));
            }
        }
        if let Some((group, percent)) = file.rollout.iter().find(|(_, p)| **p > 100) {
            return Err(format!("rollout for {group} is {percent}%, over 100"));
        }
        Ok(RouteTable {
            default: file.default.unwrap_or(Route::Java),
            groups: file.groups,
            methods: file.methods,
            rollout: file.rollout,
        })
    }

    /// `XMAGE_ROUTE_*` variables over this table
    pub fn with_env(mut self, vars: impl Iterator<Item = (String, String)>) -> Result<RouteTable, String> {
        let groups = crate::groups();
        for (key, value) in vars {
            let Some(name) = key.strip_prefix("XMAGE_ROUTE_") else { continue };
            let route: Route = value.parse()?;
            if name == "DEFAULT" {
                self.default = route;
                continue;
            }
            let group = groups
                .iter()
                .find(|g| g.to_ascii_uppercase() == name)
                .ok_or_else(|| format!("{key}: no group {name} (known: {})", groups.join(", ")))?;
            self.groups.insert(group.to_string(), route);
        }
        Ok(self)
    }

    /// The route of a call on a connection (`bucket` 0–99, fixed per connection, picks it in or out of a rollout).
    /// Methods the protocol doesn't know go to Java, which answers them.
    pub fn route(&self, method: &str, bucket: u8) -> Route {
        let Some(info) = crate::method(method) else { return Route::Java };
        let route = self
            .methods
            .get(method)
            .or_else(|| self.groups.get(&info.group))
            .copied()
            .unwrap_or(self.default);
        match self.rollout.get(&info.group) {
            Some(percent) if route != Route::Java && bucket >= *percent => Route::Java,
            _ => route,
        }
    }

    /// one line per group that isn't on Java, for the startup log
    pub fn describe(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("default {}", self.default));
        let mut groups: Vec<_> = self.groups.iter().collect();
        groups.sort();
        for (group, route) in groups {
            let rollout = self.rollout.get(group).map(|p| format!(" ({p}% of connections)")).unwrap_or_default();
            lines.push(format!("{group} {route}{rollout}"));
        }
        let mut methods: Vec<_> = self.methods.iter().collect();
        methods.sort();
        for (method, route) in methods {
            lines.push(format!("{method} {route}"));
        }
        lines.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_by_method_then_group_then_default() {
        let table = RouteTable::from_toml(
            r#"
            [groups]
            Catalog = "shadow"
            [methods]
            searchCards = "rust"
            "#,
        )
        .unwrap();
        assert_eq!(table.route("searchCards", 0), Route::Rust);
        assert_eq!(table.route("lookupCards", 0), Route::Shadow);
        assert_eq!(table.route("ping", 0), Route::Java);
        assert_eq!(table.route("noSuchMethod", 0), Route::Java);
    }

    #[test]
    fn rollout_keeps_the_rest_on_java() {
        let table = RouteTable::from_toml("[groups]\nCatalog = \"rust\"\n[rollout]\nCatalog = 25\n").unwrap();
        assert_eq!(table.route("lookupCards", 10), Route::Rust);
        assert_eq!(table.route("lookupCards", 25), Route::Java);
    }

    #[test]
    fn env_overrides_and_checks() {
        let table = RouteTable::default()
            .with_env(vec![("XMAGE_ROUTE_CATALOG".to_string(), "shadow".to_string())].into_iter())
            .unwrap();
        assert_eq!(table.route("searchCards", 0), Route::Shadow);
        assert!(RouteTable::default().with_env(vec![("XMAGE_ROUTE_NOPE".into(), "rust".into())].into_iter()).is_err());
        assert!(RouteTable::from_toml("[groups]\nNope = \"rust\"\n").is_err());
        assert!(RouteTable::from_toml("[methods]\nnope = \"rust\"\n").is_err());
        assert!(RouteTable::from_toml("[rollout]\nCatalog = 101\n").is_err());
    }
}
