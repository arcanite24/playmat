//! `searchCards` (docs/rust/Catalog.md): the card rows the catalog exports, filtered in memory as
//! `CardSearchService.search` does: the query `CardCriteria.buildQuery` makes against the H2 card database, then the
//! web refinements and `CardTextPredicate`. A case this doesn't cover is declined, and Java answers it.

use serde_json::Value;
use sqlx::sqlite::SqlitePool;
use sqlx::Row as _;
use std::collections::{HashMap, HashSet};

/// `CardSearchService.DEFAULT_PAGE_SIZE` and `MAX_PAGE_SIZE`
const DEFAULT_PAGE: i64 = 100;
const MAX_PAGE: i64 = 1000;

const WHITE: u8 = 1;
const BLUE: u8 = 2;
const BLACK: u8 = 4;
const RED: u8 = 8;
const GREEN: u8 = 16;
const COLORS: [(u8, &str); 5] = [(WHITE, "white"), (BLUE, "blue"), (BLACK, "black"), (RED, "red"), (GREEN, "green")];

/// a piece of `Cards::text`
#[derive(Clone, Copy)]
struct Text {
    start: u32,
    len: u32,
}

const NULL: Text = Text { start: u32::MAX, len: 0 };

/// A card row (`CardInfo`), with what a search filters on. Its text is kept in one string for all rows.
pub struct Row {
    pub printing: i64,
    name_rank: u32,
    card_number_int: i32,
    mana_value: i32,
    name: Text,
    set_code: Text,
    rarity: Text,
    types: Text,
    subtypes: Text,
    supertypes: Text,
    /// lower case: what `CardTextPredicate` looks for a token in
    search_names: Text,
    search_rules: Text,
    search_types: Text,
    colors: u8,
    identity: u8,
    various_art: bool,
    double_faced: bool,
}

/// An enum constant searchCards' JSON names (`CardType`, `SuperType`, `SubType`, `Rarity`).
struct Constant {
    /// what the database matches: the name, or a subtype's text
    text: String,
    include_in_search: bool,
}

pub struct Cards {
    rows: Vec<Row>,
    text: String,
    /// kind → name → constant, and the names in order
    enums: HashMap<String, Vec<(String, Constant)>>,
    formats: HashSet<String>,
    format_sets: HashMap<String, Vec<String>>,
}

/// What a search answers: the printings to send, in order, or a case left to Java.
#[derive(Debug, PartialEq)]
pub enum Found {
    Printings(Vec<i64>),
    Declined(&'static str),
}

impl Cards {
    pub async fn load(pool: &SqlitePool) -> anyhow::Result<Cards> {
        let mut text = String::new();
        let mut put = |value: Option<String>| -> Text {
            match value {
                None => NULL,
                Some(value) => {
                    let start = text.len() as u32;
                    text.push_str(&value);
                    Text { start, len: value.len() as u32 }
                }
            }
        };
        let mut rows = Vec::new();
        {
            use futures_util::TryStreamExt;
            let mut stream = sqlx::query(
                "SELECT printing, name_rank, name, set_code, card_number_int, rarity, types, subtypes, supertypes, \
                 mana_value, colors, identity, various_art, double_faced, search_names, search_rules, search_types \
                 FROM cards ORDER BY ord",
            )
            .fetch(pool);
            while let Some(r) = stream.try_next().await? {
                rows.push(Row {
                    printing: r.get(0),
                    name_rank: r.get::<i64, _>(1) as u32,
                    name: put(r.get(2)),
                    set_code: put(r.get(3)),
                    card_number_int: r.get::<i64, _>(4) as i32,
                    rarity: put(r.get(5)),
                    types: put(r.get(6)),
                    subtypes: put(r.get(7)),
                    supertypes: put(r.get(8)),
                    mana_value: r.get::<i64, _>(9) as i32,
                    colors: r.get::<i64, _>(10) as u8,
                    identity: r.get::<i64, _>(11) as u8,
                    various_art: r.get::<i64, _>(12) != 0,
                    double_faced: r.get::<i64, _>(13) != 0,
                    search_names: put(r.get(14)),
                    search_rules: put(r.get(15)),
                    search_types: put(r.get(16)),
                });
            }
        }
        text.shrink_to_fit();
        rows.shrink_to_fit();
        let mut enums: HashMap<String, Vec<(String, Constant)>> = HashMap::new();
        for r in sqlx::query("SELECT kind, name, text, include_in_search FROM enums ORDER BY kind, position")
            .fetch_all(pool)
            .await?
        {
            enums
                .entry(r.get(0))
                .or_default()
                .push((r.get(1), Constant { text: r.get(2), include_in_search: r.get::<i64, _>(3) != 0 }));
        }
        let formats = sqlx::query_scalar("SELECT name FROM formats").fetch_all(pool).await?.into_iter().collect();
        let mut format_sets: HashMap<String, Vec<String>> = HashMap::new();
        for r in
            sqlx::query("SELECT format, set_code FROM format_sets ORDER BY format, position").fetch_all(pool).await?
        {
            format_sets.entry(r.get(0)).or_default().push(r.get(1));
        }
        Ok(Cards { rows, text, enums, formats, format_sets })
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, t: Text) -> Option<&str> {
        (t.start != u32::MAX).then(|| &self.text[t.start as usize..(t.start + t.len) as usize])
    }

