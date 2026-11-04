//! `deckValidate` (docs/rust/Catalog.md): a deck against a format, as `DeckValidationService` does it. The formats
//! whose validation is `Constructed`'s, unchanged, are answered here from the rules the catalog exports (banned,
//! restricted and single cards, sets, rarities); the others (Commander, Brawl, Oathbreaker, limited…) are declined
//! and Java answers them. Commander brackets are worked out for every deck, as Java does.

use serde_json::{json, Value};
use sqlx::sqlite::SqlitePool;
use sqlx::Row as _;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// `Deck.MAX_CARDS_PER_DECK`
pub const MAX_CARDS_PER_DECK: usize = 2000;
/// `CommanderBrackets.MAX_LEVEL` and `ANY_AMOUNT`
const MAX_LEVEL: usize = 5;
const ANY_AMOUNT: usize = 99;

const GAME_CHANGERS: &str = "Game Changers";
const INFINITE_COMBOS: &str = "Early-game 2-Card Combos";
const MASS_LAND_DESTRUCTION: &str = "Mass Land Destruction";
const EXTRA_TURNS: &str = "Extra Turns";
/// `CommanderBrackets.GROUP_LIMITS`, for levels 0..5
const GROUP_LIMITS: [(&str, [usize; 6]); 4] = [
    (GAME_CHANGERS, [0, 0, 0, 3, ANY_AMOUNT, ANY_AMOUNT]),
    (INFINITE_COMBOS, [0, 0, 0, 0, ANY_AMOUNT, ANY_AMOUNT]),
    (MASS_LAND_DESTRUCTION, [0, 0, 0, 0, ANY_AMOUNT, ANY_AMOUNT]),
    (EXTRA_TURNS, [0, 0, 0, 3, ANY_AMOUNT, ANY_AMOUNT]),
];

/// `DeckValidator.maxCopiesMap`: basic lands and the cards with their own limits
const MAX_COPIES: [(&str, usize); 26] = [
    ("Plains", usize::MAX),
    ("Island", usize::MAX),
    ("Swamp", usize::MAX),
    ("Mountain", usize::MAX),
    ("Forest", usize::MAX),
    ("Wastes", usize::MAX),
    ("Snow-Covered Plains", usize::MAX),
    ("Snow-Covered Island", usize::MAX),
    ("Snow-Covered Swamp", usize::MAX),
    ("Snow-Covered Mountain", usize::MAX),
    ("Snow-Covered Forest", usize::MAX),
    ("Snow-Covered Wastes", usize::MAX),
    ("Once More with Feeling", 1),
    ("Seven Dwarves", 7),
    ("Nazgul", 9),
    ("Cid, Timeless Artificer", usize::MAX),
    ("Dragon's Approach", usize::MAX),
    ("Hare Apparent", usize::MAX),
    ("Persistent Petitioners", usize::MAX),
    ("Rat Colony", usize::MAX),
    ("Relentless Rats", usize::MAX),
    ("Shadowborn Apostle", usize::MAX),
    ("Slime Against Humanity", usize::MAX),
    ("Sphinx's Approach", usize::MAX),
    ("Tempest Hawk", usize::MAX),
    ("Templar Knight", usize::MAX),
];

/// `Rarity.toString`
fn rarity_text(name: &str) -> &str {
    match name {
        "LAND" => "Land",
        "COMMON" => "Common",
        "UNCOMMON" => "Uncommon",
        "RARE" => "Rare",
        "MYTHIC" => "Mythic",
        "SPECIAL" => "Special",
        "BONUS" => "Bonus",
        other => other,
    }
}

/// `DeckValidatorErrorType`: sort order, and whether a deck with only such errors is partly valid
fn error_type(name: &str) -> (i32, bool) {
    match name {
        "PRIMARY" => (10, false),
        "DECK_SIZE" => (20, true),
        "BANNED" => (30, false),
        "WRONG_SET" => (40, false),
        _ => (50, false),
    }
}

pub struct Format {
    pub plain: bool,
    title: String,
    short_name: String,
    deck_min: usize,
    set_codes: Vec<String>,
    banned: Vec<String>,
    restricted: Vec<String>,
    single: HashSet<String>,
    rarities: Vec<String>,
}

pub struct Formats {
    formats: HashMap<String, Format>,
    game_changers: HashSet<String>,
    /// "card1@card2", the names sorted
    combos: Vec<(String, String)>,
}

/// A card as `Deck.load` makes it: what validation reads of it.
#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub set_code: String,
    pub rarity: String,
    pub extra_deck: bool,
    pub mass_land_destruction: bool,
    pub extra_turn: bool,
}

