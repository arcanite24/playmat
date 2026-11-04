//! The card catalog (docs/rust/Catalog.md): card views, set lists and the lookups `lookupCards` makes, read from the
//! SQLite file the Java server exports (`CatalogExport`). It answers as `CardSearchService` does, step for step.

use flate2::{Decompress, FlushDecompress, Status};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::HashMap;
use std::path::Path;

/// the export format this reads (`CatalogExport.VERSION`)
pub const VERSION: &str = "2";
/// `CardSearchService.MAX_LOOKUP`
pub const MAX_LOOKUP: usize = 500;

struct Set {
    code: String,
    standard_legal: bool,
    release_date: i64,
}

pub struct Catalog {
    pool: SqlitePool,
    dictionary: Vec<u8>,
    /// by code, as `ExpansionRepository.getSetByCode` finds them (ignoring case)
    sets: HashMap<String, Set>,
    expansion_sets: Value,
    basic_land_sets: Value,
    pub stamp: String,
    pub printings: u64,
}

/// One deck entry of `lookupCards` (`DeckCardInfo`).
#[derive(Debug, Default)]
pub struct Entry {
    pub name: Option<String>,
    pub set_code: Option<String>,
    pub number: Option<String>,
}

impl Entry {
    /// as Gson reads one: fields that aren't text count as missing
    pub fn from_json(value: &Value) -> Option<Entry> {
        let object = value.as_object()?;
        let text = |key: &str| object.get(key).and_then(Value::as_str).map(str::to_string);
        Some(Entry { name: text("cardName"), set_code: text("setCode"), number: text("cardNumber") })
    }
}

/// the database compares text ignoring case (`IGNORECASE=TRUE`), so the export's keys are lower case
fn lower(text: &str) -> String {
    text.to_lowercase()
}

