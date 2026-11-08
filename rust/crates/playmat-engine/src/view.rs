//! Views (docs/rust/Views.md): what the web client sees, as Java's `JsonCodec` writes a `GameView`, with ids as
//! decision-log references, and the field-by-field comparison with Java's.

use crate::card::{Amount, CardDb, CardDef, CardType, HintDef};
use crate::replay::reference;
use crate::state::{Game, ObjId, Seat, Step, Zone};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

/// Each card's view as Java builds it out of a game (`new CardView(card)`, written by `CardFactsTest` with
/// `-Dxmage.cardFacts.views`), by name, without its ids: what the engine's card views start from.
pub const VIEWS_JSON: &str = include_str!("../cards/views.json");

fn templates() -> &'static HashMap<String, Value> {
    static T: OnceLock<HashMap<String, Value>> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(VIEWS_JSON).unwrap_or_default())
}

/// The view `seat`'s client gets of the game, as Java's `GameSessionPlayer.prepareGameView` builds it and `JsonCodec`
/// writes it, with ids as decision-log references. `names` are the players' names, by seat.
pub fn game_view(game: &Game, seat: Seat, names: &[&str]) -> Value {
    let name = |s: Option<Seat>| s.and_then(|s| names.get(s)).map(|n| n.to_string()).unwrap_or_default();
    let players: Vec<Value> = (0..game.seats()).map(|p| player_view(game, p, seat, names)).collect();
    json!({
        "players": players,
        "myPlayerId": format!("P{}", seat),
        "phase": game.step.map(phase_of).unwrap_or(""),
        "step": game.step.map(|s| s.name()).unwrap_or(""),
        "turn": game.turn,
        "activePlayerId": game.active.map(|a| format!("P{}", a)),
        "activePlayerName": name(game.active),
        "priorityPlayerName": name(game.priority),
        "special": false,
        "rollbackTurnsAllowed": false,
        "attackOption": "LEFT",
        "rangeOfInfluence": "ALL",
        "myHand": cards_view(game, &game.player(seat).hand),
        "myHelperEmblems": {},
        "opponentHands": {},
        "watchedHands": {},
        "stack": {},
        "exiles": [cards_view(game, &crate::replay::exile_order(game))],
        "combat": [],
        "revealed": [],
        "lookedAt": [],
        "companion": [],
    })
}

/// `TurnPhase` of a step
fn phase_of(step: Step) -> &'static str {
    match step {
        Step::Untap | Step::Upkeep | Step::Draw => "BEGINNING",
        Step::PrecombatMain => "PRECOMBAT_MAIN",
        Step::BeginCombat
        | Step::DeclareAttackers
        | Step::DeclareBlockers
        | Step::FirstCombatDamage
        | Step::CombatDamage
        | Step::EndCombat => "COMBAT",
        Step::PostcombatMain => "POSTCOMBAT_MAIN",
        Step::EndTurn | Step::Cleanup => "END",
    }
}

/// `PlayerView`: seat `p` as `viewer` sees it
fn player_view(game: &Game, p: Seat, viewer: Seat, names: &[&str]) -> Value {
    let pl = game.player(p);
    let exile = crate::replay::exile_order(game);
    let mut pool = pl.mana_pool;
    for (m, _, _) in &pl.conditional_pool {
        for k in 0..5 {
            pool.colored[k] += m.colored[k];
        }
        pool.colorless += m.colorless;
    }
    json!({
        "playerId": format!("P{}", p),
        "name": names.get(p).copied().unwrap_or(""),
        "controlled": p == viewer,
        "isHuman": true,
        "life": pl.life,
        "counters": pl.counters.iter().map(|(n, c)| json!({"name": n, "count": c})).collect::<Vec<_>>(),
        "wins": 0,
        "winsNeeded": 0,
        "libraryCount": pl.library.len(),
        "handCount": pl.hand.len(),
        "isActive": game.active == Some(p),
        "hasPriority": game.priority == Some(p),
        "hasLeft": false,
        "manaPool": {
            "white": pool.colored[0], "blue": pool.colored[1], "black": pool.colored[2],
            "red": pool.colored[3], "green": pool.colored[4], "colorless": pool.colorless,
        },
        "battlefield": battlefield_view(game, p, viewer, names),
        "graveyard": cards_view(game, &pl.graveyard),
        "exile": cards_view(game, &exile.iter().copied().filter(|c| game.obj(*c).owner == p).collect::<Vec<_>>()),
        "attachments": [],
        "commandList": [],
        "designationNames": [],
        "helperCards": {},
        "sideboard": {},
        "passedTurn": false,
        "passedUntilEndOfTurn": false,
        "passedUntilNextMain": false,
        "passedUntilStackResolved": false,
        "passedAllTurns": false,
        "passedUntilEndStepBeforeMyTurn": false,
        "monarch": false,
        "initiative": false,
    })
}