/// A deck list entry (`DeckCardInfo`) as Gson reads it.
#[derive(Debug, Clone, Default)]
pub struct DeckEntry {
    pub name: Option<String>,
    pub set_code: Option<String>,
    pub number: Option<String>,
    pub amount: i64,
}

/// The deck (`DeckCardLists`): main and sideboard entries. None for a shape Gson wouldn't read the same way.
pub fn deck_lists(value: &Value) -> Option<(Vec<DeckEntry>, Vec<DeckEntry>)> {
    let deck = value.as_object()?;
    let list = |key: &str| -> Option<Vec<DeckEntry>> {
        match deck.get(key) {
            None => Some(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    let entry = item.as_object()?;
                    let text = |key: &str| match entry.get(key) {
                        None | Some(Value::Null) => Some(None),
                        Some(Value::String(s)) => Some(Some(s.clone())),
                        Some(_) => None,
                    };
                    let amount = match entry.get("amount") {
                        None | Some(Value::Null) => 0,
                        Some(v) => {
                            let n = v.as_i64()?;
                            i32::try_from(n).ok()?;
                            n
                        }
                    };
                    Some(DeckEntry {
                        name: text("cardName")?,
                        set_code: text("setCode")?,
                        number: text("cardNumber")?,
                        amount,
                    })
                })
                .collect(),
            Some(_) => None,
        }
    };
    Some((list("cards")?, list("sideboard")?))
}

impl Formats {
    pub async fn load(pool: &SqlitePool) -> anyhow::Result<Formats> {
        let mut formats = HashMap::new();
        for r in sqlx::query("SELECT name, plain, title, short_name, deck_min FROM formats").fetch_all(pool).await? {
            let name: String = r.get(0);
            formats.insert(
                name,
                Format {
                    plain: r.get::<i64, _>(1) != 0,
                    title: r.get::<Option<String>, _>(2).unwrap_or_default(),
                    short_name: r.get::<Option<String>, _>(3).unwrap_or_default(),
                    deck_min: r.get::<Option<i64>, _>(4).unwrap_or(0).max(0) as usize,
                    set_codes: Vec::new(),
                    banned: Vec::new(),
                    restricted: Vec::new(),
                    single: HashSet::new(),
                    rarities: Vec::new(),
                },
            );
        }
        for r in
            sqlx::query("SELECT format, set_code FROM format_sets ORDER BY format, position").fetch_all(pool).await?
        {
            if let Some(f) = formats.get_mut(&r.get::<String, _>(0)) {
                f.set_codes.push(r.get(1));
            }
        }
        for r in sqlx::query("SELECT format, list, name FROM format_cards ORDER BY format, list, position")
            .fetch_all(pool)
            .await?
        {
            let Some(f) = formats.get_mut(&r.get::<String, _>(0)) else { continue };
            let name: String = r.get(2);
            match r.get::<String, _>(1).as_str() {
                "banned" => f.banned.push(name),
                "restricted" => f.restricted.push(name),
                "single" => {
                    f.single.insert(name);
                }
                "rarity" => f.rarities.push(name),
                _ => {}
            }
        }
        let mut game_changers = HashSet::new();
        let mut combos = Vec::new();
        for r in sqlx::query("SELECT list, value FROM brackets").fetch_all(pool).await? {
            let value: String = r.get(1);
            match r.get::<String, _>(0).as_str() {
                "gameChangers" => {
                    game_changers.insert(value);
                }
                "combos" => {
                    if let Some((a, b)) = value.split_once('@') {
                        combos.push((a.to_string(), b.to_string()));
                    }
                }
                _ => {}
            }
        }
        Ok(Formats { formats, game_changers, combos })
    }

    /// the cards whose other printings the checks look at: a rarity or set the format doesn't allow
    pub fn needs_printings(format: &Format, card: &Card) -> bool {
        (!format.rarities.is_empty() && !format.rarities.contains(&card.rarity))
            || (!format.set_codes.is_empty() && !format.set_codes.contains(&card.set_code))
    }

    pub fn get(&self, name: &str) -> Option<&Format> {
        self.formats.get(name)
    }

