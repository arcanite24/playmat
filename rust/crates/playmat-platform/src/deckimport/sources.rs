//! The deck websites: Archidekt, TCGplayer, MTGTop8, Scryfall and ManaBox, read as Java's `*Source` classes do.

use super::fetch::Fetcher;
use super::html::{self, Page};
use super::json;
use super::{java_trim, Failure, ImportedDeck, Link, Reason, Section};
use regex::Regex;
use std::sync::LazyLock;

/// Java's `\s`
const WS: &str = r"[ \t\n\x0B\x0C\r]";
/// Java's `.`: anything but a line terminator
const ANY: &str = r"[^\n\r\x{85}\x{2028}\x{2029}]";

static SLASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("{WS}*/{{1,3}}{WS}*")).unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("{WS}+")).unwrap());
static ARCHIDEKT_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^/(?:api/)?decks/([0-9]+)(?:/{ANY}*)?$")).unwrap());
static TCGPLAYER_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^/(?:content/)?magic-the-gathering/deck/[^/]+/([0-9]+)/?$").unwrap());
static MTGTOP8_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new("^/(event|dec|mtgo)$").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[0-9]+$").unwrap());
static LETTERS: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[A-Za-z]+$").unwrap());
static CARD_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(SB:{WS}*)?([0-9]+){WS}+\[([A-Za-z0-9]*)\]{WS}+({ANY}+)$")).unwrap());
static TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({ANY}+?){WS}+-{WS}+({ANY}+?){WS}+@{WS}+mtgtop8\.com$")).unwrap());
static SCRYFALL_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "^/(?:@[^/]+/)?decks/([0-9a-fA-F]{{8}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{12}})(?:/{ANY}*)?$"
    ))
    .unwrap()
});
static SCRYFALL_AUTHOR: LazyLock<Regex> = LazyLock::new(|| Regex::new("/@([^/]+)/decks/").unwrap());
static MANABOX_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new("^/decks/([A-Za-z0-9_-]{22})/?$").unwrap());

/// "Fire/Ice", "Wear / Tear" → "Fire // Ice" (`SourceSupport.cardName`)
pub fn card_name(name: Option<String>) -> Option<String> {
    let name = name?;
    let slashed = SLASHES.replace_all(&name, " // ");
    Some(java_trim(&SPACES.replace_all(&slashed, " ")).to_string())
}

/// A site's format name ("modern", "Commander") as an XMage deck type (`SourceSupport.deckType`)
pub fn deck_type(site_format: Option<&str>) -> Option<&'static str> {
    let key = java_trim(site_format?).to_lowercase().replace(['_', '-'], " ");
    Some(match key.as_str() {
        "standard" => "Constructed - Standard",
        "pioneer" => "Constructed - Pioneer",
        "modern" => "Constructed - Modern",
        "legacy" => "Constructed - Legacy",
        "vintage" => "Constructed - Vintage",
        "pauper" => "Constructed - Pauper",
        "historic" => "Constructed - Historic",
        "premodern" => "Constructed - Premodern",
        "frontier" => "Constructed - Frontier",
        "canadian highlander" | "canlander" => "Constructed - Canadian Highlander",
        "commander" | "edh" | "cedh" => "Variant Magic - Commander",
        "duel commander" | "duel" => "Variant Magic - Duel Commander",
        "1v1 commander" => "Variant Magic - MTGO 1v1 Commander",
        "brawl" | "historic brawl" | "historicbrawl" => "Variant Magic - Brawl",
        "oathbreaker" => "Variant Magic - Oathbreaker",
        "penny dreadful commander" => "Variant Magic - Penny Dreadful Commander",
        "tiny leaders" => "Variant Magic - Tiny Leaders",
        _ => return None,
    })
}

