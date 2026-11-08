//! The card catalog (docs/rust/Catalog.md): card views, set lists and the lookups `lookupCards` makes, read from the
//! SQLite file the Java server exports (`CatalogExport`). It answers as `CardSearchService` does, step for step.

use flate2::{Decompress, FlushDecompress, Status};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::HashMap;
use std::path::Path;

/// the export format this reads (`CatalogExport.VERSION`)
pub const VERSION: &str = "3";
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
    /// the card rows, for `searchCards`
    cards: crate::search::Cards,
    /// the formats' rules, for `deckValidate`
    formats: crate::validate::Formats,
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
        let cards = crate::search::Cards::load(&pool).await?;
        let formats = crate::validate::Formats::load(&pool).await?;
        Ok(Catalog {
            pool,
            dictionary,
            sets,
            expansion_sets,
            basic_land_sets,
            cards,
            formats,
            stamp,
            printings: printings as u64,
        })
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

    /// `CardSearchService.search`: the views of what the criteria find, or None for a case left to Java
    pub async fn search(&self, criteria: &Value, filters: Option<&Value>) -> anyhow::Result<Option<Vec<Value>>> {
        let ids = match self.cards.search(criteria, filters) {
            crate::search::Found::Printings(ids) => ids,
            crate::search::Found::Declined(why) => {
                tracing::debug!("searchCards left to Java: {why}");
                return Ok(None);
            }
        };
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        let views = self.views(&unique).await?;
        ids.iter()
            .map(|id| views.get(id).map(fresh_ids).ok_or_else(|| anyhow::anyhow!("printing {id} has no view")))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(Some)
    }

    /// `DeckValidationService.validate`: Ok(None) for a case left to Java, Err(message) for an unknown format
    pub async fn validate(&self, deck_type: &str, deck: &Value) -> anyhow::Result<Result<Option<Value>, String>> {
        use crate::validate::{deck_lists, Card};
        let Some(format) = self.formats.get(deck_type) else {
            return Ok(Err(format!("Unknown deck type: {deck_type}")));
        };
        if !format.plain {
            return Ok(Ok(None));
        }
        let Some((main, side)) = deck_lists(deck) else { return Ok(Ok(None)) };
        // `Deck.load`: each copy, the printing a set and number name, else the one preferred for the name
        let mut total = 0usize;
        let mut wanted: Vec<(bool, i64)> = Vec::new();
        let mut keys: Vec<(String, String)> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for (sideboard, list) in [(false, &main), (true, &side)] {
            'entries: for entry in list.iter() {
                for _ in 0..entry.amount.max(0) {
                    if total > crate::validate::MAX_CARDS_PER_DECK {
                        break 'entries;
                    }
                    let set = entry.set_code.as_deref().unwrap_or("");
                    let number = entry.number.as_deref().unwrap_or("");
                    if !set.is_empty() && !number.is_empty() {
                        keys.push((lower(set), lower(number)));
                        wanted.push((sideboard, -(keys.len() as i64)));
                    } else {
                        let Some(name) = entry.name.as_deref() else { return Ok(Ok(None)) };
                        names.push(lower(name));
                        wanted.push((sideboard, names.len() as i64));
                    }
                    total += 1;
                }
            }
        }
        let mut unique_keys = keys.clone();
        unique_keys.sort();
        unique_keys.dedup();
        let exact = self.exact(&unique_keys).await?;
        let mut unique_names = names.clone();
        unique_names.sort();
        unique_names.dedup();
        let preferred = self.names(&unique_names, 2).await?;
        let mut ids = Vec::with_capacity(wanted.len());
        for (sideboard, at) in &wanted {
            let id = if *at < 0 {
                exact.get(&keys[(-at - 1) as usize]).copied()
            } else {
                preferred.get(&names[(*at - 1) as usize]).and_then(|found| found.first()).map(|(id, _)| *id)
            };
            // an unknown card: Java's Deck.load refuses the deck, with its own message
            let Some(id) = id else { return Ok(Ok(None)) };
            ids.push((*sideboard, id));
        }
        let mut unique: Vec<i64> = ids.iter().map(|(_, id)| *id).collect();
        unique.sort_unstable();
        unique.dedup();
        let mut info: HashMap<i64, Card> = HashMap::new();
        for chunk in unique.chunks(500) {
            let sql = format!(
                "SELECT id, name, set_code, rarity, extra_deck, mass_land_destruction, extra_turn FROM printings \
                 WHERE id IN ({})",
                vec!["?"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(&sql);
            for id in chunk {
                query = query.bind(id);
            }
            for row in query.fetch_all(&self.pool).await? {
                info.insert(
                    row.get(0),
                    Card {
                        name: row.get(1),
                        set_code: row.get(2),
                        rarity: row.get(3),
                        extra_deck: row.get::<i64, _>(4) != 0,
                        mass_land_destruction: row.get::<i64, _>(5) != 0,
                        extra_turn: row.get::<i64, _>(6) != 0,
                    },
                );
            }
        }
        let (mut cards, mut sideboard) = (Vec::new(), Vec::new());
        for (side, id) in &ids {
            let card = info.get(id).ok_or_else(|| anyhow::anyhow!("printing {id} is missing"))?.clone();
            if *side {
                sideboard.push(card)
            } else {
                cards.push(card)
            }
        }
        // `CardRepository.findCards(name)` for the cards whose rarity or set the format checks
        let mut printings_of: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut card_names: Vec<String> = cards
            .iter()
            .chain(&sideboard)
            .filter(|c| crate::validate::Formats::needs_printings(format, c))
            .map(|c| c.name.clone())
            .collect();
        card_names.sort();
        card_names.dedup();
        for chunk in card_names.chunks(400) {
            let sql = format!(
                "SELECT n.query, p.set_code, p.rarity FROM names n JOIN printings p ON p.id = n.printing \
                 WHERE n.kind = 0 AND n.query IN ({}) ORDER BY n.query, n.position",
                vec!["?"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(&sql);
            for name in chunk {
                query = query.bind(lower(name));
            }
            let by_query: HashMap<String, &String> = chunk.iter().map(|n| (lower(n), n)).collect();
            for row in query.fetch_all(&self.pool).await? {
                let q: String = row.get(0);
                if let Some(name) = by_query.get(&q) {
                    printings_of.entry((*name).clone()).or_default().push((row.get(1), row.get(2)));
                }
            }
        }
        Ok(Ok(Some(self.formats.validate(deck_type, format, &cards, &sideboard, &printings_of))))
    }

    /// `CardRepository.findCard(set, number)` for the Career: the printing's name, set, number, rarity (the
    /// `Rarity` name) and whether it has the basic supertype
    pub async fn printing(
        &self,
        set: &str,
        number: &str,
    ) -> anyhow::Result<Option<(String, String, String, String, bool)>> {
        let row = sqlx::query(
            "SELECT p.name, p.set_code, p.card_number, p.rarity, \
             (SELECT c.supertypes FROM cards c WHERE c.printing = p.id ORDER BY c.ord LIMIT 1) \
             FROM exact e JOIN printings p ON p.id = e.printing WHERE e.set_code = ? AND e.card_number = ?",
        )
        .bind(lower(set))
        .bind(lower(number))
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| {
            let supertypes: Option<String> = row.get(4);
            (row.get(0), row.get(1), row.get(2), row.get(3), supertypes.is_some_and(|s| s.contains("BASIC")))
        }))
    }

    pub fn card_rows(&self) -> usize {
        self.cards.row_count()
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