    fn str(&self, t: Text) -> &str {
        self.get(t).unwrap_or("")
    }

    fn constant(&self, kind: &str, name: &str) -> Option<&Constant> {
        self.enums.get(kind)?.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    fn all(&self, kind: &str) -> Vec<&str> {
        self.enums.get(kind).map(|v| v.iter().map(|(n, _)| n.as_str()).collect()).unwrap_or_default()
    }

    /// `CardSearchService.search(criteria, filters)`
    pub fn search(&self, criteria: &Value, filters: Option<&Value>) -> Found {
        let mut c = match Criteria::from_json(criteria, self) {
            Ok(c) => c,
            Err(why) => return Found::Declined(why),
        };
        let identity = match filters {
            None | Some(Value::Null) => None,
            Some(Value::Object(f)) => match f.get("colorIdentity") {
                None | Some(Value::Null) => None,
                Some(Value::String(letters)) => Some(identity_of(letters)),
                Some(_) => return Found::Declined("filters.colorIdentity isn't text"),
            },
            Some(_) => return Found::Declined("filters isn't an object"),
        };
        let start = c.start.map(|s| s.max(0)).unwrap_or(0);
        let count = c.count.map(|n| n.clamp(0, MAX_PAGE)).unwrap_or(DEFAULT_PAGE);

        let text_search = c.search_text.as_deref().is_some_and(|t| !java_trim(t).is_empty())
            && (c.search_names || c.search_types || c.search_rules);
        let mana_value = c.mana_value;
        let operator = c.mana_value_operator.clone();
        let refinements = text_search
            || identity.is_some()
            || c.unique_names
            || !c.web_colors.is_empty()
            || !c.web_excluded_colors.is_empty()
            || !c.excluded_rarities.is_empty()
            || (mana_value.is_some() && operator.as_deref().is_some_and(|o| o != "eq"));
        if !refinements && count == 0 {
            return Found::Declined("a database page of 0");
        }

        if let Some(format) = c.format.as_deref().filter(|f| !f.is_empty()) {
            if self.formats.contains(format) {
                if let Some(codes) = self.format_sets.get(format) {
                    c.set_codes.extend(codes.iter().cloned());
                }
            }
        }
        // the database only matches mana value exactly
        if mana_value.is_some() && operator.as_deref().is_some_and(|o| o != "eq") {
            c.mana_value = None;
        }
        let query = match c.query(self) {
            Ok(q) => q,
            Err(why) => return Found::Declined(why),
        };

        let mut rows: Vec<&Row> = self.rows.iter().filter(|r| query.matches(self, r)).collect();
        if c.sort_by.is_some() {
            rows.sort_by_key(|r| r.name_rank);
        }
        if !refinements {
            return Found::Printings(
                rows.into_iter().skip(start as usize).take(count as usize).map(|r| r.printing).collect(),
            );
        }

        let text = if text_search {
            Some(TextPredicate::new(
                c.search_text.as_deref().unwrap_or(""),
                c.search_names,
                c.search_types,
                c.search_rules,
            ))
        } else {
            None
        };
        if text.as_ref().is_some_and(|t| t.tokens.iter().any(|token| token.contains('@'))) {
            return Found::Declined("a search token with @");
        }
        let web_colors = normalize(&c.web_colors);
        let excluded_colors = normalize(&c.web_excluded_colors);
        let mut seen_text: HashSet<&str> = HashSet::new();
        let spaced = text.as_ref().is_some_and(|t| t.tokens.iter().any(|token| token.contains(' ')));
        let mut seen: HashSet<&str> = HashSet::new();
        let mut out = Vec::new();
        let mut skipped = 0i64;
        for row in rows {
            if out.len() as i64 >= count {
                break;
            }
            if let Some(text) = &text {
                // CardTextPredicate keeps its own list of names found when the search is for unique names
                if !text.applies(self, row, spaced) {
                    continue;
                }
                if c.unique_names && !seen_text.insert(self.str(row.name)) {
                    continue;
                }
            }
            let card_colors = colors_of(row.colors);
            let refined = matches_colors(&card_colors, &web_colors, c.color_match.as_deref())
                && !matches_any_color(&card_colors, &excluded_colors)
                && !c.excluded_rarities.iter().any(|r| r == self.str(row.rarity))
                && matches_mana_value(row.mana_value, mana_value, operator.as_deref())
                && identity.is_none_or(|allowed| row.colors & !allowed == 0 && row.identity & !allowed == 0);
            if !refined {
                continue;
            }
            if c.unique_names && !seen.insert(self.str(row.name)) {
                continue;
            }
            if skipped < start {
                skipped += 1;
                continue;
            }
            out.push(row.printing);
        }
        Found::Printings(out)
    }
}

/// `CardCriteria` as Gson reads it: the constructor's defaults, then the fields the JSON has.
#[derive(Debug, Clone)]
struct Criteria {
    name_contains: Option<String>,
    name: Option<String>,
    set_codes: Vec<String>,
    ignore_set_codes: Vec<String>,
    types: Vec<String>,
    not_types: Vec<String>,
    supertypes: Vec<String>,
    not_supertypes: Vec<String>,
    /// as the database has them (`SubType.toString`)
    subtypes: Vec<String>,
    rarities: Vec<String>,
    excluded_rarities: Vec<String>,
    web_colors: Vec<Option<String>>,
    web_excluded_colors: Vec<Option<String>>,
    various_art: Option<bool>,
    double_faced: Option<bool>,
    black: bool,
    blue: bool,
    green: bool,
    red: bool,
    white: bool,
    colorless: bool,
    mana_value: Option<i64>,
    sort_by: Option<String>,
    start: Option<i64>,
    count: Option<i64>,
    search_text: Option<String>,
    search_names: bool,
    search_types: bool,
    search_rules: bool,
    unique_names: bool,
    color_match: Option<String>,
    mana_value_operator: Option<String>,
    min_card_number: i64,
    max_card_number: i64,
    format: Option<String>,
}

type Why = &'static str;

impl Criteria {
    fn from_json(value: &Value, cards: &Cards) -> Result<Criteria, Why> {
        let Value::Object(o) = value else { return Err("criteria isn't an object") };
        let string = |key: &str| -> Result<Option<String>, Why> {
            match o.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(s)) => Ok(Some(s.clone())),
                Some(_) => Err("a text field that isn't text"),
            }
        };
        let boolean = |key: &str| -> Result<Option<bool>, Why> {
            match o.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Bool(b)) => Ok(Some(*b)),
                Some(_) => Err("a true/false field that isn't"),
            }
        };
        let long = |key: &str| -> Result<Option<i64>, Why> {
            match o.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Number(n)) => n.as_i64().map(Some).ok_or("a number that isn't a whole one"),
                Some(_) => Err("a number field that isn't a number"),
            }
        };
        let int = |key: &str| -> Result<Option<i64>, Why> {
            let v = long(key)?;
            match v {
                Some(n) if n < i32::MIN as i64 || n > i32::MAX as i64 => Err("a number out of int range"),
                other => Ok(other),
            }
        };
        let texts = |key: &str| -> Result<Vec<String>, Why> {
            match o.get(key) {
                None => Ok(Vec::new()),
                Some(Value::Array(items)) => items
                    .iter()
                    .map(|i| i.as_str().map(str::to_string).ok_or("a list of text with something else in it"))
                    .collect(),
                Some(_) => Err("a list field that isn't a list"),
            }
        };
        let optional_texts = |key: &str| -> Result<Vec<Option<String>>, Why> {
            match o.get(key) {
                None => Ok(Vec::new()),
                Some(Value::Array(items)) => items
                    .iter()
                    .map(|i| match i {
                        Value::Null => Ok(None),
                        Value::String(s) => Ok(Some(s.clone())),
                        _ => Err("a list of text with something else in it"),
                    })
                    .collect(),
                Some(_) => Err("a list field that isn't a list"),
            }
        };
        let constants = |key: &str, kind: &str| -> Result<Vec<String>, Why> {
            texts(key)?
                .into_iter()
                .map(|name| cards.constant(kind, &name).map(|c| c.text.clone()).ok_or("an unknown enum constant"))
                .collect()
        };
        // a primitive keeps its default when the JSON has null
        let flag = |key: &str, default: bool| boolean(key).map(|v| v.unwrap_or(default));
        if o.get("nightCard") != Some(&Value::Bool(false)) && o.contains_key("nightCard") {
            return Err("nightCard other than false (night faces aren't exported)");
        }
        if o.get("modalDoubleFaced").is_some_and(|v| !v.is_null()) {
            return Err("modalDoubleFaced (its column isn't in the database)");
        }
        if o.get("rules").is_some_and(|v| !v.is_null()) {
            return Err("rules (the rules column isn't kept)");
        }
        let sort_by = string("sortBy")?;
        if sort_by.as_deref().is_some_and(|s| s != "name") {
            return Err("a sort other than by name");
        }
        Ok(Criteria {
            name_contains: string("nameContains")?,
            name: string("name")?,
            set_codes: texts("setCodes")?,
            ignore_set_codes: texts("ignoreSetCodes")?,
            types: constants("types", "CardType")?,
            not_types: constants("notTypes", "CardType")?,
            supertypes: constants("supertypes", "SuperType")?,
            not_supertypes: constants("notSupertypes", "SuperType")?,
            subtypes: constants("subtypes", "SubType")?,
            rarities: constants("rarities", "Rarity")?,
            excluded_rarities: constants("excludedRarities", "Rarity")?,
            web_colors: optional_texts("webColors")?,
            web_excluded_colors: optional_texts("webExcludedColors")?,
            various_art: boolean("variousArt")?,
            double_faced: boolean("doubleFaced")?,
            black: flag("black", true)?,
            blue: flag("blue", true)?,
            green: flag("green", true)?,
            red: flag("red", true)?,
            white: flag("white", true)?,
            colorless: flag("colorless", true)?,
            mana_value: int("manaValue")?,
            sort_by,
            start: long("start")?,
            count: long("count")?,
            search_text: string("searchText")?,
            search_names: flag("searchNames", false)?,
            search_types: flag("searchTypes", false)?,
            search_rules: flag("searchRules", false)?,
            unique_names: flag("uniqueNames", false)?,
            color_match: string("colorMatch")?,
            mana_value_operator: string("manaValueOperator")?,
            min_card_number: int("minCardNumber")?.unwrap_or(i32::MIN as i64),
            max_card_number: int("maxCardNumber")?.unwrap_or(i32::MAX as i64),
            format: string("format")?,
        })
    }

    /// `optimize()` then `buildQuery()`: the WHERE clause as a filter
    fn query(&self, cards: &Cards) -> Result<Query, Why> {
        let mut c = self.clone();
        let all_rarities = cards.all("Rarity");
        if !c.rarities.is_empty() && all_rarities.iter().all(|r| c.rarities.iter().any(|x| x == r)) {
            c.rarities.clear();
        }
        if c.black && c.blue && c.green && c.red && c.white && c.colorless {
            (c.black, c.blue, c.green, c.red, c.white, c.colorless) = (false, false, false, false, false, false);
        }
        if !c.types.is_empty() {
            let unused_in_search = cards
                .enums
                .get("CardType")
                .map(|all| all.iter().any(|(n, k)| k.include_in_search && !c.types.contains(n)))
                .unwrap_or(false);
            if !unused_in_search {
                c.types.clear();
            }
        }
        // "if all types selected - no selection needed": the literal 7 in buildQuery
        let types = if c.types.len() != 7 { c.types.clone() } else { Vec::new() };
        let mut colors = 0u8;
        if c.black {
            colors |= BLACK;
        }
        if c.blue {
            colors |= BLUE;
        }
        if c.green {
            colors |= GREEN;
        }
        if c.red {
            colors |= RED;
        }
        if c.white {
            colors |= WHITE;
        }
        Ok(Query {
            name_contains: c.name_contains.map(|t| like_pattern(&format!("%{t}%"))),
            name: c.name,
            various_art: c.various_art,
            double_faced: c.double_faced,
            rarities: c.rarities,
            set_codes: c.set_codes,
            ignore_set_codes: c.ignore_set_codes,
            types: types.iter().map(|t| like_pattern(&format!("%{t}%"))).collect(),
            not_types: c.not_types.iter().map(|t| like_pattern(&format!("%{t}%"))).collect(),
            supertypes: c.supertypes.iter().map(|t| like_pattern(&format!("%{t}%"))).collect(),
            not_supertypes: c.not_supertypes.iter().map(|t| like_pattern(&format!("%{t}%"))).collect(),
            subtypes: c.subtypes.iter().map(|t| like_pattern(&format!("%{t}%"))).collect(),
            mana_value: c.mana_value,
            colors,
            colorless: c.colorless,
            min_card_number: c.min_card_number,
            max_card_number: c.max_card_number,
        })
    }
}