fn capture(regex: &Regex, text: &str) -> Option<String> {
    regex.captures(text).map(|c| c[1].to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Archidekt,
    Tcgplayer,
    MtgTop8,
    Scryfall,
    Manabox,
}

impl Source {
    /// `DeckImportService.defaultSources()`, in its order
    pub const ALL: [Source; 5] =
        [Source::Archidekt, Source::Tcgplayer, Source::MtgTop8, Source::Scryfall, Source::Manabox];

    /// the site id shared with the web client (core/deckImport/sites.ts)
    pub fn id(self) -> &'static str {
        match self {
            Source::Archidekt => "archidekt",
            Source::Tcgplayer => "tcgplayer",
            Source::MtgTop8 => "mtgtop8",
            Source::Scryfall => "scryfall",
            Source::Manabox => "manabox",
        }
    }

    /// hosts of the site's deck pages, as players paste them
    pub fn page_hosts(self) -> &'static [&'static str] {
        match self {
            Source::Archidekt => &["archidekt.com", "www.archidekt.com"],
            Source::Tcgplayer => &["infinite.tcgplayer.com", "www.tcgplayer.com", "tcgplayer.com"],
            Source::MtgTop8 => &["mtgtop8.com", "www.mtgtop8.com"],
            Source::Scryfall => &["scryfall.com", "www.scryfall.com", "api.scryfall.com"],
            Source::Manabox => &["manabox.app", "www.manabox.app"],
        }
    }

    /// every host this source fetches from; together they form the allowlist
    pub fn fetch_hosts(self) -> &'static [&'static str] {
        match self {
            Source::Archidekt => &["archidekt.com", "www.archidekt.com"],
            Source::Tcgplayer => &["infinite-api.tcgplayer.com"],
            Source::MtgTop8 => &["mtgtop8.com", "www.mtgtop8.com"],
            Source::Scryfall => &["api.scryfall.com"],
            Source::Manabox => &["cloud.manabox.app"],
        }
    }

    pub fn for_host(host: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|source| source.page_hosts().contains(&host))
    }

    /// the deck id in a deck page link, or None when the link isn't a deck page
    pub fn deck_id(self, link: &Link) -> Option<String> {
        match self {
            Source::Archidekt => capture(&ARCHIDEKT_PATH, &link.path),
            Source::Tcgplayer => capture(&TCGPLAYER_PATH, &link.path),
            Source::MtgTop8 => {
                if !MTGTOP8_PATH.is_match(&link.path) {
                    return None;
                }
                query(link, "d").filter(|d| NUMBER.is_match(d))
            }
            Source::Scryfall => capture(&SCRYFALL_PATH, &link.path).map(|id| id.to_lowercase()),
            Source::Manabox => capture(&MANABOX_PATH, &link.path),
        }
    }

    pub async fn fetch(self, link: &Link, deck_id: &str, fetcher: &dyn Fetcher) -> Result<ImportedDeck, Failure> {
        const JSON: &str = "application/json";
        match self {
            Source::Archidekt => {
                archidekt(&fetcher.get(&format!("https://archidekt.com/api/decks/{deck_id}/"), JSON).await?, deck_id)
            }
            Source::Tcgplayer => {
                let url = format!(
                    "https://infinite-api.tcgplayer.com/deck/magic/{deck_id}/?source=infinite-content&subDecks=true&cards=true&stats=false"
                );
                tcgplayer(&fetcher.get(&url, JSON).await?, deck_id)
            }
            Source::MtgTop8 => {
                let list = fetcher.get(&format!("https://mtgtop8.com/dec?d={deck_id}"), "text/plain").await?;
                let event = query(link, "e");
                let format = query(link, "f");
                let mut page = None;
                if let Some(event) = event.as_deref().filter(|e| NUMBER.is_match(e)) {
                    let format_part = match format.as_deref() {
                        Some(f) if LETTERS.is_match(f) => format!("&f={f}"),
                        _ => String::new(),
                    };
                    let url = format!("https://mtgtop8.com/event?e={event}&d={deck_id}{format_part}");
                    page = match fetcher.get(&url, "text/html").await {
                        Ok(page) => Some(page),
                        // the list is what matters; the page only names it
                        Err(Failure::Import(..)) => None,
                        Err(Failure::Shape) => return Err(Failure::Shape),
                    };
                }
                mtgtop8(&list, page.as_deref(), deck_id, event.as_deref(), format.as_deref())
            }
            Source::Scryfall => {
                let url = format!("https://api.scryfall.com/decks/{deck_id}/export/json");
                scryfall(&fetcher.get(&url, JSON).await?, deck_id)
            }
            Source::Manabox => {
                manabox(&fetcher.get(&format!("https://cloud.manabox.app/decks/{deck_id}"), JSON).await?, deck_id)
            }
        }
    }
}