    /// `DeckValidationResult` for a plain format. `printings_of(name)`: the (set code, rarity) of every printing
    /// `CardRepository.findCards(name)` finds.
    pub fn validate(
        &self,
        deck_type: &str,
        format: &Format,
        cards: &[Card],
        sideboard: &[Card],
        printings_of: &HashMap<String, Vec<(String, String)>>,
    ) -> Value {
        let mut errors: Vec<Error> = Vec::new();
        let mut valid = true;
        let maindeck = cards.iter().filter(|c| !c.extra_deck).count();
        if maindeck < format.deck_min {
            errors.push(Error::new(
                "DECK_SIZE",
                "Deck",
                format!("Must contain at least {} cards: has only {} cards", format.deck_min, maindeck),
                false,
            ));
            valid = false;
        }
        if sideboard.len() > 15 {
            errors.push(Error::new(
                "DECK_SIZE",
                "Sideboard",
                format!("Must contain no more than 15 cards : has {} cards", sideboard.len()),
                false,
            ));
            valid = false;
        }
        let mut counts: Vec<(&str, usize)> = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for card in cards.iter().chain(sideboard) {
            match index.get(card.name.as_str()) {
                Some(&i) => counts[i].1 += 1,
                None => {
                    index.insert(&card.name, counts.len());
                    counts.push((&card.name, 1));
                }
            }
        }
        let count = |name: &str| index.get(name).map(|&i| counts[i].1);
        for (name, n) in &counts {
            let max = MAX_COPIES.iter().find(|(card, _)| card == name).map(|(_, m)| *m).unwrap_or(4);
            if *n > max {
                errors.push(Error::new("OTHER", name, format!("Too many: {n}"), true));
                valid = false;
            }
        }
        for banned in &format.banned {
            if count(banned).is_some() {
                errors.push(Error::new("BANNED", banned, "Banned".into(), true));
                valid = false;
            }
        }
        for restricted in &format.restricted {
            if let Some(n) = count(restricted).filter(|n| *n > 1) {
                errors.push(Error::new("OTHER", restricted, format!("Restricted amount: {n}"), true));
                valid = false;
            }
        }
        let none = Vec::new();
        let has_group = |errors: &[Error], group: &str| errors.iter().any(|e| e.group == group);
        if !format.rarities.is_empty() {
            for card in cards.iter().chain(sideboard) {
                if format.rarities.contains(&card.rarity) {
                    continue;
                }
                let legal =
                    printings_of.get(&card.name).unwrap_or(&none).iter().any(|(_, r)| format.rarities.contains(r));
                if !legal {
                    if !has_group(&errors, &card.name) {
                        errors.push(Error::new(
                            "OTHER",
                            &card.name,
                            format!("Invalid rarity: {}", rarity_text(&card.rarity)),
                            true,
                        ));
                    }
                    valid = false;
                }
            }
        }
        let set_allowed = |code: &str| format.set_codes.is_empty() || format.set_codes.iter().any(|c| c == code);
        for card in cards.iter().chain(sideboard) {
            if set_allowed(&card.set_code) {
                continue;
            }
            let legal = printings_of.get(&card.name).unwrap_or(&none).iter().any(|(set, _)| set_allowed(set))
                || format.single.contains(&card.name);
            if !legal {
                if !has_group(&errors, &card.name) {
                    errors.push(Error::new("WRONG_SET", &card.name, format!("Invalid set: {}", card.set_code), true));
                }
                valid = false;
            }
        }

        let partly_valid = errors.iter().all(|e| error_type(&e.kind).1);
        // a stable sort, as List.sort: by type, then group, then message
        errors.sort_by(|a, b| {
            error_type(&a.kind)
                .0
                .cmp(&error_type(&b.kind).0)
                .then_with(|| java_cmp(&a.group, &b.group))
                .then_with(|| java_cmp(&a.message, &b.message))
        });
        json!({
            "deckType": deck_type,
            "name": format.title,
            "shortName": format.short_name,
            "valid": valid,
            "partlyValid": partly_valid,
            "errors": errors.iter().map(Error::to_json).collect::<Vec<_>>(),
            "edhPowerLevel": 0,
            "edhPowerCards": [],
            "edhPowerDetails": [],
            "commanderBrackets": self.brackets(cards, sideboard),
        })
    }