/// The WHERE clause `buildQuery` makes (always with `splitCardHalf = false` and, by default, `nightCard = false`:
/// the rows the catalog exports).
struct Query {
    name_contains: Option<Vec<Like>>,
    name: Option<String>,
    various_art: Option<bool>,
    double_faced: Option<bool>,
    rarities: Vec<String>,
    set_codes: Vec<String>,
    ignore_set_codes: Vec<String>,
    types: Vec<Vec<Like>>,
    not_types: Vec<Vec<Like>>,
    supertypes: Vec<Vec<Like>>,
    not_supertypes: Vec<Vec<Like>>,
    subtypes: Vec<Vec<Like>>,
    mana_value: Option<i64>,
    /// any of these colors set, or colorless
    colors: u8,
    colorless: bool,
    min_card_number: i64,
    max_card_number: i64,
}

impl Query {
    fn matches(&self, cards: &Cards, r: &Row) -> bool {
        // SQL: a LIKE on NULL is NULL, and so is NOT LIKE: neither keeps the row
        let like = |pattern: &[Like], value: Text| cards.get(value).is_some_and(|v| like_matches(pattern, v));
        let not_like = |pattern: &[Like], value: Text| cards.get(value).is_some_and(|v| !like_matches(pattern, v));
        let name = cards.str(r.name);
        let set_code = cards.str(r.set_code);
        if let Some(p) = &self.name_contains {
            if !like_matches(p, name) {
                return false;
            }
        }
        if let Some(wanted) = &self.name {
            if !equals_ignore_case(name, wanted) {
                return false;
            }
        }
        if self.various_art.is_some_and(|v| v != r.various_art)
            || self.double_faced.is_some_and(|v| v != r.double_faced)
        {
            return false;
        }
        if !self.rarities.is_empty() && !self.rarities.iter().any(|x| equals_ignore_case(cards.str(r.rarity), x)) {
            return false;
        }
        if !self.set_codes.is_empty() && !self.set_codes.iter().any(|x| equals_ignore_case(set_code, x)) {
            return false;
        }
        if self.ignore_set_codes.iter().any(|x| equals_ignore_case(set_code, x)) {
            return false;
        }
        if !self.types.is_empty() && !self.types.iter().any(|p| like(p, r.types)) {
            return false;
        }
        if !self.not_types.iter().all(|p| not_like(p, r.types))
            || !self.supertypes.iter().all(|p| like(p, r.supertypes))
            || !self.not_supertypes.iter().all(|p| not_like(p, r.supertypes))
            || !self.subtypes.iter().all(|p| like(p, r.subtypes))
        {
            return false;
        }
        if self.mana_value.is_some_and(|v| v != r.mana_value as i64) {
            return false;
        }
        if (self.colors != 0 || self.colorless) && r.colors & self.colors == 0 && !(self.colorless && r.colors == 0) {
            return false;
        }
        if self.min_card_number != i32::MIN as i64 && (r.card_number_int as i64) < self.min_card_number {
            return false;
        }
        if self.max_card_number != i32::MAX as i64 && (r.card_number_int as i64) > self.max_card_number {
            return false;
        }
        true
    }
}