/// `MtgTop8Source.query`: the raw value of the first `key=` pair
fn query(link: &Link, key: &str) -> Option<String> {
    let raw = link.raw_query.as_deref()?;
    raw.split('&').find_map(|pair| match pair.find('=') {
        Some(eq) if eq > 0 && &pair[..eq] == key => Some(pair[eq + 1..].to_string()),
        _ => None,
    })
}

fn no_cards(deck: &ImportedDeck, message: String) -> Result<(), Failure> {
    if deck.card_count() == 0 {
        return Err(Failure::new(Reason::NotFound, message));
    }
    Ok(())
}

fn java_equals_ignore_case(a: &str, b: &str) -> bool {
    a.chars().count() == b.chars().count()
        && a.chars()
            .zip(b.chars())
            .all(|(x, y)| x == y || x.to_uppercase().eq(y.to_uppercase()) || x.to_lowercase().eq(y.to_lowercase()))
}

fn archidekt_format(number: i32) -> Option<&'static str> {
    Some(match number {
        1 => "Constructed - Standard",
        2 => "Constructed - Modern",
        3 => "Variant Magic - Commander",
        4 => "Constructed - Legacy",
        5 => "Constructed - Vintage",
        6 => "Constructed - Pauper",
        8 => "Constructed - Frontier",
        11 => "Variant Magic - MTGO 1v1 Commander",
        12 => "Variant Magic - Duel Commander",
        13 | 20 => "Variant Magic - Brawl",
        14 => "Variant Magic - Oathbreaker",
        15 => "Constructed - Pioneer",
        16 => "Constructed - Historic",
        22 => "Constructed - Premodern",
        25 => "Constructed - Canadian Highlander",
        _ => return None,
    })
}

/// Archidekt's public deck API: each card lists its categories, the first is its main one, and categories can be
/// marked as not part of the deck (the maybeboard)
pub fn archidekt(body: &str, deck_id: &str) -> Result<ImportedDeck, Failure> {
    let root = json::parse_object(body, "Archidekt")?;
    let json = Some(&root);
    if root.contains_key("error") && !root.contains_key("cards") {
        return Err(Failure::new(Reason::NotFound, format!("Archidekt has no deck {deck_id}")));
    }
    if !root.contains_key("cards") {
        return Err(Failure::new(Reason::SiteChanged, "Archidekt's deck has no card list"));
    }
    let mut deck = ImportedDeck::new("archidekt", deck_id, format!("https://archidekt.com/decks/{deck_id}"));
    deck.set_name(json::string(json, "name").as_deref());
    deck.set_author(json::string(json::object(json, "owner"), "username").as_deref());
    deck.remote_updated_at = json::iso_millis(json::string(json, "updatedAt").as_deref());
    deck.format = archidekt_format(json::integer(json, "deckFormat", -1)).map(str::to_string);

    let mut outside: Vec<Option<String>> = Vec::new();
    for element in json::array(json, "categories") {
        if let Some(category) = element.as_object() {
            if category.contains_key("includedInDeck") && !json::boolean(Some(category), "includedInDeck") {
                outside.push(json::string(Some(category), "name"));
            }
        }
    }

    let featured = json::string(json, "featured");
    for element in json::array(json, "cards") {
        let Some(entry) = element.as_object() else { continue };
        let entry = Some(entry);
        let card = json::object(entry, "card");
        let oracle = json::object(card, "oracleCard");
        let name = card_name(json::string(oracle, "name"));
        let set_code = json::string(json::object(card, "edition"), "editioncode");
        let number = json::string(card, "collectorNumber");
        let amount = json::integer(entry, "quantity", 0);

        let categories = json::array(entry, "categories");
        let primary = categories.first().and_then(json::primitive_string).unwrap_or_default();
        let section = if outside.contains(&Some(primary.clone())) || java_equals_ignore_case("Maybeboard", &primary) {
            Section::Maybe
        } else if java_equals_ignore_case("Commander", &primary) {
            Section::Commander
        } else if json::boolean(entry, "companion") {
            Section::Companion
        } else if java_equals_ignore_case("Sideboard", &primary) {
            Section::Side
        } else {
            Section::Main
        };
        deck.add(section, name.as_deref(), set_code.as_deref(), number.as_deref(), amount)?;

        let uid = json::string(card, "uid");
        if let (None, Some(featured), Some(uid)) = (&deck.cover, &featured, &uid) {
            if featured.contains(uid.as_str()) {
                deck.set_cover(name.as_deref(), set_code.as_deref(), number.as_deref());
            }
        }
    }
    no_cards(&deck, format!("Archidekt's deck {deck_id} has no cards"))?;
    Ok(deck)
}

