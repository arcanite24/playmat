//! Decks imported by link from deck websites (`deckImportFromUrl`, `deckImportSources`), as Java's
//! `DeckImportService` does: the same five sites, the same five minute cache, ten imports a minute per connection,
//! and a site that keeps failing in ways that mean it changed or blocks us is reported as degraded for half an hour.
//!
//! Links Java would read differently are declined (anything outside plain ASCII, `%` escapes, user info, IPv6 hosts,
//! hosts that aren't a deck site's), so Java answers those.

mod fetch;
mod html;
mod json;
mod sources;

pub use fetch::{Fetcher, HttpFetcher};

use playmat_protocol::jsonrpc::Outcome;
use serde_json::{json, Map, Value};
use sources::Source;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tracing::info;

pub const METHODS: [&str; 2] = ["deckImportFromUrl", "deckImportSources"];

/// `RpcException.DECK_IMPORT_FAILED`
pub const DECK_IMPORT_FAILED: i64 = -32005;

const CACHE_MILLIS: u64 = 5 * 60_000;
const CACHE_SIZE: usize = 200;
const IMPORTS_PER_MINUTE: usize = 10;
const FAILURES_TO_DEGRADE: u32 = 3;
const DEGRADED_MILLIS: u64 = 30 * 60_000;

/// `DeckImportException.Reason`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Private,
    NotFound,
    Blocked,
    RateLimited,
    SiteChanged,
    Unsupported,
    TooLarge,
    Timeout,
    Unavailable,
}

impl Reason {
    pub fn wire_name(self) -> &'static str {
        match self {
            Reason::Private => "private",
            Reason::NotFound => "not_found",
            Reason::Blocked => "blocked",
            Reason::RateLimited => "rate_limited",
            Reason::SiteChanged => "site_changed",
            Reason::Unsupported => "unsupported",
            Reason::TooLarge => "too_large",
            Reason::Timeout => "timeout",
            Reason::Unavailable => "unavailable",
        }
    }
}

/// Why a deck couldn't be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Import(Reason, String),
    /// where Java's parser would throw a RuntimeException: the service reports the site as changed
    Shape,
}

impl Failure {
    pub fn new(reason: Reason, message: impl Into<String>) -> Failure {
        Failure::Import(reason, message.into())
    }
}

/// `DeckCardInfo`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCard {
    pub name: Option<String>,
    pub set_code: Option<String>,
    pub number: Option<String>,
    pub amount: i32,
}

impl DeckCard {
    fn to_json(&self) -> Value {
        let mut out = Map::new();
        if let Some(name) = &self.name {
            out.insert("cardName".into(), json!(name));
        }
        if let Some(set_code) = &self.set_code {
            out.insert("setCode".into(), json!(set_code));
        }
        if let Some(number) = &self.number {
            out.insert("cardNumber".into(), json!(number));
        }
        out.insert("amount".into(), json!(self.amount));
        Value::Object(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Main,
    Side,
    Commander,
    Companion,
    Maybe,
}

/// `ImportedDeck`: a deck read from a deck website, before its cards are matched against the card database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportedDeck {
    pub site: String,
    pub remote_id: String,
    pub url: String,
    pub name: Option<String>,
    pub author: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub main: Vec<DeckCard>,
    pub side: Vec<DeckCard>,
    pub commanders: Vec<DeckCard>,
    pub companion: Vec<DeckCard>,
    pub maybeboard_count: i32,
    pub cover: Option<DeckCard>,
    pub remote_updated_at: Option<i64>,
}

impl ImportedDeck {
    fn new(site: &str, remote_id: &str, url: String) -> ImportedDeck {
        ImportedDeck { site: site.into(), remote_id: remote_id.into(), url, ..ImportedDeck::default() }
    }