    /// `CommanderBrackets.analyze`, then a `BracketInfo` per level 1..5
    fn brackets(&self, cards: &[Card], sideboard: &[Card]) -> Value {
        let all: Vec<&Card> = cards.iter().chain(sideboard).collect();
        let sorted = |mut names: Vec<String>| {
            names.sort_by(|a, b| java_cmp(a, b));
            names
        };
        let names: HashSet<&str> = all.iter().map(|c| c.name.as_str()).collect();
        let mut in_combo: HashSet<&str> = HashSet::new();
        for (a, b) in &self.combos {
            if names.contains(a.as_str()) && names.contains(b.as_str()) && a != b {
                in_combo.insert(a);
                in_combo.insert(b);
            }
        }
        let mut seen = HashSet::new();
        let combo_cards: Vec<String> = all
            .iter()
            .filter(|c| in_combo.contains(c.name.as_str()) && seen.insert(c.name.as_str()))
            .map(|c| c.name.clone())
            .collect();
        let groups: Vec<(&str, Vec<String>)> = vec![
            (
                GAME_CHANGERS,
                sorted(all.iter().filter(|c| self.game_changers.contains(&c.name)).map(|c| c.name.clone()).collect()),
            ),
            (INFINITE_COMBOS, sorted(combo_cards)),
            (
                MASS_LAND_DESTRUCTION,
                sorted(all.iter().filter(|c| c.mass_land_destruction).map(|c| c.name.clone()).collect()),
            ),
            (EXTRA_TURNS, sorted(all.iter().filter(|c| c.extra_turn).map(|c| c.name.clone()).collect())),
        ];
        let max = |group: &str, level: usize| {
            GROUP_LIMITS.iter().find(|(g, _)| *g == group).map(|(_, limits)| limits[level.min(MAX_LEVEL)]).unwrap_or(0)
        };
        (1..=MAX_LEVEL)
            .map(|level| {
                let mut bad: Vec<String> = Vec::new();
                for (group, cards) in &groups {
                    if cards.len() > max(group, level) {
                        bad.extend(cards.iter().cloned());
                    }
                }
                let mut bad = sorted(bad);
                bad.dedup();
                json!({
                    "level": level,
                    "name": format!("Bracket {level}"),
                    "shortName": format!("B{level}"),
                    "valid": bad.is_empty(),
                    "badCards": bad,
                    "groups": groups.iter().map(|(group, cards)| json!({
                        "name": group,
                        "count": cards.len(),
                        "max": max(group, level),
                        "cards": cards,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect()
    }
}

struct Error {
    kind: String,
    group: String,
    message: String,
    card_name: Option<String>,
}

impl Error {
    fn new(kind: &str, group: &str, message: String, card_error: bool) -> Error {
        Error { kind: kind.into(), group: group.into(), message, card_name: card_error.then(|| group.to_string()) }
    }

    fn to_json(&self) -> Value {
        let mut error = json!({"type": self.kind, "group": self.group, "message": self.message});
        if let Some(card) = &self.card_name {
            error["cardName"] = Value::String(card.clone());
        }
        error
    }
}

/// `String.compareTo`: by UTF-16 code unit
pub fn java_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str, set: &str, rarity: &str) -> Card {
        Card {
            name: name.into(),
            set_code: set.into(),
            rarity: rarity.into(),
            extra_deck: false,
            mass_land_destruction: false,
            extra_turn: false,
        }
    }

    fn pauper() -> Format {
        Format {
            plain: true,
            title: "Constructed - Pauper".into(),
            short_name: "Pauper".into(),
            deck_min: 60,
            set_codes: vec![],
            banned: vec!["Gush".into()],
            restricted: vec![],
            single: HashSet::new(),
            rarities: vec!["COMMON".into()],
        }
    }

    #[test]
    fn counts_rarities_and_sorts_errors() {
        let formats = Formats { formats: HashMap::new(), game_changers: HashSet::new(), combos: vec![] };
        let mut cards: Vec<Card> = (0..5).map(|_| card("Lightning Bolt", "M10", "COMMON")).collect();
        cards.push(card("Gush", "MMQ", "COMMON"));
        cards.push(card("Baneslayer Angel", "M10", "MYTHIC"));
        let printings =
            HashMap::from([("Baneslayer Angel".to_string(), vec![("M10".to_string(), "MYTHIC".to_string())])]);
        let result = formats.validate("Constructed - Pauper", &pauper(), &cards, &[], &printings);
        assert_eq!(result["valid"], false);
        assert_eq!(result["partlyValid"], false);
        let messages: Vec<&str> =
            result["errors"].as_array().unwrap().iter().map(|e| e["message"].as_str().unwrap()).collect();
        assert_eq!(
            messages,
            vec!["Must contain at least 60 cards: has only 7 cards", "Banned", "Invalid rarity: Mythic", "Too many: 5"]
        );
        assert!(result["errors"][0].get("cardName").is_none());
        assert_eq!(result["errors"][1]["cardName"], "Gush");
        assert_eq!(result["commanderBrackets"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn utf16_order() {
        assert_eq!(java_cmp("Ab", "a"), Ordering::Less);
        assert_eq!(java_cmp("\u{FF5E}", "\u{1F600}"), Ordering::Greater);
    }
}