/// A zone's cards (`CardsView`), by reference
fn cards_view(game: &Game, ids: &[ObjId]) -> Value {
    let mut m = Map::new();
    for id in ids {
        let r = reference(game, *id);
        m.insert(r.clone(), card_view(game, *id, &r, true));
    }
    Value::Object(m)
}

/// `CardView`: the card's template with its reference, owner and controller (`game_object`: built from the game,
/// not from the card alone, as `original` is)
fn card_view(game: &Game, id: ObjId, r: &str, game_object: bool) -> Value {
    let o = game.obj(id);
    let mut v = token_view(game, id)
        .or_else(|| templates().get(&o.def.name).cloned())
        .unwrap_or_else(|| json!({"name": o.def.name}));
    if let Value::Object(m) = &mut v {
        m.insert("id".into(), json!(r));
        m.insert("cardId".into(), json!(r));
        m.insert("ownerId".into(), json!(format!("P{}", o.owner)));
        m.insert("controllerId".into(), json!(format!("P{}", o.controller)));
        m.insert("gameObject".into(), json!(game_object));
        let hints = if game_object { hint_lines(game, id) } else { Vec::new() };
        if let (true, Some(Value::Array(rules))) = (game_object, m.get_mut("rules")) {
            changed_rules(game, id, rules);
        }
        if let Some(Value::Array(rules)) = m.get_mut("rules") {
            if !hints.is_empty() {
                rules.push(json!(HINT_START));
                rules.extend(hints.into_iter().map(Value::String));
            }
            // a permanent's info comes after its hints (`PermanentImpl.getRules`)
            if game_object && o.zone == Zone::Battlefield {
                rules.extend(o.info.iter().map(|(_, l)| json!(l)));
            }
        }
        if let Some(Value::Object(f)) = m.get_mut("secondCardFace") {
            f.insert("id".into(), json!(format!("{}f1", r)));
            f.insert("cardId".into(), json!(r));
            f.insert("ownerId".into(), json!(format!("P{}", o.owner)));
            f.insert("controllerId".into(), json!(format!("P{}", o.controller)));
            f.insert("gameObject".into(), json!(game_object));
        }
    }
    v
}

/// A granted ability's rule (`Ability.getRule`), for the abilities effects give
fn ability_rule(a: &crate::card::AbilityDef) -> Option<String> {
    use crate::card::AbilityDef as A;
    Some(
        match a {
            A::FlyingAbility => "Flying",
            A::ReachAbility => "Reach",
            A::HasteAbility => "Haste",
            A::VigilanceAbility => "Vigilance",
            A::TrampleAbility => "Trample",
            A::FirstStrikeAbility => "First strike",
            A::DoubleStrikeAbility => "Double strike",
            A::LifelinkAbility => "Lifelink",
            A::DeathtouchAbility => "Deathtouch",
            A::DefenderAbility => "Defender",
            A::MenaceAbility => "Menace",
            A::HexproofAbility => "Hexproof",
            A::IndestructibleAbility => "Indestructible",
            A::FlashAbility => "Flash",
            A::WardAbility { rule, .. } => return Some(upper_first(rule)),
            A::WhiteManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {W}.",
            A::BlueManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {U}.",
            A::BlackManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {B}.",
            A::RedManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {R}.",
            A::GreenManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {G}.",
            A::ColorlessManaAbility { effects, cost } if effects.is_empty() && cost.is_empty() => "{T}: Add {C}.",
            A::AnyColorManaAbility { amount: 1, effects, cost, .. } if effects.is_empty() && cost.is_empty() => {
                "{T}: Add one mana of any color."
            }
            A::ProtectionFromColorAbility { c } => {
                return Some(format!("Protection from {}", color_name(*c)));
            }
            _ => return None,
        }
        .to_string(),
    )
}

