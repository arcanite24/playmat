//! Settings, from the environment.

use crate::net::{parse_list, Cidr};
use playmat_protocol::routes::RouteTable;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    /// where browsers connect (`XMAGE_GATEWAY_LISTEN`, default 0.0.0.0:17180)
    pub listen: SocketAddr,
    /// the Java bridge (`XMAGE_GATEWAY_JAVA_URL`, default ws://127.0.0.1:17172)
    pub java_url: url::Url,
    /// proxies whose X-Forwarded-For / X-Real-IP are believed (`XMAGE_GATEWAY_TRUSTED_PROXIES`, default loopback)
    pub trusted_proxies: Vec<Cidr>,
    /// make game-state patches here instead of in Java (`XMAGE_GATEWAY_STATE_DIFFS`, default true)
    pub state_diffs: bool,
    /// unsent bytes queued for one client before it is dropped (`XMAGE_GATEWAY_MAX_OUTGOING_BYTES`, default 16 MB)
    pub max_outgoing_bytes: usize,
    /// largest message a client may send (4 MB, as the Java bridge)
    pub max_message_bytes: usize,
    pub routes: RouteTable,
    /// when set, `shadow` routes run Rust only for these users, lower case; everyone else goes to Java
    /// (`XMAGE_SHADOW_USERS`, comma separated)
    pub shadow_users: Option<Vec<String>>,
    /// games Java hands over are played on the Rust host (`XMAGE_RUST_HOST`, default off; docs/rust/GameHost.md)
    pub rust_host: bool,
}

impl Config {
    /// whether a `shadow` call by this user runs Rust as well as Java
    pub fn shadows(&self, user: Option<&str>) -> bool {
        match &self.shadow_users {
            None => true,
            Some(users) => user.map(|u| users.contains(&u.to_lowercase())).unwrap_or(false),
        }
    }
}

fn names(list: &str) -> Vec<String> {
    list.split(',').map(|n| n.trim().to_lowercase()).filter(|n| !n.is_empty()).collect()
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn flag(name: &str, default: bool) -> Result<bool, String> {
    match var(name) {
        None => Ok(default),
        Some(v) => match v.to_ascii_lowercase().as_str() {
            "true" | "1" | "on" | "yes" => Ok(true),
            "false" | "0" | "off" | "no" => Ok(false),
            _ => Err(format!("{name} must be true or false (got '{v}')")),
        },
    }
}

impl Config {
    pub fn from_env() -> Result<Config, String> {
        let listen = var("XMAGE_GATEWAY_LISTEN")
            .unwrap_or_else(|| "0.0.0.0:17180".into())
            .parse()
            .map_err(|e| format!("XMAGE_GATEWAY_LISTEN: {e}"))?;
        let java_url = var("XMAGE_GATEWAY_JAVA_URL")
            .unwrap_or_else(|| "ws://127.0.0.1:17172".into())
            .parse()
            .map_err(|e| format!("XMAGE_GATEWAY_JAVA_URL: {e}"))?;
        let trusted_proxies =
            parse_list(&var("XMAGE_GATEWAY_TRUSTED_PROXIES").unwrap_or_else(|| "127.0.0.1,::1".into()))
                .map_err(|e| format!("XMAGE_GATEWAY_TRUSTED_PROXIES: {e}"))?;
        let max_outgoing_bytes = match var("XMAGE_GATEWAY_MAX_OUTGOING_BYTES") {
            None => 16 * 1024 * 1024,
            Some(v) => v.parse().map_err(|_| format!("XMAGE_GATEWAY_MAX_OUTGOING_BYTES: not a number: {v}"))?,
        };
        let routes = match var("XMAGE_ROUTES_FILE") {
            None => RouteTable::default(),
            Some(path) => {
                let text = std::fs::read_to_string(&path).map_err(|e| format!("XMAGE_ROUTES_FILE {path}: {e}"))?;
                RouteTable::from_toml(&text).map_err(|e| format!("XMAGE_ROUTES_FILE {path}: {e}"))?
            }
        };
        let routes = routes.with_env(std::env::vars())?;
        Ok(Config {
            listen,
            java_url,
            trusted_proxies,
            state_diffs: flag("XMAGE_GATEWAY_STATE_DIFFS", true)?,
            max_outgoing_bytes,
            max_message_bytes: 4 * 1024 * 1024,
            routes,
            shadow_users: var("XMAGE_SHADOW_USERS").map(|v| names(&v)),
            rust_host: flag("XMAGE_RUST_HOST", false)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(shadow_users: Option<&str>) -> Config {
        Config {
            listen: "127.0.0.1:1".parse().unwrap(),
            java_url: "ws://127.0.0.1:2".parse().unwrap(),
            trusted_proxies: vec![],
            state_diffs: true,
            max_outgoing_bytes: 1,
            max_message_bytes: 1,
            routes: RouteTable::default(),
            shadow_users: shadow_users.map(names),
            rust_host: false,
        }
    }

    #[test]
    fn shadow_users() {
        assert!(config(None).shadows(Some("anyone")));
        assert!(config(None).shadows(None));
        let listed = config(Some(" Neri, other ,"));
        assert!(listed.shadows(Some("neri")));
        assert!(listed.shadows(Some("NERI")));
        assert!(listed.shadows(Some("other")));
        assert!(!listed.shadows(Some("someone")));
        assert!(!listed.shadows(None));
    }
}