    fn add(
        &mut self,
        section: Section,
        name: Option<&str>,
        set_code: Option<&str>,
        number: Option<&str>,
        amount: i32,
    ) -> Result<(), Failure> {
        let Some(name) = name.map(java_trim).filter(|n| !n.is_empty()) else { return Ok(()) };
        if amount <= 0 {
            return Ok(());
        }
        // DeckCardInfo.makeSureCardAmountFine throws
        if amount > 99 {
            return Err(Failure::Shape);
        }
        let card = DeckCard {
            name: Some(name.to_string()),
            set_code: blank_to_null(set_code).map(|s| s.to_uppercase()),
            number: blank_to_null(number).map(str::to_string),
            amount,
        };
        match section {
            Section::Main => self.main.push(card),
            Section::Side => self.side.push(card),
            Section::Commander => self.commanders.push(card),
            Section::Companion => self.companion.push(card),
            Section::Maybe => self.maybeboard_count = self.maybeboard_count.wrapping_add(amount),
        }
        Ok(())
    }

    pub fn card_count(&self) -> i32 {
        [&self.main, &self.side, &self.commanders, &self.companion]
            .iter()
            .flat_map(|list| list.iter())
            .fold(0i32, |sum, card| sum.wrapping_add(card.amount))
    }

    fn set_name(&mut self, name: Option<&str>) {
        self.name = blank_to_null(name).map(str::to_string);
    }

    fn set_author(&mut self, author: Option<&str>) {
        self.author = blank_to_null(author).map(str::to_string);
    }

    fn set_description(&mut self, description: Option<&str>) {
        self.description = blank_to_null(description).map(str::to_string);
    }

    fn set_cover(&mut self, name: Option<&str>, set_code: Option<&str>, number: Option<&str>) {
        let (Some(set_code), Some(number)) = (set_code, number) else { return };
        if set_code.is_empty() || number.is_empty() {
            return;
        }
        self.cover = Some(DeckCard {
            name: name.map(str::to_string),
            set_code: Some(set_code.to_uppercase()),
            number: Some(number.to_string()),
            amount: 1,
        });
    }

    /// as Gson writes it: fields in declaration order, nulls left out
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("site".into(), json!(self.site));
        out.insert("remoteId".into(), json!(self.remote_id));
        out.insert("url".into(), json!(self.url));
        for (key, value) in [
            ("name", &self.name),
            ("author", &self.author),
            ("format", &self.format),
            ("description", &self.description),
        ] {
            if let Some(value) = value {
                out.insert(key.into(), json!(value));
            }
        }
        for (key, list) in [
            ("main", &self.main),
            ("side", &self.side),
            ("commanders", &self.commanders),
            ("companion", &self.companion),
        ] {
            out.insert(key.into(), Value::Array(list.iter().map(DeckCard::to_json).collect()));
        }
        out.insert("maybeboardCount".into(), json!(self.maybeboard_count));
        if let Some(cover) = &self.cover {
            out.insert("cover".into(), cover.to_json());
        }
        if let Some(at) = self.remote_updated_at {
            out.insert("remoteUpdatedAt".into(), json!(at));
        }
        Value::Object(out)
    }
}

/// Java's `String.trim()`: every char up to U+0020 at both ends
pub(crate) fn java_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c <= ' ')
}

fn blank_to_null(value: Option<&str>) -> Option<&str> {
    value.map(java_trim).filter(|v| !v.is_empty())
}

/// A deck link as Java's `java.net.URI` reads it, for the links both read the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// lower case
    pub host: String,
    pub path: String,
    pub raw_query: Option<String>,
}

enum Parsed {
    Link(Link),
    Failed(Failure),
    /// Java's reading could differ: left to Java
    Unsure,
}