/// TCGplayer Infinite's deck API: sub-decks maindeck, sideboard and commandzone; set codes but no collector numbers
pub fn tcgplayer(body: &str, deck_id: &str) -> Result<ImportedDeck, Failure> {
    let root = json::parse_object(body, "TCGplayer")?;
    let json = Some(&root);
    let Some(result) = json::object(json, "result") else {
        if root.contains_key("error") {
            return Err(Failure::new(Reason::NotFound, format!("TCGplayer has no deck {deck_id}")));
        }
        return Err(Failure::new(Reason::SiteChanged, "TCGplayer's deck has no result"));
    };
    let result = Some(result);
    let info = json::object(result, "deck");
    let sub_decks = json::object(info, "subDecks");
    let cards = json::object(result, "cards");
    let (Some(sub_decks), Some(_)) = (sub_decks, cards) else {
        return Err(Failure::new(Reason::SiteChanged, "TCGplayer's deck has no card list"));
    };
    let url = match json::string(result, "canonicalURL") {
        Some(canonical) => format!("https://infinite.tcgplayer.com{canonical}"),
        None => format!("https://infinite.tcgplayer.com/magic-the-gathering/deck/deck/{deck_id}"),
    };
    let mut deck = ImportedDeck::new("tcgplayer", deck_id, url);
    deck.set_name(json::string(info, "name").as_deref());
    deck.set_author(json::string(info, "playerName").as_deref());
    deck.format = deck_type(json::string(info, "format").as_deref()).map(str::to_string);
    deck.remote_updated_at = json::iso_millis(json::string(info, "updated").as_deref());
    if let Some(event) = json::string(info, "eventName") {
        let description = match json::string(info, "eventRank") {
            Some(rank) => format!("{rank} at {event}"),
            None => event,
        };
        deck.set_description(Some(&description));
    }

    for (group, value) in sub_decks {
        let section = match group.as_str() {
            "commandzone" => Section::Commander,
            "sideboard" => Section::Side,
            "maindeck" => Section::Main,
            _ => Section::Maybe,
        };
        let Some(entries) = value.as_array() else { continue };
        for element in entries {
            let Some(entry) = element.as_object() else { continue };
            let entry = Some(entry);
            let Some(card_id) = json::string(entry, "cardID") else { continue };
            let Some(card) = json::object(cards, &card_id) else { continue };
            let name = card_name(json::string(Some(card), "name"));
            let set = json::string(Some(card), "set");
            deck.add(section, name.as_deref(), set.as_deref(), None, json::integer(entry, "quantity", 0))?;
        }
    }
    no_cards(&deck, format!("TCGplayer's deck {deck_id} has no cards"))?;
    Ok(deck)
}

fn mtgtop8_format(code: &str) -> Option<&'static str> {
    Some(match code {
        "ST" => "Constructed - Standard",
        "PI" => "Constructed - Pioneer",
        "MO" => "Constructed - Modern",
        "LE" => "Constructed - Legacy",
        "VI" => "Constructed - Vintage",
        "PAU" => "Constructed - Pauper",
        "PREM" => "Constructed - Premodern",
        "HI" => "Constructed - Historic",
        "EDH" => "Variant Magic - Duel Commander",
        "CEDH" => "Variant Magic - Commander",
        _ => return None,
    })
}

