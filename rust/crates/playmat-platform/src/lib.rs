//! The platform service (docs/RustMigration.md phase 2): the calls the gateway answers in Rust. It runs in the
//! gateway's process; the routing table decides which of its methods are used (`rust`), compared with Java's answers
//! (`shadow`) or left to Java.
//!
//! Now: the card catalog (`lookupCards`, `searchCards`, `getExpansionSets`, `getBasicLandSets`) from the file Java
//! exports, and the
//! web client's deck sync (`deckSync*`) on Java's own decks database, and decks imported by link from deck websites
//! (`deckImport*`).

pub mod admin;
pub mod catalog;
pub mod deckimport;
pub mod decks;
pub mod search;
pub mod validate;

use admin::AdminStores;
use async_trait::async_trait;
use catalog::{Catalog, Entry};
use deckimport::DeckImports;
use decks::DeckStore;
use playmat_protocol::handlers::{Caller, Handlers};
use playmat_protocol::jsonrpc::{Outcome, Request, INTERNAL_ERROR, INVALID_PARAMS};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

const CATALOG_METHODS: [&str; 5] =
    ["lookupCards", "getExpansionSets", "getBasicLandSets", "searchCards", "deckValidate"];

pub struct Platform {
    catalog_path: Option<PathBuf>,
    /// None until the file exists (the server writes it after its first start): the methods stay with Java
    catalog: RwLock<Option<Arc<Catalog>>>,
    /// db/web_decks.db (`XMAGE_PLATFORM_DECKS`); None leaves deck sync to Java
    decks: Option<DeckStore>,
    imports: DeckImports,
    /// db/web_admin.db and db/web_reports.db (`XMAGE_PLATFORM_ADMIN`, `XMAGE_PLATFORM_REPORTS`); None leaves the staff
    /// console to Java
    admin: Option<AdminStores>,
}

impl Platform {
    /// `catalog`: the file `-Dxmage.catalog.export` names (`XMAGE_PLATFORM_CATALOG`)
    pub fn new(catalog: Option<PathBuf>, decks: Option<DeckStore>, admin: Option<AdminStores>) -> Arc<Platform> {
        Arc::new(Platform {
            catalog_path: catalog,
            catalog: RwLock::new(None),
            decks,
            imports: DeckImports::new(),
            admin,
        })
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
                        info!(
                            "Catalog {}: {} printings, {} card rows ({})",
                            path.display(),
                            catalog.printings,
                            catalog.card_rows(),
                            catalog.stamp
                        );
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
        (CATALOG_METHODS.contains(&method) && self.catalog.read().unwrap().is_some())
            || (decks::METHODS.contains(&method) && self.decks.is_some())
            || deckimport::METHODS.contains(&method)
            || self.admin.as_ref().is_some_and(|admin| admin.handles(method))
    }

    fn difference(&self, method: &str, java: &Value, rust: &Value) -> Option<String> {
        admin::difference(method, java, rust)
    }

    fn slow(&self, method: &str) -> bool {
        method == "deckImportFromUrl"
    }

    async fn call(&self, caller: &Caller, request: &Request) -> Outcome {
        if let (Some(decks), true) = (&self.decks, decks::METHODS.contains(&request.method.as_str())) {
            return decks.call(caller, request, true).await;
        }
        if let Some(admin) = self.admin.as_ref().filter(|admin| admin.handles(&request.method)) {
            return admin.call(caller, request, true).await;
        }
        match request.method.as_str() {
            "deckImportSources" => return Outcome::Result(self.imports.sources()),
            "deckImportFromUrl" => {
                // a session call: before login, Java says so
                let (Some(_), Some(Value::String(link))) = (&caller.user, request.params.first()) else {
                    return Outcome::Declined;
                };
                return self.imports.import(link, &caller.connection.to_string()).await;
            }
            _ => {}
        }
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
            "deckValidate" => {
                // `call.string(0)`, `call.object(1, DeckCardLists.class)`: anything unusual is left to Java's messages
                let (Some(Value::String(deck_type)), Some(deck @ Value::Object(_))) =
                    (request.params.first(), request.params.get(1))
                else {
                    return Outcome::Declined;
                };
                match catalog.validate(deck_type, deck).await {
                    Ok(Ok(Some(result))) => Ok(result),
                    Ok(Ok(None)) => return Outcome::Declined,
                    Ok(Err(message)) => return Outcome::error(INVALID_PARAMS, message),
                    Err(e) => Err(e),
                }
            }
            "searchCards" => {
                // `call.object(0, CardCriteria.class)`, then the filters when they're there
                let criteria = match request.params.first() {
                    None | Some(Value::Null) => {
                        return Outcome::error(INVALID_PARAMS, "Missing parameter 1 (criteria)");
                    }
                    Some(criteria) => criteria,
                };
                match catalog.search(criteria, request.params.get(1)).await {
                    Ok(None) => return Outcome::Declined,
                    other => other.map(|views| Value::Array(views.unwrap_or_default())),
                }
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

    async fn shadow(&self, caller: &Caller, request: &Request) -> Outcome {
        match &self.decks {
            // Java stores the change: Rust's is rolled back
            Some(decks) if decks::METHODS.contains(&request.method.as_str()) => {
                decks.call(caller, request, false).await
            }
            _ if self.admin.as_ref().is_some_and(|admin| admin.handles(&request.method)) => {
                self.admin.as_ref().expect("checked").call(caller, request, false).await
            }
            _ => self.call(caller, request).await,
        }
    }
}