impl Catalog {
    pub async fn open(path: &Path) -> anyhow::Result<Catalog> {
        let options = SqliteConnectOptions::new().filename(path).read_only(true).immutable(true);
        let pool = SqlitePoolOptions::new().max_connections(4).connect_with(options).await?;
        let meta: HashMap<String, Vec<u8>> = sqlx::query("SELECT key, value FROM meta")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .map(|row| (row.get::<String, _>(0), row.get::<Vec<u8>, _>(1)))
            .collect();
        let text = |key: &str| meta.get(key).map(|v| String::from_utf8_lossy(v).into_owned()).unwrap_or_default();
        let stamp = text("stamp");
        if stamp.split(':').next() != Some(VERSION) {
            anyhow::bail!("catalog format {stamp:?}, this reads {VERSION}");
        }
        let dictionary = meta.get("viewDictionary").cloned().unwrap_or_default();
        let sets = sqlx::query("SELECT code, standard_legal, release_date FROM sets")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .map(|row| {
                let code: String = row.get(0);
                (lower(&code), Set { code, standard_legal: row.get::<i64, _>(1) != 0, release_date: row.get(2) })
            })
            .collect();
        let result = |method: &'static str| {
            let pool = pool.clone();
            async move {
                let json: String = sqlx::query_scalar("SELECT json FROM results WHERE method = ?")
                    .bind(method)
                    .fetch_one(&pool)
                    .await?;
                anyhow::Ok(serde_json::from_str::<Value>(&json)?)
            }
        };
        let expansion_sets = result("getExpansionSets").await?;
        let basic_land_sets = result("getBasicLandSets").await?;
        let printings: i64 = sqlx::query_scalar("SELECT count(*) FROM printings").fetch_one(&pool).await?;
        Ok(Catalog { pool, dictionary, sets, expansion_sets, basic_land_sets, stamp, printings: printings as u64 })
    }

    pub fn expansion_sets(&self) -> Value {
        self.expansion_sets.clone()
    }

    pub fn basic_land_sets(&self) -> Value {
        self.basic_land_sets.clone()
    }

    /// `CardSearchService.lookup`: a view per entry, in order, null for unknown cards; at most 500. The lookups
    /// `findEntry` makes one by one are made here in a few queries for the whole list.
    pub async fn lookup(&self, entries: &[Option<Entry>]) -> anyhow::Result<Vec<Value>> {
        let entries = &entries[..entries.len().min(MAX_LOOKUP)];
        let field = |text: &Option<String>| text.as_deref().unwrap_or("").to_string();

        // 1. by set and number
        let mut keys: Vec<(String, String)> = entries
            .iter()
            .flatten()
            .map(|e| (lower(&field(&e.set_code)), lower(&field(&e.number))))
            .filter(|(set, number)| !set.is_empty() && !number.is_empty())
            .collect();
        keys.sort();
        keys.dedup();
        let exact = self.exact(&keys).await?;

        // 2. by name, for the entries that didn't match a printing
        let wants_name = |e: &Entry| {
            let key = (lower(&field(&e.set_code)), lower(&field(&e.number)));
            !exact.contains_key(&key) && !field(&e.name).is_empty()
        };
        // most entries name a card without a set: the printing Java's findPreferredCoreExpansionCard picks for
        // the name is exported (kind 2), so they need no list of printings
        let names_of = |preferred: bool| {
            let mut queries: Vec<String> = entries
                .iter()
                .flatten()
                .filter(|e| wants_name(e) && field(&e.set_code).is_empty() == preferred)
                .map(|e| lower(&field(&e.name)))
                .collect();
            queries.sort();
            queries.dedup();
            queries
        };
        let without_set = names_of(true);
        let preferred = self.names(&without_set, 2).await?;
        let with_set = names_of(false);
        let by_name = self.names(&with_set, 0).await?;
        // an unknown "A // B": the printings named A
        let mut halves: Vec<String> = with_set
            .iter()
            .filter(|q| !by_name.contains_key(*q))
            .chain(without_set.iter().filter(|q| !preferred.contains_key(*q)))
            .filter(|q| q.contains(" // "))
            .map(|q| q.split(" // ").next().unwrap_or("").to_string())
            .collect();
        halves.sort();
        halves.dedup();
        let by_first_half = self.names(&halves, 1).await?;

        let no_cards = Vec::new();
        let found: Vec<Option<i64>> = entries
            .iter()
            .map(|entry| {
                let entry = entry.as_ref()?;
                let key = (lower(&field(&entry.set_code)), lower(&field(&entry.number)));
                if let Some(id) = exact.get(&key) {
                    return Some(*id);
                }
                let name = lower(&field(&entry.name));
                if name.is_empty() {
                    return None;
                }
                if field(&entry.set_code).is_empty() {
                    if let Some(found) = preferred.get(&name) {
                        return found.first().map(|(id, _)| *id);
                    }
                    if !name.contains(" // ") {
                        return None;
                    }
                    // an unknown "A // B" without a set: the list of A's printings, below
                }
                let cards = by_name.get(&name).unwrap_or_else(|| {
                    if name.contains(" // ") {
                        by_first_half.get(name.split(" // ").next().unwrap_or("")).unwrap_or(&no_cards)
                    } else {
                        &no_cards
                    }
                });
                self.preferred_or_latest(cards, &field(&entry.set_code))
            })
            .collect();

        let mut ids: Vec<i64> = found.iter().flatten().copied().collect();
        ids.sort_unstable();
        ids.dedup();
        let views = self.views(&ids).await?;
        // each card gets ids of its own, as Java's views do
        found
            .iter()
            .map(|id| match id {
                None => Ok(Value::Null),
                Some(id) => views.get(id).map(fresh_ids).ok_or_else(|| anyhow::anyhow!("printing {id} has no view")),
            })
            .collect()
    }

    /// `CardRepository.findCard(set, number)` for each key
    async fn exact(&self, keys: &[(String, String)]) -> anyhow::Result<HashMap<(String, String), i64>> {
        let mut out = HashMap::new();
        for chunk in keys.chunks(400) {
            let sql = format!(
                "SELECT set_code, card_number, printing FROM exact WHERE (set_code, card_number) IN (VALUES {})",
                vec!["(?, ?)"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(&sql);
            for (set, number) in chunk {
                query = query.bind(set).bind(number);
            }
            for row in query.fetch_all(&self.pool).await? {
                out.insert((row.get(0), row.get(1)), row.get(2));
            }
        }
        Ok(out)
    }

    /// the printings each query finds, in the export's order (kind 0: `findCards(query)`; kind 1: named exactly that)
    async fn names(&self, queries: &[String], kind: i64) -> anyhow::Result<HashMap<String, Vec<(i64, String)>>> {
        let mut out: HashMap<String, Vec<(i64, String)>> = HashMap::new();
        for chunk in queries.chunks(500) {
            let sql = format!(
                "SELECT n.query, n.printing, p.set_code FROM names n JOIN printings p ON p.id = n.printing \
                 WHERE n.kind = ? AND n.query IN ({}) ORDER BY n.query, n.position",
                vec!["?"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(&sql).bind(kind);
            for name in chunk {
                query = query.bind(name);
            }
            for row in query.fetch_all(&self.pool).await? {
                out.entry(row.get(0)).or_default().push((row.get(1), row.get(2)));
            }
        }
        Ok(out)
    }

    /// `CardRepository.findPreferredOrLatestCard`: the preferred set's printing, else the newest standard-legal
    /// set's, else the newest
    fn preferred_or_latest(&self, cards: &[(i64, String)], preferred_set: &str) -> Option<i64> {
        let mut last_release: Option<i64> = None;
        let mut last_expansion: Option<i64> = None;
        let mut use_card = None;
        for (id, set_code) in cards {
            let Some(set) = self.sets.get(&lower(set_code)) else { continue };
            if preferred_set == set.code {
                return Some(*id);
            }
            if set.standard_legal && last_expansion.is_none_or(|date| set.release_date > date) {
                use_card = Some(*id);
                last_expansion = Some(set.release_date);
            }
            if last_expansion.is_none() && last_release.is_none_or(|date| set.release_date > date) {
                use_card = Some(*id);
                last_release = Some(set.release_date);
            }
        }
        use_card
    }

    async fn views(&self, ids: &[i64]) -> anyhow::Result<HashMap<i64, Value>> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(500) {
            let sql = format!("SELECT id, view FROM printings WHERE id IN ({})", vec!["?"; chunk.len()].join(", "));
            let mut query = sqlx::query(&sql);
            for id in chunk {
                query = query.bind(id);
            }
            for row in query.fetch_all(&self.pool).await? {
                let compressed: Vec<u8> = row.get(1);
                out.insert(row.get(0), serde_json::from_slice(&inflate(&compressed, &self.dictionary)?)?);
            }
        }
        Ok(out)
    }
}

/// raw deflate with a preset dictionary (`CatalogExport.Printings.compress`)
fn inflate(compressed: &[u8], dictionary: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut inflater = Decompress::new(false);
    if !dictionary.is_empty() {
        inflater.set_dictionary(dictionary)?;
    }
    let mut out = Vec::with_capacity(compressed.len() * 12);
    loop {
        out.reserve(16 * 1024);
        let (read, written) = (inflater.total_in(), inflater.total_out());
        let status = inflater.decompress_vec(&compressed[read as usize..], &mut out, FlushDecompress::Finish)?;
        if status == Status::StreamEnd {
            return Ok(out);
        }
        if inflater.total_in() == read && inflater.total_out() == written {
            anyhow::bail!("card view cut short");
        }
    }
}

/// the view with every UUID replaced by a new one (the same new one wherever the same id appears)
fn fresh_ids(view: &Value) -> Value {
    fn walk(value: &Value, ids: &mut HashMap<String, String>) -> Value {
        match value {
            Value::String(text) if playmat_protocol::compare::is_uuid(text) => {
                Value::String(ids.entry(text.clone()).or_insert_with(|| uuid::Uuid::new_v4().to_string()).clone())
            }
            Value::Array(items) => Value::Array(items.iter().map(|item| walk(item, ids)).collect()),
            Value::Object(fields) => {
                Value::Object(fields.iter().map(|(key, value)| (key.clone(), walk(value, ids))).collect())
            }
            other => other.clone(),
        }
    }
    walk(view, &mut HashMap::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compress, Compression, FlushCompress};

    #[test]
    fn inflates_with_the_dictionary() {
        let dictionary = br#"{"name":"Lightning Bolt","rules":["deals 3 damage"]}"#;
        let view = br#"{"name":"Lightning Strike","rules":["deals 3 damage to any target"]}"#;
        let mut deflater = Compress::new(Compression::best(), false);
        deflater.set_dictionary(dictionary).unwrap();
        let mut compressed = Vec::with_capacity(256);
        deflater.compress_vec(view, &mut compressed, FlushCompress::Finish).unwrap();
        assert_eq!(inflate(&compressed, dictionary).unwrap(), view);
    }

    #[test]
    fn new_ids_keep_their_pairs() {
        let id = "0b6f2c1e-3d4a-4e5f-8a9b-1c2d3e4f5a6b";
        let view = serde_json::json!({"cardId": id, "second": {"cardId": id, "parentId": "x"}, "name": "Shock"});
        let fresh = fresh_ids(&view);
        assert_ne!(fresh["cardId"], view["cardId"]);
        assert_eq!(fresh["cardId"], fresh["second"]["cardId"]);
        assert_eq!(fresh["second"]["parentId"], "x");
        assert_eq!(fresh["name"], "Shock");
    }
}