/// MTGTop8's .dec export, named from the deck's event page title ("5c Domain Aggro - Ryuumei @ mtgtop8.com")
pub fn mtgtop8(
    list: &str,
    page: Option<&str>,
    deck_id: &str,
    event: Option<&str>,
    format: Option<&str>,
) -> Result<ImportedDeck, Failure> {
    let url = format!(
        "https://mtgtop8.com/event?{}d={deck_id}{}",
        event.map(|e| format!("e={e}&")).unwrap_or_default(),
        format.map(|f| format!("&f={f}")).unwrap_or_default()
    );
    let mut deck = ImportedDeck::new("mtgtop8", deck_id, url);
    if java_trim(list).starts_with('<') {
        return Err(Failure::new(Reason::SiteChanged, "MTGTop8 answered with a page instead of a deck list"));
    }
    for raw in list.split('\n') {
        let line = java_trim(raw.strip_suffix('\r').unwrap_or(raw));
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let Some(card) = CARD_LINE.captures(line) else { continue };
        let section = if card.get(1).is_some() { Section::Side } else { Section::Main };
        // Integer.parseInt throws past an int
        let amount: i32 = card[2].parse().map_err(|_| Failure::Shape)?;
        deck.add(section, card_name(Some(card[4].to_string())).as_deref(), Some(&card[3]), None, amount)?;
    }
    no_cards(&deck, format!("MTGTop8 has no deck {deck_id}"))?;

    let mut deck_type = format.and_then(|f| mtgtop8_format(&f.to_uppercase()));
    if let Some(page) = page {
        let doc = Page::parse(page);
        let title = doc.title();
        if let Some(title) = TITLE.captures(java_trim(&title)) {
            deck.set_name(Some(&title[1]));
            deck.set_author(Some(&title[2]));
        }
        if let Some(event_title) = doc.first_div("event_title") {
            deck.set_description(Some(&html::text(&event_title)));
        }
        if let (None, Some(meta)) = (deck_type, doc.first_div("meta_arch")) {
            deck_type = self::deck_type(Some(java_trim(&html::own_text(&meta))));
        }
    }
    deck.format = deck_type.map(str::to_string);
    if deck.name.is_none() {
        deck.set_name(Some(&format!("MTGTop8 deck {deck_id}")));
    }
    Ok(deck)
}

/// Scryfall's JSON export: entries grouped by section
pub fn scryfall(body: &str, deck_id: &str) -> Result<ImportedDeck, Failure> {
    let root = json::parse_object(body, "Scryfall")?;
    let json = Some(&root);
    let object = json::string(json, "object");
    if object.as_deref() == Some("error") {
        return Err(Failure::new(Reason::NotFound, format!("Scryfall has no deck {deck_id}")));
    }
    let entries = json::object(json, "entries");
    let (Some("deck"), Some(entries)) = (object.as_deref(), entries) else {
        return Err(Failure::new(Reason::SiteChanged, "Scryfall's deck export has no entries"));
    };
    let page = json::string(json, "scryfall_uri");
    let url = page.clone().unwrap_or_else(|| format!("https://scryfall.com/decks/{deck_id}"));
    let mut deck = ImportedDeck::new("scryfall", deck_id, url);
    deck.set_name(json::string(json, "name").as_deref());
    deck.format = deck_type(json::string(json, "format").as_deref()).map(str::to_string);
    deck.set_description(json::string(json, "description").as_deref());
    if let Some(author) = page.as_deref().and_then(|p| capture(&SCRYFALL_AUTHOR, p)) {
        deck.set_author(Some(&author));
    }

    for (group, value) in entries {
        let section = match group.as_str() {
            "commanders" => Section::Commander,
            "sideboard" | "outside" => Section::Side,
            "maybeboard" => Section::Maybe,
            _ => Section::Main,
        };
        let Some(list) = value.as_array() else { continue };
        for element in list {
            let Some(entry) = element.as_object() else { continue };
            let entry = Some(entry);
            // a line Scryfall couldn't match to a card (an empty row, or a typo)
            let Some(card) = json::object(entry, "card_digest") else { continue };
            let card = Some(card);
            let name = card_name(json::string(card, "name"));
            let set_code = json::string(card, "set");
            let number = json::string(card, "collector_number");
            deck.add(
                section,
                name.as_deref(),
                set_code.as_deref(),
                number.as_deref(),
                json::integer(entry, "count", 0),
            )?;
            if section == Section::Commander && deck.cover.is_none() {
                deck.set_cover(name.as_deref(), set_code.as_deref(), number.as_deref());
            }
        }
    }
    no_cards(&deck, format!("Scryfall's deck {deck_id} has no cards"))?;
    Ok(deck)
}