fn color_name(c: crate::card::Color) -> &'static str {
    use crate::card::Color as C;
    match c {
        C::W => "White",
        C::U => "Blue",
        C::B => "Black",
        C::R => "Red",
        C::G => "Green",
    }
}

/// The printed faces, by name: what a card's abilities were before effects changed its definition
fn printed(name: &str) -> Option<Arc<CardDef>> {
    static DB: OnceLock<HashMap<String, Arc<CardDef>>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut m = HashMap::new();
        let Ok((db, _)) = CardDb::load_json(crate::CARDS_JSON) else { return m };
        fn add(m: &mut HashMap<String, Arc<CardDef>>, d: &Arc<CardDef>) {
            m.entry(d.name.clone()).or_insert_with(|| d.clone());
            for f in [&d.adventure, &d.back, &d.transform_back, &d.prepare].into_iter().flatten() {
                add(m, f);
            }
            for f in &d.room {
                add(m, f);
            }
        }
        for d in db.iter() {
            add(&mut m, d);
        }
        m
    })
    .get(name)
    .cloned()
}

/// A permanent's rules as effects changed them: the printed abilities it lost taken out, the ones its definition
/// gained and the ones effects give it added (`Permanent.getAbilities(game).getRules()`)
fn changed_rules(game: &Game, id: ObjId, rules: &mut Vec<Value>) {
    let o = game.obj(id);
    if let Some(p) = printed(&o.def.name).filter(|p| !Arc::ptr_eq(p, &o.def)) {
        for a in p.abilities.iter().filter(|a| !o.def.abilities.contains(a)) {
            if let Some(r) = ability_rule(a) {
                rules.retain(|x| x.as_str() != Some(&r));
            }
        }
        for a in o.def.abilities.iter().filter(|a| !p.abilities.contains(a)) {
            if let Some(r) = ability_rule(a) {
                rules.push(Value::String(r));
            }
        }
    }
    for a in game.granted_abilities(id) {
        if let Some(r) = ability_rule(&a) {
            rules.push(Value::String(r));
        }
    }
}

/// Where a card's hints start in its rules (`HintUtils.HINT_START_MARK`)
const HINT_START: &str = "<br/><hintstart/>";