fn parse(link: &str) -> Parsed {
    if link.encode_utf16().count() > 2048 {
        return Parsed::Failed(Failure::new(Reason::Unsupported, "Not a deck link"));
    }
    let trimmed = java_trim(link);
    let plain = |c: char| c.is_ascii_alphanumeric() || "-._~:/?#!$&'()*+,;=@".contains(c);
    if trimmed.is_empty() || !trimmed.chars().all(plain) || trimmed.matches('#').count() > 1 {
        return Parsed::Unsure;
    }
    let lower = trimmed.to_ascii_lowercase();
    let rest = if lower.starts_with("https://") {
        &trimmed[8..]
    } else if lower.starts_with("http://") {
        &trimmed[7..]
    } else if trimmed.contains(':') {
        // "https://" goes in front: "mtgtop8.com:443/…" and the like read as a port, or not
        return Parsed::Unsure;
    } else {
        trimmed
    };
    let without_fragment = rest.split('#').next().unwrap_or("");
    let authority_end = without_fragment.find(['/', '?']).unwrap_or(without_fragment.len());
    let (authority, after) = without_fragment.split_at(authority_end);
    if authority.contains('@') {
        // user info
        return Parsed::Unsure;
    }
    let host = match authority.split_once(':') {
        Some((host, port)) if !port.is_empty() && port.len() <= 5 && port.bytes().all(|b| b.is_ascii_digit()) => host,
        Some(_) => return Parsed::Unsure,
        None => authority,
    };
    let host = host.to_ascii_lowercase();
    if !Source::ALL.iter().any(|source| source.page_hosts().contains(&host.as_str())) {
        return Parsed::Unsure;
    }
    let (path, raw_query) = match after.split_once('?') {
        Some((path, query)) => (path, Some(query.to_string())),
        None => (after, None),
    };
    Parsed::Link(Link { host, path: path.to_string(), raw_query })
}

struct Cached {
    deck: Value,
    at: u64,
    used: u64,
}

#[derive(Default)]
struct Health {
    consecutive_failures: u32,
    degraded_until: u64,
    successes: u64,
    failures: u64,
}

pub struct DeckImports {
    fetcher: Box<dyn Fetcher>,
    clock: Box<dyn Fn() -> u64 + Send + Sync>,
    cache: Mutex<(HashMap<String, Cached>, u64)>,
    health: Mutex<HashMap<&'static str, Health>>,
    recent: Mutex<HashMap<String, VecDeque<u64>>>,
}

impl DeckImports {
    pub fn new() -> DeckImports {
        let hosts = Source::ALL.iter().flat_map(|s| s.fetch_hosts().iter().map(|h| h.to_string())).collect();
        DeckImports::with(Box::new(HttpFetcher::new(hosts)), Box::new(epoch_millis))
    }

    pub fn with(fetcher: Box<dyn Fetcher>, clock: Box<dyn Fn() -> u64 + Send + Sync>) -> DeckImports {
        DeckImports {
            fetcher,
            clock,
            cache: Mutex::new((HashMap::new(), 0)),
            health: Mutex::new(HashMap::new()),
            recent: Mutex::new(HashMap::new()),
        }
    }

    /// `deckImportFromUrl`; `session`: who is importing, for the per-connection limit
    pub async fn import(&self, link: &str, session: &str) -> Outcome {
        match self.import_deck(link, session).await {
            Some(Ok(deck)) => Outcome::Result(deck),
            Some(Err((reason, message))) => {
                Outcome::Error { code: DECK_IMPORT_FAILED, message, data: Some(json!({"reason": reason.wire_name()})) }
            }
            None => Outcome::Declined,
        }
    }