/// `CardTextPredicate`: every token in the names, or every token in the rules, or every token in the types.
struct TextPredicate {
    tokens: Vec<String>,
    names: bool,
    types: bool,
    rules: bool,
}

impl TextPredicate {
    fn new(text: &str, names: bool, types: bool, rules: bool) -> TextPredicate {
        // "([^"]*)"|(\S+) over the lower case text
        let lower = text.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        let space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r');
        while i < chars.len() {
            if chars[i] == '"' {
                if let Some(end) = chars[i + 1..].iter().position(|&c| c == '"') {
                    tokens.push(chars[i + 1..i + 1 + end].iter().collect());
                    i += end + 2;
                    continue;
                }
            }
            if space(chars[i]) {
                i += 1;
                continue;
            }
            let begin = i;
            while i < chars.len() && !space(chars[i]) {
                i += 1;
            }
            tokens.push(chars[begin..i].iter().collect());
        }
        TextPredicate { tokens, names, types, rules }
    }

    /// `textHasTokens` over the lower case text, and over it with ", " as " " when a token has a space (only such
    /// a token can be found there and not in the text itself)
    fn applies(&self, cards: &Cards, r: &Row, spaced: bool) -> bool {
        if self.tokens.is_empty() {
            return true;
        }
        let has_all = |text: &str| {
            self.tokens.iter().all(|t| text.contains(t.as_str()))
                || (spaced && text.contains(", ") && {
                    let spaced = text.replace(", ", " ");
                    self.tokens.iter().all(|t| text.contains(t.as_str()) || spaced.contains(t.as_str()))
                })
        };
        (self.names && has_all(cards.str(r.search_names)))
            || (self.rules && has_all(cards.str(r.search_rules)))
            || (self.types && has_all(cards.str(r.search_types)))
    }
}

