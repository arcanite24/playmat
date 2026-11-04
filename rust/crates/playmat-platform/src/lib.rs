//! The platform service (docs/RustMigration.md phase 2): the calls the gateway answers in Rust. It runs in the
//! gateway's process; the routing table decides which of its methods are used (`rust`), compared with Java's answers
//! (`shadow`) or left to Java.
//!
//! Now: the card catalog (`lookupCards`, `getExpansionSets`, `getBasicLandSets`) from the file Java exports.

pub mod catalog;

use async_trait::async_trait;
use catalog::{Catalog, Entry};
use playmat_protocol::handlers::{Caller, Handlers};
use playmat_protocol::jsonrpc::{Outcome, Request, INTERNAL_ERROR, INVALID_PARAMS};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

const CATALOG_METHODS: [&str; 3] = ["lookupCards", "getExpansionSets", "getBasicLandSets"];

pub struct Platform {
    catalog_path: Option<PathBuf>,
    /// None until the file exists (the server writes it after its first start): the methods stay with Java
    catalog: RwLock<Option<Arc<Catalog>>>,
}

impl Platform {
    /// `catalog`: the file `-Dxmage.catalog.export` names (`XMAGE_PLATFORM_CATALOG`)
    pub fn new(catalog: Option<PathBuf>) -> Arc<Platform> {
        Arc::new(Platform { catalog_path: catalog, catalog: RwLock::new(None) })
    }

    pub fn catalog(&self) -> Option<Arc<Catalog>> {
        self.catalog.read().unwrap().clone()
    }

    /// Opens the catalog, and opens it again whenever the server writes a new one (it replaces the file whole).
    pub async fn watch(self: Arc<Platform>) {
        let Some(path) = self.catalog_path.clone() else { return };
        let mut loaded: Option<SystemTime> = None;
        loop {
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if modified.is_some() && modified != loaded {
                match Catalog::open(&path).await {
                    Ok(catalog) => {
                        info!("Catalog {}: {} printings ({})", path.display(), catalog.printings, catalog.stamp);
                        *self.catalog.write().unwrap() = Some(Arc::new(catalog));
                        loaded = modified;
                    }
                    Err(e) => warn!("Catalog {} can't be read: {e}", path.display()),
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}

#[async_trait]
impl Handlers for Platform {
    fn handles(&self, method: &str) -> bool {
        CATALOG_METHODS.contains(&method) && self.catalog.read().unwrap().is_some()
    }

    async fn call(&self, _caller: &Caller, request: &Request) -> Outcome {
        let Some(catalog) = self.catalog() else {
            return Outcome::error(INTERNAL_ERROR, "The card catalog isn't loaded");
        };
        let started = std::time::Instant::now();
        let result = match request.method.as_str() {
            "getExpansionSets" => Ok(catalog.expansion_sets()),
            "getBasicLandSets" => Ok(catalog.basic_land_sets()),
            "lookupCards" => {
                // `call.object(0, DeckCardInfo[].class)`: the parameter must be there and be a list
                let entries: Vec<Option<Entry>> = match request.params.first() {
                    Some(Value::Array(items)) => items.iter().map(Entry::from_json).collect(),
                    None | Some(Value::Null) => return Outcome::error(INVALID_PARAMS, "Missing parameter 1 (cards)"),
                    Some(_) => return Outcome::error(INVALID_PARAMS, "Parameter 1 (cards) must be DeckCardInfo[]"),
                };
                catalog.lookup(&entries).await.map(Value::Array)
            }
            _ => return Outcome::error(playmat_protocol::jsonrpc::METHOD_NOT_FOUND, "Not a platform method"),
        };
        debug!("{} in {:?}", request.method, started.elapsed());
        match result {
            Ok(value) => Outcome::Result(value),
            Err(e) => {
                warn!("{}: {e}", request.method);
                Outcome::error(INTERNAL_ERROR, "Internal error")
            }
        }
    }
}