    async fn import_deck(&self, link: &str, session: &str) -> Option<Result<Value, (Reason, String)>> {
        let link = match parse(link) {
            Parsed::Link(link) => link,
            Parsed::Failed(Failure::Import(reason, message)) => return Some(Err((reason, message))),
            Parsed::Failed(Failure::Shape) | Parsed::Unsure => return None,
        };
        let source = Source::for_host(&link.host)?;
        let Some(deck_id) = source.deck_id(&link) else {
            return Some(Err((Reason::Unsupported, "Not a deck link the server reads".into())));
        };

        let key = format!("{}:{deck_id}", source.id());
        let now = (self.clock)();
        {
            let mut cache = self.cache.lock().unwrap();
            let (entries, counter) = &mut *cache;
            *counter += 1;
            if let Some(cached) = entries.get_mut(&key) {
                cached.used = *counter;
                if now.saturating_sub(cached.at) < CACHE_MILLIS {
                    return Some(Ok(cached.deck.clone()));
                }
            }
        }
        if !self.check_rate(session, now) {
            return Some(Err((Reason::RateLimited, "Too many deck imports in a minute".into())));
        }

        match source.fetch(&link, &deck_id, self.fetcher.as_ref()).await {
            Ok(deck) => {
                let deck = deck.to_json();
                let mut cache = self.cache.lock().unwrap();
                let (entries, counter) = &mut *cache;
                *counter += 1;
                entries.insert(key, Cached { deck: deck.clone(), at: now, used: *counter });
                if entries.len() > CACHE_SIZE {
                    let eldest = entries.iter().min_by_key(|(_, c)| c.used).map(|(k, _)| k.clone());
                    if let Some(eldest) = eldest {
                        entries.remove(&eldest);
                    }
                }
                drop(cache);
                self.record(source.id(), None);
                Some(Ok(deck))
            }
            Err(Failure::Import(reason, message)) => {
                self.record(source.id(), Some(reason));
                Some(Err((reason, message)))
            }
            Err(Failure::Shape) => {
                // a parser tripping over an unexpected shape is the site changing, never a server error
                self.record(source.id(), Some(Reason::SiteChanged));
                Some(Err((Reason::SiteChanged, format!("{} answered in a shape we don't read", source.id()))))
            }
        }
    }

    /// `deckImportSources`: every site the server reads, with whether it works right now
    pub fn sources(&self) -> Value {
        let now = (self.clock)();
        let health = self.health.lock().unwrap();
        Value::Array(
            Source::ALL
                .iter()
                .map(|source| {
                    let degraded = health.get(source.id()).is_some_and(|h| h.degraded_until > now);
                    json!({"site": source.id(), "status": if degraded { "degraded" } else { "ok" }})
                })
                .collect(),
        )
    }

    fn check_rate(&self, session: &str, now: u64) -> bool {
        let mut recent = self.recent.lock().unwrap();
        let times = recent.entry(session.to_string()).or_default();
        while times.front().is_some_and(|&first| now.saturating_sub(first) >= 60_000) {
            times.pop_front();
        }
        if times.len() >= IMPORTS_PER_MINUTE {
            return false;
        }
        times.push_back(now);
        true
    }

    fn record(&self, site: &'static str, failure: Option<Reason>) {
        let now = (self.clock)();
        let (successes, failures) = {
            let mut health = self.health.lock().unwrap();
            let entry = health.entry(site).or_default();
            match failure {
                None => {
                    entry.successes += 1;
                    entry.consecutive_failures = 0;
                    entry.degraded_until = 0;
                }
                Some(reason) => {
                    entry.failures += 1;
                    if matches!(reason, Reason::SiteChanged | Reason::Blocked) {
                        entry.consecutive_failures += 1;
                        if entry.consecutive_failures >= FAILURES_TO_DEGRADE {
                            entry.degraded_until = now + DEGRADED_MILLIS;
                        }
                    }
                }
            }
            (entry.successes, entry.failures)
        };
        // counts only: never deck contents, links or user names
        info!(
            "Deck import from {site}: {} ({successes} ok, {failures} failed since start)",
            failure.map(Reason::wire_name).unwrap_or("ok")
        );
    }
}

impl Default for DeckImports {
    fn default() -> Self {
        DeckImports::new()
    }
}