/// ManaBox's cloud API: board 0 is the deck, 1 the sideboard, 2 the commander
pub fn manabox(body: &str, deck_id: &str) -> Result<ImportedDeck, Failure> {
    let root = json::parse_object(body, "ManaBox")?;
    let json = Some(&root);
    if !root.contains_key("cards") {
        return Err(Failure::new(Reason::SiteChanged, "ManaBox's deck has no card list"));
    }
    let mut deck = ImportedDeck::new("manabox", deck_id, format!("https://manabox.app/decks/{deck_id}"));
    deck.set_name(json::string(json, "name").as_deref());
    if json::integer(json, "format", -1) == 4 {
        deck.format = Some("Variant Magic - Commander".into());
    }
    let edited = json::long_value(json, "editDateUTC", -1);
    if edited > 0 {
        deck.remote_updated_at = Some(edited);
    }
    let cover_id = json::integer(json, "coverImageCvId", -1);

    for element in json::array(json, "cards") {
        let Some(card) = element.as_object() else { continue };
        let card = Some(card);
        let section = match json::integer(card, "boardCategory", 0) {
            0 => Section::Main,
            1 => Section::Side,
            2 => Section::Commander,
            _ => Section::Maybe,
        };
        let name = card_name(json::string(card, "name"));
        let set_code = json::string(card, "setId");
        let number = json::string(card, "collectorNumber");
        deck.add(section, name.as_deref(), set_code.as_deref(), number.as_deref(), json::integer(card, "quantity", 0))?;
        if cover_id >= 0 && json::integer(card, "cvId", -2) == cover_id && deck.cover.is_none() {
            deck.set_cover(name.as_deref(), set_code.as_deref(), number.as_deref());
        }
    }
    no_cards(&deck, format!("ManaBox's deck {deck_id} has no cards"))?;
    Ok(deck)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// the recordings Java's DeckImportSourcesTest reads
    pub fn fixture(path: &str) -> String {
        let file = format!("{}/../../../Mage.Server/src/test/resources/deckimport/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
        String::from_utf8(bytes).unwrap()
    }

    fn latin1(path: &str) -> String {
        let file = format!("{}/../../../Mage.Server/src/test/resources/deckimport/{path}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&file).unwrap().into_iter().map(char::from).collect()
    }

    fn count(cards: &[super::super::DeckCard]) -> i32 {
        cards.iter().map(|c| c.amount).sum()
    }

    fn names(cards: &[super::super::DeckCard]) -> Vec<String> {
        cards.iter().filter_map(|c| c.name.clone()).collect()
    }

    fn reason(result: Result<ImportedDeck, Failure>) -> Reason {
        match result {
            Err(Failure::Import(reason, _)) => reason,
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    fn link(url: &str) -> Link {
        match super::super::parse(url) {
            super::super::Parsed::Link(link) => link,
            _ => panic!("{url}"),
        }
    }

    #[test]
    fn archidekt_decks() {
        let deck = archidekt(&fixture("archidekt/commander-7031486.json"), "7031486").unwrap();
        assert_eq!(deck.name.as_deref(), Some("Buffs by Hans"));
        assert_eq!(deck.author.as_deref(), Some("SalubriousSnail"));
        assert_eq!(deck.format.as_deref(), Some("Variant Magic - Commander"));
        assert_eq!((count(&deck.commanders), count(&deck.main)), (1, 99));
        assert!(deck.side.is_empty() && deck.remote_updated_at.is_some() && deck.cover.is_some());
        assert_eq!((deck.main[0].set_code.as_deref(), deck.main[0].number.as_deref()), (Some("BRO"), Some("182")));

        let deck = archidekt(&fixture("archidekt/modern-sideboard-maybeboard-3872725.json"), "3872725").unwrap();
        assert_eq!(deck.format.as_deref(), Some("Constructed - Modern"));
        assert_eq!((count(&deck.main), count(&deck.side)), (58, 17));
        assert!(names(&deck.main).contains(&"Brazen Borrower // Petty Theft".to_string()));

        let deck = archidekt(&fixture("archidekt/companion-13235048.json"), "13235048").unwrap();
        assert_eq!(names(&deck.companion), vec!["Lurrus of the Dream-Den"]);
        assert!(deck.format.is_none());

        assert_eq!(reason(archidekt(&fixture("archidekt/not-found.json"), "999999999")), Reason::NotFound);
        assert_eq!(reason(archidekt("<html>maintenance</html>", "1")), Reason::SiteChanged);

        let id = |url: &str| Source::Archidekt.deck_id(&link(url));
        assert_eq!(id("https://archidekt.com/decks/7031486/buffs_by_hans").as_deref(), Some("7031486"));
        assert_eq!(id("https://archidekt.com/api/decks/7031486/").as_deref(), Some("7031486"));
        assert_eq!(id("https://archidekt.com/search/decks"), None);
    }

    #[test]
    fn mtgtop8_decks() {
        let list = latin1("mtgtop8/896184.dec");
        let page = latin1("mtgtop8/event-91675-896184.html");
        let deck = mtgtop8(&list, Some(&page), "896184", Some("91675"), Some("MO")).unwrap();
        assert_eq!(deck.name.as_deref(), Some("5c Domain Aggro"));
        assert_eq!(deck.author.as_deref(), Some("Ryuumei"));
        assert_eq!(deck.format.as_deref(), Some("Constructed - Modern"));
        assert_eq!((count(&deck.main), count(&deck.side)), (60, 15));
        assert!(names(&deck.side).contains(&"Wear // Tear".to_string()));
        let bolt = deck.main.iter().find(|c| c.name.as_deref() == Some("Lightning Bolt")).unwrap();
        assert_eq!((bolt.set_code.as_deref(), bolt.number.as_deref()), (Some("M11"), None));
        assert_eq!(deck.url, "https://mtgtop8.com/event?e=91675&d=896184&f=MO");

        let deck = mtgtop8(&list, Some(&page), "896184", Some("91675"), None).unwrap();
        assert_eq!(deck.format.as_deref(), Some("Constructed - Modern"));
        let deck = mtgtop8(&list, None, "896184", None, None).unwrap();
        assert_eq!((deck.name.as_deref(), deck.card_count()), (Some("MTGTop8 deck 896184"), 75));

        assert_eq!(reason(mtgtop8("// Deck file created with mtgtop8.com\n", None, "1", None, None)), Reason::NotFound);
        assert_eq!(reason(mtgtop8("<!DOCTYPE html><html></html>", None, "1", None, None)), Reason::SiteChanged);
        assert_eq!(mtgtop8("99999999999 [] Island", None, "1", None, None), Err(Failure::Shape));

        let id = |url: &str| Source::MtgTop8.deck_id(&link(url));
        assert_eq!(id("https://mtgtop8.com/event?e=91675&d=896184&f=MO").as_deref(), Some("896184"));
        assert_eq!(id("https://mtgtop8.com/dec?d=896184").as_deref(), Some("896184"));
        assert_eq!(id("https://mtgtop8.com/event?e=91675"), None);
        assert_eq!(id("https://mtgtop8.com/format?f=MO"), None);
    }

    #[test]
    fn scryfall_decks() {
        let deck =
            scryfall(&fixture("scryfall/commander-e7aceb4c.json"), "e7aceb4c-29d5-49f5-9a49-c24f64da264b").unwrap();
        assert_eq!(deck.name.as_deref(), Some("[mtg-parser] 3 Amigos"));
        assert_eq!(deck.author.as_deref(), Some("gorila"));
        assert_eq!(deck.format.as_deref(), Some("Variant Magic - Commander"));
        assert_eq!((count(&deck.commanders), count(&deck.side), deck.maybeboard_count), (2, 1, 0));
        assert_eq!(deck.cover.unwrap().set_code.as_deref(), Some("CMR"));
        assert_eq!(reason(scryfall(&fixture("scryfall/not-found.json"), "0")), Reason::NotFound);

        let id = |url: &str| Source::Scryfall.deck_id(&link(url));
        assert_eq!(
            id("https://scryfall.com/@gorila/decks/e7aceb4c-29d5-49f5-9a49-c24f64da264b").as_deref(),
            Some("e7aceb4c-29d5-49f5-9a49-c24f64da264b")
        );
        assert_eq!(
            id("https://api.scryfall.com/decks/E7ACEB4C-29D5-49F5-9A49-C24F64DA264B/export/json").as_deref(),
            Some("e7aceb4c-29d5-49f5-9a49-c24f64da264b")
        );
        assert_eq!(id("https://scryfall.com/card/m11/149/lightning-bolt"), None);
    }

    #[test]
    fn tcgplayer_decks() {
        let deck = tcgplayer(&fixture("tcgplayer/commander-496887.json"), "496887").unwrap();
        assert_eq!(deck.name.as_deref(), Some("Malcolm and Vial Smasher"));
        assert_eq!(deck.author.as_deref(), Some("Everett Delfel"));
        assert_eq!(deck.format.as_deref(), Some("Variant Magic - Commander"));
        assert_eq!((count(&deck.commanders), count(&deck.main)), (2, 98));
        assert_eq!(deck.description.as_deref(), Some("5th-8th at SCG Con Las Vegas cEDH $8K"));
        assert!(deck.main.iter().all(|c| c.number.is_none()));
        assert_eq!(deck.url, "https://infinite.tcgplayer.com/magic-the-gathering/deck/Malcolm-and-Vial-Smasher/496887");

        let deck = tcgplayer(&fixture("tcgplayer/modern-496500.json"), "496500").unwrap();
        assert_eq!(deck.format.as_deref(), Some("Constructed - Modern"));
        assert_eq!((count(&deck.main), count(&deck.side)), (60, 15));
        assert_eq!(reason(tcgplayer(&fixture("tcgplayer/not-found.json"), "560000")), Reason::NotFound);
    }

    #[test]
    fn manabox_decks() {
        let deck =
            manabox(&fixture("manabox/commander-AZ6V4n88fhahOvNRFHbXTQ.json"), "AZ6V4n88fhahOvNRFHbXTQ").unwrap();
        assert_eq!(deck.name.as_deref(), Some("Flash Fire"));
        assert_eq!(deck.format.as_deref(), Some("Variant Magic - Commander"));
        assert_eq!(names(&deck.commanders), vec!["Surrak Dragonclaw"]);
        assert_eq!((count(&deck.main), count(&deck.side)), (99, 3));
        assert_eq!(deck.remote_updated_at, Some(1_786_314_484_436));
        assert!(deck.cover.is_some());

        let deck =
            manabox(&fixture("manabox/constructed-NzWns2ueTeO8p0S0mDEkVw.json"), "NzWns2ueTeO8p0S0mDEkVw").unwrap();
        assert!(deck.format.is_none() && deck.commanders.is_empty());
        assert_eq!(count(&deck.main), 66);
        assert_eq!(reason(manabox("{\"id\":\"x\"}", "x")), Reason::SiteChanged);
    }

    #[test]
    fn names_and_formats() {
        assert_eq!(card_name(Some(" Fire/Ice ".into())).as_deref(), Some("Fire // Ice"));
        assert_eq!(card_name(Some("Wear  /  Tear".into())).as_deref(), Some("Wear // Tear"));
        assert_eq!(card_name(Some("a////b".into())).as_deref(), Some("a // // b"));
        assert_eq!(deck_type(Some(" Historic_Brawl ")), Some("Variant Magic - Brawl"));
        assert_eq!(deck_type(Some("canadian-highlander")), Some("Constructed - Canadian Highlander"));
        assert_eq!(deck_type(Some("gladiator")), None);
    }
}