/// one piece of a LIKE pattern (H2's, `\` escapes)
#[derive(Debug, Clone, PartialEq)]
enum Like {
    Any,
    One,
    Char(char),
}

fn like_pattern(pattern: &str) -> Vec<Like> {
    let mut out = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\\' => match chars.next() {
                Some(next) => Like::Char(next),
                None => Like::Char('\\'),
            },
            '%' => {
                if out.last() == Some(&Like::Any) {
                    continue;
                }
                Like::Any
            }
            '_' => Like::One,
            c => Like::Char(c),
        });
    }
    out
}

/// LIKE ignoring case, as the card database compares (`IGNORECASE=TRUE`)
fn like_matches(pattern: &[Like], value: &str) -> bool {
    let value: Vec<char> = value.chars().collect();
    let (mut p, mut v) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while v < value.len() {
        match pattern.get(p) {
            Some(Like::Any) => {
                star = Some((p, v));
                p += 1;
            }
            Some(Like::One) => {
                p += 1;
                v += 1;
            }
            Some(Like::Char(c)) if same_ignoring_case(*c, value[v]) => {
                p += 1;
                v += 1;
            }
            _ => match star {
                Some((sp, sv)) => {
                    p = sp + 1;
                    v = sv + 1;
                    star = Some((sp, sv + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|x| *x == Like::Any)
}

/// `Character.toUpperCase`: one character to one
fn upper(c: char) -> char {
    let mut u = c.to_uppercase();
    match (u.next(), u.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

fn lower_char(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

fn same_ignoring_case(a: char, b: char) -> bool {
    a == b || upper(a) == upper(b) || lower_char(upper(a)) == lower_char(upper(b))
}

/// `String.equalsIgnoreCase`
fn equals_ignore_case(a: &str, b: &str) -> bool {
    a.chars().count() == b.chars().count() && a.chars().zip(b.chars()).all(|(x, y)| same_ignoring_case(x, y))
}

/// `String.trim`: drops characters up to U+0020 at both ends
fn java_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c <= ' ')
}

fn identity_of(letters: &str) -> u8 {
    letters.to_uppercase().chars().fold(0, |bits, c| {
        bits | match c {
            'W' => WHITE,
            'U' => BLUE,
            'B' => BLACK,
            'R' => RED,
            'G' => GREEN,
            _ => 0,
        }
    })
}

fn colors_of(bits: u8) -> HashSet<&'static str> {
    COLORS.iter().filter(|(bit, _)| bits & bit != 0).map(|(_, name)| *name).collect()
}

/// `CardSearchService.normalize`
fn normalize(colors: &[Option<String>]) -> HashSet<String> {
    colors.iter().flatten().filter(|c| !java_trim(c).is_empty()).map(|c| java_trim(c).to_lowercase()).collect()
}

fn matches_colors(card: &HashSet<&str>, requested: &HashSet<String>, color_match: Option<&str>) -> bool {
    if requested.is_empty() {
        return true;
    }
    if card.is_empty() {
        return requested.contains("colorless");
    }
    let requested: HashSet<&str> = requested.iter().map(String::as_str).filter(|c| *c != "colorless").collect();
    if requested.is_empty() {
        return false;
    }
    match color_match.unwrap_or("any") {
        "exact" => *card == requested,
        "include" => requested.iter().all(|c| card.contains(c)),
        _ => requested.iter().any(|c| card.contains(c)),
    }
}

fn matches_any_color(card: &HashSet<&str>, excluded: &HashSet<String>) -> bool {
    if excluded.is_empty() {
        return false;
    }
    if card.is_empty() {
        return excluded.contains("colorless");
    }
    excluded.iter().any(|c| card.contains(c.as_str()))
}

fn matches_mana_value(value: i32, wanted: Option<i64>, operator: Option<&str>) -> bool {
    match (wanted, operator) {
        (Some(w), Some("lte")) => (value as i64) <= w,
        (Some(w), Some("gte")) => (value as i64) >= w,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_ignores_case_and_escapes() {
        let p = like_pattern("%bolt%");
        assert!(like_matches(&p, "Lightning Bolt"));
        assert!(!like_matches(&p, "Lightning Strike"));
        assert!(like_matches(&like_pattern("%a_c%"), "xxABCxx"));
        assert!(!like_matches(&like_pattern("%a\\_c%"), "xxABCxx"));
        assert!(like_matches(&like_pattern("%a\\_c%"), "a_c"));
        assert!(like_matches(&like_pattern("%%"), ""));
        assert!(like_matches(&like_pattern("%CREATURE%"), "CREATURE@@@ARTIFACT@@@"));
        assert!(!like_matches(&like_pattern("%ab%"), "a"));
    }

    #[test]
    fn tokens_as_the_predicate_splits_them() {
        let t = TextPredicate::new(r#"Draw "a card" Fly"#, true, false, false);
        assert_eq!(t.tokens, vec!["draw", "a card", "fly"]);
        let t = TextPredicate::new(r#"x "open"#, true, false, false);
        assert_eq!(t.tokens, vec!["x", "\"open"]);
    }

    #[test]
    fn colors() {
        let card = colors_of(WHITE | BLUE);
        let want = |c: &[&str]| c.iter().map(|s| s.to_string()).collect::<HashSet<_>>();
        assert!(matches_colors(&card, &want(&["white"]), None));
        assert!(!matches_colors(&card, &want(&["white"]), Some("exact")));
        assert!(matches_colors(&card, &want(&["white", "blue"]), Some("exact")));
        assert!(!matches_colors(&colors_of(0), &want(&["white"]), None));
        assert!(matches_colors(&colors_of(0), &want(&["colorless"]), None));
        assert!(!matches_colors(&card, &want(&["colorless"]), None));
        assert_eq!(identity_of("wu"), WHITE | BLUE);
    }
}