fn epoch_millis() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Recorded {
        body: Result<String, Failure>,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Fetcher for Recorded {
        async fn get(&self, _url: &str, _accept: &str) -> Result<String, Failure> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.body.clone()
        }
    }

    fn service(body: Result<String, Failure>) -> (DeckImports, Arc<AtomicUsize>, Arc<AtomicU64>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let clock = Arc::new(AtomicU64::new(1_000_000));
        let ticks = clock.clone();
        let imports = DeckImports::with(
            Box::new(Recorded { body, calls: calls.clone() }),
            Box::new(move || ticks.load(Ordering::Relaxed)),
        );
        (imports, calls, clock)
    }

    fn archidekt() -> String {
        sources::tests::fixture("archidekt/commander-7031486.json")
    }

    fn reason(outcome: &Outcome) -> Option<String> {
        match outcome {
            Outcome::Error { data: Some(data), .. } => data["reason"].as_str().map(str::to_string),
            _ => None,
        }
    }

    #[tokio::test]
    async fn caches_and_limits() {
        let (imports, calls, clock) = service(Ok(archidekt()));
        let first = imports.import("archidekt.com/decks/7031486/buffs", "s").await;
        assert!(matches!(first, Outcome::Result(_)));
        imports.import("https://www.archidekt.com/decks/7031486", "s").await;
        assert_eq!(calls.load(Ordering::Relaxed), 1, "the second import comes from the cache");

        for id in 1..=9 {
            imports.import(&format!("https://archidekt.com/decks/{id}"), "s").await;
        }
        let limited = imports.import("https://archidekt.com/decks/10", "s").await;
        assert_eq!(reason(&limited).as_deref(), Some("rate_limited"));
        assert!(matches!(imports.import("https://archidekt.com/decks/10", "other").await, Outcome::Result(_)));
        clock.fetch_add(60_000, Ordering::Relaxed);
        assert!(matches!(imports.import("https://archidekt.com/decks/11", "s").await, Outcome::Result(_)));
    }

    #[tokio::test]
    async fn degrades_after_three_changed_answers() {
        let (imports, _, clock) = service(Ok("<html>maintenance</html>".into()));
        for id in 1..=3 {
            let outcome = imports.import(&format!("https://archidekt.com/decks/{id}"), &id.to_string()).await;
            assert_eq!(reason(&outcome).as_deref(), Some("site_changed"));
        }
        assert_eq!(imports.sources()[0], json!({"site": "archidekt", "status": "degraded"}));
        assert_eq!(imports.sources()[1], json!({"site": "tcgplayer", "status": "ok"}));
        clock.fetch_add(DEGRADED_MILLIS, Ordering::Relaxed);
        assert_eq!(imports.sources()[0]["status"], "ok");
    }

    #[tokio::test]
    async fn links() {
        let (imports, calls, _) = service(Ok(archidekt()));
        let unsupported = imports.import("https://archidekt.com/search/decks", "s").await;
        assert_eq!(
            unsupported,
            Outcome::Error {
                code: DECK_IMPORT_FAILED,
                message: "Not a deck link the server reads".into(),
                data: Some(json!({"reason": "unsupported"}))
            }
        );
        for unsure in [
            "https://example.com/decks/1",
            "https://archidekt.com/decks/1 2",
            "https://archidekt.com/d%65cks/1",
            "https://user@archidekt.com/decks/1",
            "https://archidekt.com:x/decks/1",
            "https://archidekt.com/decks/1#a#b",
            "https://archidekt.com/decks/é",
        ] {
            assert_eq!(imports.import(unsure, "s").await, Outcome::Declined, "{unsure}");
        }
        let long = format!("https://archidekt.com/decks/1?{}", "a".repeat(2048));
        assert_eq!(reason(&imports.import(&long, "s").await).as_deref(), Some("unsupported"));
        assert!(matches!(imports.import("HTTPS://ARCHIDEKT.COM:443/decks/5#top", "s").await, Outcome::Result(_)));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