/// The hints of a card's abilities in a started game, each once (`CardUtil.getCardRulesWithAdditionalInfo`)
fn hint_lines(game: &Game, id: ObjId) -> Vec<String> {
    if game.step.is_none() {
        return Vec::new();
    }
    let o = game.obj(id);
    let you = o.controller;
    let mut out: Vec<String> = Vec::new();
    for h in &o.def.hints {
        let text = match h {
            HintDef::Condition { condition, icons, true_text, false_text, true_color, false_color } => {
                let holds = game.condition_holds(Some(condition), you, id);
                let (text, color) = if holds { (true_text, true_color) } else { (false_text, false_color) };
                let text = upper_first(text.as_deref().unwrap_or(""));
                let text = match color {
                    Some(c) => format!("<font color={}>{}</font>", c, text),
                    None => text,
                };
                let icon = if !*icons {
                    ""
                } else if holds {
                    "ICON_GOOD"
                } else {
                    "ICON_BAD"
                };
                format!("{}{}", icon, text)
            }
            HintDef::Value { name, value } => {
                format!("{}: {}", name, game.value(&Amount::Of(value.clone()), you, id, 0))
            }
            HintDef::Named { name } => named_hint(game, name, you, id),
            HintDef::Unknown { .. } => String::new(),
        };
        if !text.is_empty() && !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// The hint classes of their own (`mage.abilities.hint.common`)
fn named_hint(game: &Game, name: &str, you: Seat, id: ObjId) -> String {
    match name {
        "CanPlayAdditionalLandsHint" => {
            let per_turn = crate::rules::lands_per_turn(game, you);
            if per_turn == 1 {
                return String::new();
            }
            let played = game.player(you).lands_played;
            let stats = format!(" (played {} of {})", played, per_turn);
            if played < per_turn {
                format!("ICON_GOODCan play more lands{}", stats)
            } else {
                format!("ICON_BADCan't play lands{}", stats)
            }
        }
        "ClassLevelHint" => match game.obj(id).zone {
            Zone::Battlefield => format!("Class level: {}", game.obj(id).class_level.max(1)),
            _ => String::new(),
        },
        "OpenSideboardHint" => {
            "Cards in your sideboard: 0 (<i>Right click on battlefield to open player's sideboard at any time</i>)"
                .into()
        }
        "DayNightHint" => match game.day {
            None => "It's neither day nor night.".into(),
            Some(day) => {
                let spells = game.active.map(|a| game.player(a).spells_cast).unwrap_or(0);
                let changes = if day { spells == 0 } else { spells >= 2 };
                format!(
                    "It's currently {}, active player has cast {} spells this turn. It will {} become {} next turn.",
                    if day { "day" } else { "night" },
                    spells,
                    if changes { "" } else { "not" },
                    if day { "night" } else { "day" }
                )
            }
        },
        _ => String::new(),
    }
}

/// A token made from a token class: its class's view, with the image the random pick took (`TokenImpl`)
fn token_view(game: &Game, id: ObjId) -> Option<Value> {
    let o = game.obj(id);
    let (class, set, pick) = o.token_image.as_ref()?;
    let t = templates().get(&format!("token:{}", class))?;
    let mut v = t.get("view")?.clone();
    let images: Vec<(String, u64)> = t
        .get("images")
        .and_then(|i| i.as_array())
        .map(|a| a.iter().filter_map(|x| Some((x.get(0)?.as_str()?.to_string(), x.get(1)?.as_u64()?))).collect())
        .unwrap_or_default();
    let of_set: Vec<&(String, u64)> = images.iter().filter(|(s, _)| s == set).collect();
    let choices: Vec<&(String, u64)> = if of_set.is_empty() { images.iter().collect() } else { of_set };
    if let (Some((s, n)), Value::Object(m)) = (choices.get(*pick as usize), &mut v) {
        m.insert("expansionSetCode".into(), json!(s));
        m.insert("imageNumber".into(), json!(n));
        m.insert("rarity".into(), json!("SPECIAL"));
        m.insert("isToken".into(), json!(true));
        m.insert("ownerId".into(), json!(format!("P{}", o.owner)));
        m.insert("controllerId".into(), json!(format!("P{}", o.controller)));
        // `CardView` capitalizes each rule
        if let Some(Value::Array(rules)) = m.get_mut("rules") {
            for x in rules.iter_mut() {
                if let Some(t) = x.as_str() {
                    *x = json!(upper_first(t));
                }
            }
        }
    }
    Some(v)
}

/// A player's permanents (`PermanentView`), by reference
fn battlefield_view(game: &Game, p: Seat, viewer: Seat, names: &[&str]) -> Value {
    let mut m = Map::new();
    for id in game.battlefield.iter().copied() {
        if game.obj(id).controller == p {
            let r = reference(game, id);
            m.insert(r.clone(), permanent_view(game, id, &r, viewer, names));
        }
    }
    Value::Object(m)
}

/// `PermanentView`: the card's view as a permanent, with its state, and (for its controller, or a token) the card
/// it was printed as
fn permanent_view(game: &Game, id: ObjId, r: &str, viewer: Seat, names: &[&str]) -> Value {
    let o = game.obj(id);
    let mut v = card_view(game, id, r, true);
    let controlled = o.controller == viewer;
    let Value::Object(m) = &mut v else { return v };
    m.insert("mageObjectType".into(), json!("PERMANENT"));
    m.insert("tapped".into(), json!(o.tapped));
    m.insert("flipped".into(), json!(false));
    m.insert("phasedIn".into(), json!(true));
    m.insert("summoningSickness".into(), json!(o.sick));
    m.insert("damage".into(), json!(o.damage));
    m.insert("attachments".into(), json!([]));
    if let Some(t) = token_view(game, id) {
        // `PermanentView`: a token's original is the token as made, without the game's
        let mut original = t.clone();
        if let Value::Object(x) = &mut original {
            x.insert("id".into(), json!("?"));
            x.insert("gameObject".into(), json!(false));
            for k in ["originalPower", "originalToughness", "originalColorIdentity", "ownerId", "controllerId"] {
                x.remove(k);
            }
        }
        m.insert("original".into(), original);
        if let Value::Object(t) = t {
            for (k, x) in t {
                if !matches!(k.as_str(), "id" | "cardId" | "gameObject" | "rules") {
                    m.insert(k, x);
                }
            }
        }
        m.insert("mageObjectType".into(), json!("TOKEN"));
    } else if controlled || o.token {
        m.insert("original".into(), card_view(game, id, r, false));
    }
    m.insert("copy".into(), json!(false));
    m.insert(
        "nameOwner".into(),
        json!(if o.owner != o.controller { names.get(o.owner).copied().unwrap_or("") } else { "" }),
    );
    m.insert("nameController".into(), json!(names.get(o.controller).copied().unwrap_or("")));
    m.insert("controlled".into(), json!(controlled));
    for k in ["morphed", "disguised", "manifested", "cloaked", "attachedToPermanent", "attachedControllerDiffers"] {
        m.insert(k.into(), json!(false));
    }
    m.insert("mutateView".into(), json!({}));
    m.insert("mutated".into(), json!(false));
    m.insert("roomLeftUnlocked".into(), json!(false));
    m.insert("roomRightUnlocked".into(), json!(false));
    m.insert("bandedCards".into(), json!([]));
    if game.obj(id).zone == Zone::Battlefield
        && (o.def.types.contains(&CardType::Creature) || game.has_type(id, CardType::Creature))
    {
        let (power, toughness) = game.power_toughness(id);
        m.insert("power".into(), json!(power.to_string()));
        m.insert("toughness".into(), json!(toughness.to_string()));
        for (k, n) in [("originalPower", power), ("originalToughness", toughness)] {
            if let Some(Value::Object(x)) = m.get_mut(k) {
                x.insert("boostedValue".into(), json!(n));
            }
        }
    }
    // `CardView`: with counters, they're listed, and its loyalty and defense are counted
    let counters: Vec<&(String, i32)> = o.counters.iter().filter(|(_, n)| *n > 0).collect();
    if !counters.is_empty() {
        m.insert("loyalty".into(), json!(o.counter("loyalty").to_string()));
        m.insert("defense".into(), json!(o.counter("defense").to_string()));
        m.insert(
            "counters".into(),
            json!(counters.iter().map(|(n, c)| json!({"name": n, "count": c})).collect::<Vec<_>>()),
        );
    }
    if o.owner != o.controller {
        m.insert("controlledByOwner".into(), json!(false));
    }
    v
}

/// Fields left out of the comparison, by name anywhere in the view: they can't match by design.
pub const IGNORED: &[(&str, &str)] = &[
    ("priorityTime", "the priority timer"),
    ("bufferTime", "the priority timer"),
    ("priorityTimeLeftSecs", "the priority timer"),
    ("priorityTimeSavedTimeMs", "the priority timer"),
    ("bufferTimeLeft", "the priority timer"),
    ("timerActive", "the priority timer"),
    ("totalErrorsCount", "Java's own error counters"),
    ("totalEffectsCount", "Java's own effect counters"),
    ("gameCycle", "Java's own state counter"),
    ("userData", "the player's settings, which the host passes through"),
    ("statesSavedSize", "Java's rollback states"),
];

/// One field that differs: its path (`players[0].battlefield{B3}.tapped`), Java's value and the engine's.
#[derive(Clone, Debug, PartialEq)]
pub struct Difference {
    pub path: String,
    pub java: String,
    pub rust: String,
}

/// The path with list indexes and map keys left out (`players[].battlefield{}.tapped`), to count differences by field.
pub fn field_of(path: &str) -> String {
    let mut out = String::new();
    let mut skip: Option<char> = None;
    for c in path.chars() {
        match (skip, c) {
            (None, '[') => {
                skip = Some(']');
                out.push('[');
            }
            (None, '{') => {
                skip = Some('}');
                out.push('{');
            }
            (Some(end), c) if c == end => {
                skip = None;
                out.push(c);
            }
            (Some(_), _) => {}
            (None, c) => out.push(c),
        }
    }
    out
}

/// A map key that is an object's reference (`B3`, `H0.2`, `?`), not a field name.
fn is_ref(key: &str) -> bool {
    let mut c = key.chars();
    key == "?" || (c.next().is_some_and(|c| c.is_ascii_uppercase()) && c.next().is_some_and(|c| c.is_ascii_digit()))
}

fn ignored(key: &str) -> bool {
    IGNORED.iter().any(|(k, _)| *k == key)
}

fn short(v: Option<&Value>) -> String {
    match v {
        None => "(missing)".into(),
        Some(v) => {
            let s = v.to_string();
            if s.len() > 200 {
                format!("{}…", &s[..s.char_indices().nth(200).map(|(i, _)| i).unwrap_or(s.len())])
            } else {
                s
            }
        }
    }
}

/// An object's key in a list: its `id` when it has one.
fn list_key(v: &Value) -> Option<String> {
    v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string())
}

/// Every field where the two views differ. Maps match by key; lists of objects with ids by id, other lists by place.
pub fn compare(java: &Value, rust: &Value) -> Vec<Difference> {
    let mut out = Vec::new();
    diff(&mut out, String::new(), Some(java), Some(rust));
    out
}

fn diff(out: &mut Vec<Difference>, path: String, java: Option<&Value>, rust: Option<&Value>) {
    // Gson leaves out nulls: a null is the same as a missing field
    let java = java.filter(|v| !v.is_null());
    let rust = rust.filter(|v| !v.is_null());
    match (java, rust) {
        (None, None) => {}
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let mut keys: Vec<&String> = a.keys().collect();
            for k in b.keys() {
                if !a.contains_key(k) {
                    keys.push(k);
                }
            }
            for k in keys {
                if ignored(k) {
                    continue;
                }
                let p = if is_ref(k) {
                    format!("{}{{{}}}", path, k)
                } else if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                diff(out, p, a.get(k), b.get(k));
            }
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) => {
            let keyed = !a.is_empty() && a.iter().chain(b.iter()).all(|v| list_key(v).is_some());
            if keyed {
                let mut keys: Vec<String> = a.iter().filter_map(list_key).collect();
                for k in b.iter().filter_map(list_key) {
                    if !keys.contains(&k) {
                        keys.push(k);
                    }
                }
                for k in keys {
                    let x = a.iter().find(|v| list_key(v).as_deref() == Some(&k));
                    let y = b.iter().find(|v| list_key(v).as_deref() == Some(&k));
                    diff(out, format!("{}[{}]", path, k), x, y);
                }
            } else if a.len() != b.len() || a.iter().any(|v| v.is_object() || v.is_array()) {
                for i in 0..a.len().max(b.len()) {
                    diff(out, format!("{}[{}]", path, i), a.get(i), b.get(i));
                }
            } else if a != b {
                out.push(Difference { path, java: short(java), rust: short(rust) });
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(Difference { path, java: short(a), rust: short(b) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_by_field() {
        let java = json!({"turn": 3, "players": [{"life": 20, "battlefield": {"B0": {"tapped": true}}}],
                          "priorityTime": 5, "stack": {}, "gone": null});
        let rust = json!({"turn": 3, "players": [{"life": 18, "battlefield": {"B0": {"tapped": false}}}],
                          "priorityTime": 9, "stack": {}});
        let d = compare(&java, &rust);
        let paths: Vec<&str> = d.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, vec!["players[0].life", "players[0].battlefield{B0}.tapped"]);
        assert_eq!(field_of("players[0].hand{H0.1}.rules[2]"), "players[].hand{}.rules[]");
    }

    #[test]
    fn lists_with_ids_match_by_id() {
        let java = json!({"combat": [{"id": "B1", "x": 1}, {"id": "B2", "x": 2}]});
        let rust = json!({"combat": [{"id": "B2", "x": 2}, {"id": "B1", "x": 1}]});
        assert!(compare(&java, &rust).is_empty());
    }
}
