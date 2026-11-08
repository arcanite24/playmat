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
        "myHelperEmblems": helper_emblems(game),
        "opponentHands": {},
        "watchedHands": {},
        "stack": stack_view(game, names),
        "exiles": [cards_view(game, &crate::replay::exile_order(game))],
        "combat": combat_view(game, names),
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
    // on the battlefield, the face it's on as a permanent (`PermanentView`): an adventurer without its adventure, an
    // MDFC's played face
    // and its card (`PermanentView.original`): a double-faced card's face on its own
    let permanent = || {
        let kind = if game_object { "permanent" } else { "face" };
        (o.zone == Zone::Battlefield).then(|| templates().get(&format!("{}:{}", kind, o.def.name)).cloned()).flatten()
    };
    let mut v = token_view(game, id)
        .or_else(permanent)
        .or_else(|| templates().get(&o.def.name).cloned())
        .unwrap_or_else(|| json!({"name": o.def.name}));
    place_ids(&mut v, r, &format!("P{}", o.owner), &format!("P{}", o.controller), game_object);
    if let Value::Object(m) = &mut v {
        m.insert("id".into(), json!(r));
        m.entry("cardId").or_insert_with(|| json!(r));
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
        if game_object && o.zone == Zone::Battlefield {
            characteristics(game, id, m);
            permanent_state(game, id, m);
            let mut icons = permanent_icons(game, id);
            if spell_x(&o.def) {
                icons.push(x_icon(o.cast_x));
            }
            m.insert("cardIcons".into(), Value::Array(icons));
        }
    }
    v
}

/// What a card's view of a permanent shows of its state (`CardView(permanent)`): its power and toughness, its
/// counters, loyalty and defense
fn permanent_state(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    let o = game.obj(id);
    if game.obj(id).zone == Zone::Battlefield
        && (o.def.types.contains(&CardType::Creature) || game.has_type(id, CardType::Creature))
    {
        let (power, toughness) = game.power_toughness(id);
        let (base_power, base_toughness) = game.base_power_toughness(id);
        m.insert("power".into(), json!(power.to_string()));
        m.insert("toughness".into(), json!(toughness.to_string()));
        for (k, n, base) in [("originalPower", power, base_power), ("originalToughness", toughness, base_toughness)] {
            if let Some(Value::Object(x)) = m.get_mut(k) {
                x.insert("boostedValue".into(), json!(n));
                x.insert("modifiedBaseValue".into(), json!(base));
                x.insert("cardValue".into(), json!(base.to_string()));
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
}

/// A permanent's characteristics as effects left them: its colors, card types and subtypes (once its definition
/// changed: an animated land, a land given basic land types), a land's frame colors (the mana it can make), and the
/// color identity of what it has now (`ManaUtil.getColorIdentity`)
fn characteristics(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    const NAMES: [&str; 5] = ["white", "blue", "black", "red", "green"];
    let o = game.obj(id);
    let colors = game.colors(id);
    let five = |has: &dyn Fn(usize) -> bool| -> Value {
        Value::Object(NAMES.iter().enumerate().map(|(i, n)| (n.to_string(), json!(has(i)))).collect())
    };
    m.insert("color".into(), five(&|i| colors.contains(&crate::card::Color::ALL[i])));
    let changed = printed(&o.def.name).is_none_or(|p| !Arc::ptr_eq(&p, &o.def));
    if changed && !o.token {
        let types: Vec<Value> = o.def.types.iter().map(|t| json!(type_name(*t))).collect();
        m.insert("cardTypes".into(), Value::Array(types));
        let subtypes: Vec<Value> = o.def.subtypes.iter().map(|s| json!(subtype_name(s))).collect();
        m.insert("subTypes".into(), Value::Array(subtypes));
    }
    let rules: Vec<String> = m
        .get("rules")
        .and_then(|r| r.as_array())
        .map(|r| r.iter().filter_map(|x| x.as_str()).take_while(|x| *x != HINT_START).map(String::from).collect())
        .unwrap_or_default();
    if o.is(CardType::Land) {
        // the mana its mana abilities make
        let mut makes = [false; 5];
        for r in &rules {
            let Some((_, add)) = r.split_once("Add ") else { continue };
            for (i, c) in ["W", "U", "B", "R", "G"].iter().enumerate() {
                makes[i] |= add.contains("any color") || add.contains(&format!("{{{}}}", c));
            }
        }
        m.insert("frameColor".into(), five(&|i| makes[i]));
    }
    // color identity: its colors and the mana symbols of its cost and rules (not their reminder text)
    let mut texts: Vec<String> = rules;
    let strs = |v: Option<&Value>| -> Vec<String> {
        v.and_then(|x| x.as_array()).into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect()
    };
    texts.extend(strs(m.get("manaCostLeftStr")));
    texts.extend(strs(m.get("manaCostRightStr")));
    // and the back of a double-faced card on its front (`Card.getSecondCardFace`)
    if let Some(Value::Object(f)) = m.get("secondCardFace").filter(|f| f["backFace"] == json!(true)) {
        for k in ["rules", "manaCostLeftStr", "manaCostRightStr"] {
            texts.extend(strs(f.get(k)));
        }
    }

    let mut identity = String::new();
    for (i, c) in ['W', 'U', 'B', 'R', 'G'].iter().enumerate() {
        let has =
            colors.contains(&crate::card::Color::ALL[i]) || texts.iter().any(|t| has_symbol(&strip_italic(t), *c));
        if has {
            identity.push_str(&format!("{{{}}}", c));
        }
    }
    m.insert("originalColorIdentity".into(), json!(if identity.is_empty() { "{C}".to_string() } else { identity }));
}

/// Java's `ManaUtil` regex for a color's mana symbol: `{` up to two characters, the color, up to two, `}`
fn has_symbol(text: &str, c: char) -> bool {
    let b: Vec<char> = text.chars().collect();
    (0..b.len()).any(|i| {
        b[i] == c
            && (1..=3).any(|l| i >= l && b[i - l] == '{' && b[i - l + 1..i].iter().all(|x| *x != '{' && *x != '}'))
            && (1..=3)
                .any(|r| i + r < b.len() && b[i + r] == '}' && b[i + 1..i + r].iter().all(|x| *x != '{' && *x != '}'))
    })
}

/// a rule without its reminder text (`<i>…</i>`)
fn strip_italic(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("<i") {
        out.push_str(&rest[..i]);
        match rest[i..].find("</i>") {
            Some(j) => rest = &rest[i + j + 4..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn type_name(t: CardType) -> &'static str {
    match t {
        CardType::Artifact => "ARTIFACT",
        CardType::Battle => "BATTLE",
        CardType::Creature => "CREATURE",
        CardType::Enchantment => "ENCHANTMENT",
        CardType::Instant => "INSTANT",
        CardType::Kindred => "KINDRED",
        CardType::Land => "LAND",
        CardType::Planeswalker => "PLANESWALKER",
        CardType::Sorcery => "SORCERY",
    }
}

/// Java's `SubType` constant for a subtype ("Power-Plant" is `POWER_PLANT`)
fn subtype_name(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect()
}

/// A template's ids in place: `$` (the object's own) becomes its reference `r` (`$f0`, its first face, `rf0`), and
/// the card and each face it shows get the owner, the controller and whether it's the game's object
fn place_ids(v: &mut Value, r: &str, owner: &str, controller: &str, game_object: bool) {
    fn strings(v: &mut Value, r: &str) {
        match v {
            Value::String(t) if t.starts_with('$') && !t.starts_with("$P") => *t = format!("{}{}", r, &t[1..]),
            Value::Array(a) => a.iter_mut().for_each(|x| strings(x, r)),
            Value::Object(m) => m.values_mut().for_each(|x| strings(x, r)),
            _ => {}
        }
    }
    strings(v, r);
    let mut face = Some(v);
    while let Some(Value::Object(m)) = face {
        m.insert("ownerId".into(), json!(owner));
        m.insert("controllerId".into(), json!(controller));
        m.insert("gameObject".into(), json!(game_object));
        face = m.get_mut("secondCardFace").filter(|f| f.is_object());
    }
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
            A::WardAbility { rule, .. }
            | A::ColoredManaAbility { rule, .. }
            | A::SimpleActivatedAbility { rule, .. }
            | A::TriggeredAbility { rule, .. }
            | A::SimpleStaticAbility { rule, .. }
                if !rule.is_empty() =>
            {
                return Some(upper_first(rule))
            }
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
        // (an ability rewritten in place, Sunken Citadel's for its chosen color, keeps its line)
        for a in o.def.abilities.iter().filter(|a| !p.abilities.contains(a)) {
            if let Some(r) = ability_rule(a).filter(|r| !rules.iter().any(|x| x.as_str() == Some(r))) {
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
        "DayNightHint" => day_night_hint(game),
        _ => String::new(),
    }
}

/// `DayNightHint`
fn day_night_hint(game: &Game) -> String {
    match game.day {
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
    }
}

/// The combat groups (`CombatGroupView`), an attacker each, with its blockers: permanents' views for no player
fn combat_view(game: &Game, names: &[&str]) -> Value {
    let nobody = Seat::MAX;
    let mut groups = Vec::new();
    for id in game.battlefield.iter().copied() {
        let o = game.obj(id);
        let Some(defender) = o.attacking else { continue };
        let (defender_id, defender_name) = match o.attacking_pw {
            Some((pw, _)) if game.obj(pw).zone == Zone::Battlefield => {
                (reference(game, pw), game.obj(pw).def.name.clone())
            }
            _ => (format!("P{}", defender), names.get(defender).copied().unwrap_or("").to_string()),
        };
        let r = reference(game, id);
        let mut attackers = Map::new();
        attackers.insert(r.clone(), permanent_view(game, id, &r, nobody, names));
        let mut blockers = Map::new();
        for b in o.blocked_by.iter().copied().filter(|b| game.obj(*b).zone == Zone::Battlefield) {
            let r = reference(game, b);
            blockers.insert(r.clone(), permanent_view(game, b, &r, nobody, names));
        }
        groups.push(json!({
            "attackers": attackers,
            "blockers": blockers,
            "isBlocked": o.blocked,
            "defenderName": defender_name,
            "defenderId": defender_id,
        }));
    }
    Value::Array(groups)
}

/// The stack (`GameView`'s `stack`), by reference from the top: spells as their cards (`SpellView`), abilities as
/// `StackAbilityView`s with their source's view
fn stack_view(game: &Game, names: &[&str]) -> Value {
    let mut out = Map::new();
    for (i, item) in game.stack.iter().rev().enumerate() {
        let r = format!("S{}", i);
        let targets: Vec<Value> = item
            .targets
            .iter()
            .flatten()
            .map(|t| match t {
                crate::state::Target::Player(p) => json!(format!("P{}", p)),
                crate::state::Target::Obj(o) => json!(stack_ref(game, *o)),
            })
            .collect();
        let o = game.obj(item.card);
        let paid = game.unpaid != Some(game.stack.len() - 1 - i);
        let mut icons = Vec::new();
        let def = game.item_def(item);
        let spell = item.ability.is_none() && item.delayed.is_none();
        let ability = item.ability.and_then(|(k, _)| def.abilities.get(k));
        // the targets it declares, chosen or not (`CardView.targets`, set when its ability has targets)
        let declared = !item.targets.is_empty()
            || if spell {
                !def.spell.targets.is_empty()
            } else {
                ability.is_some_and(|a| {
                    a.triggered().is_some_and(|t| !t.2.is_empty()) || a.activated().is_some_and(|t| !t.2.is_empty())
                })
            };
        // (an X its costs announce shows once it's announced: before, the cost tag isn't there)
        let announced = paid || item.x != 0;
        let source_x = def.cost.x > 0
            || announced
                && if spell {
                    x_cost(&def.spell.costs)
                } else {
                    ability.and_then(|a| a.activated()).is_some_and(|t| x_cost(t.3))
                };
        if !spell && o.zone == Zone::Battlefield {
            // an ability's view shows its source's icons while it's a permanent
            icons = permanent_icons(game, item.card);
        }
        if source_x {
            icons.push(x_icon(item.x));
        }
        if !targets.is_empty() {
            icons.push(targets_icon(game, item, names));
        }
        let targets = declared.then_some(targets);
        let v = if item.ability.is_none() && item.delayed.is_none() {
            // a spell: its card's view
            let mut v = card_view(game, item.card, &r, true);
            if let Value::Object(m) = &mut v {
                m.insert("cardId".into(), json!("?"));
                m.insert("mageObjectType".into(), json!("SPELL"));
                m.insert("paid".into(), json!(paid));
                m.insert("cardIcons".into(), Value::Array(icons));
                if let Some(c) = m.get("color").cloned() {
                    m.insert("frameColor".into(), c);
                }
                if let Some(t) = targets {
                    m.insert("targets".into(), Value::Array(t));
                }
            }
            v
        } else {
            let mut v: Value = serde_json::from_str(STACK_ABILITY).unwrap_or_default();
            let def = game.item_def(item);
            let ability = item.ability.and_then(|(k, _)| def.abilities.get(k));
            let rule = match (&item.delayed, item.ability) {
                (Some(d), _) => d.rule.clone(),
                // its own rule, or its card's at its index when it has none (an effect's def can reorder them)
                (None, Some((k, _))) => ability
                    .and_then(|a| a.triggered().map(|t| t.0).or_else(|| a.activated().map(|t| t.0)))
                    .filter(|r| !r.is_empty())
                    .map(str::to_string)
                    .or_else(|| ability_rules(game, item.card).and_then(|r| r.get(k).cloned()))
                    .unwrap_or_default()
                    .replace("{this}", &o.def.name),
                _ => String::new(),
            };
            let triggered = item.delayed.is_some() || ability.is_some_and(|a| a.trigger().is_some());
            let kind = if o.token { "ABILITY_STACK_FROM_TOKEN" } else { "ABILITY_STACK_FROM_CARD" };
            let source_ref = reference(game, item.card);
            let mut source = card_view(game, item.card, &source_ref, true);
            source["mageObjectType"] = json!(kind);
            v["id"] = json!(r);
            v["rules"] = json!([rule]);
            v["mageObjectType"] = json!(kind);
            v["abilityType"] = json!(if triggered { "TRIGGERED_NONMANA" } else { "ACTIVATED_NONMANA" });
            if let Some(c) = source.get("counters").cloned() {
                v["counters"] = c;
            }
            v["paid"] = json!(paid);
            v["cardIcons"] = Value::Array(icons);
            if let Some(t) = targets {
                v["targets"] = Value::Array(t);
            }
            v["sourceCard"] = source;
            v
        };
        out.insert(r, v);
    }
    Value::Object(out)
}

fn icon(kind: &str, hint: &str, text: &str) -> Value {
    json!({"cardIconType": kind, "text": text, "hint": hint})
}

/// The icon an ability shows on its permanent (`Ability.getIcons(game)`)
fn ability_icon(game: &Game, id: ObjId, a: &crate::card::AbilityDef) -> Option<Value> {
    use crate::card::AbilityDef as A;
    let (kind, hint) = match a {
        A::FlyingAbility => ("ABILITY_FLYING", "Flying"),
        A::ReachAbility => ("ABILITY_REACH", "Reach"),
        A::VigilanceAbility => ("ABILITY_VIGILANCE", "Vigilance"),
        A::TrampleAbility => ("ABILITY_TRAMPLE", "Trample"),
        A::FirstStrikeAbility => ("ABILITY_FIRST_STRIKE", "First Strike"),
        A::DoubleStrikeAbility => ("ABILITY_DOUBLE_STRIKE", "Double Strike"),
        A::LifelinkAbility => ("ABILITY_LIFELINK", "Lifelink"),
        A::DeathtouchAbility => ("ABILITY_DEATHTOUCH", "Deathtouch"),
        A::DefenderAbility => ("ABILITY_DEFENDER", "Defender"),
        A::IndestructibleAbility => ("ABILITY_INDESTRUCTIBLE", "Indestructible"),
        A::HexproofAbility => ("ABILITY_HEXPROOF", "Hexproof from all"),
        A::WardAbility { rule, .. } => {
            return Some(icon("ABILITY_HEXPROOF", &upper_first(strip_italic(rule).trim()), ""));
        }
        A::ClassReminderAbility => {
            let n = game.obj(id).class_level;
            return Some(icon("ABILITY_CLASS_LEVEL", &format!("Current class level: {}", n), &n.to_string()));
        }
        _ => return None,
    };
    Some(icon(kind, hint, ""))
}

/// A permanent's icons from its abilities, printed then gained (`CardView.generateCardIconsForPermanent`)
fn permanent_icons(game: &Game, id: ObjId) -> Vec<Value> {
    let o = game.obj(id);
    o.def
        .abilities
        .iter()
        .cloned()
        .chain(game.granted_abilities(id))
        .filter_map(|a| ability_icon(game, id, &a))
        .collect()
}

/// Costs that announce an X: an X in a mana cost, a `SacrificeXTargetCost` (`CardUtil.checkSourceCostsTagExists`)
fn x_cost(costs: &[crate::card::CostDef]) -> bool {
    costs.iter().any(|c| match c {
        crate::card::CostDef::ManaCost { cost } => cost.contains("{X}"),
        crate::card::CostDef::SacrificeXTargetCost { .. } => true,
        _ => false,
    })
}

/// A card cast with an X: in its mana cost or its spell's costs
fn spell_x(def: &CardDef) -> bool {
    def.cost.x > 0 || x_cost(&def.spell.costs)
}

/// The X announced for a spell or ability, or for the spell a permanent was cast as
fn x_icon(x: i32) -> Value {
    icon("OTHER_COST_X", &format!("Announced X = {}", x), &format!("x={}", x))
}

/// A stack object's targets, each as its controller's name and its name with its reference (Java's id names,
/// `Name [c3b]`, have their short ids replaced by references when views are compared), sorted
fn targets_icon(game: &Game, item: &crate::state::StackItem, names: &[&str]) -> Value {
    let name = |p: Seat| names.get(p).copied().unwrap_or("").to_string();
    let mut lines: Vec<String> = item
        .targets
        .iter()
        .flatten()
        .map(|t| match t {
            crate::state::Target::Player(p) => name(*p),
            crate::state::Target::Obj(t) => {
                let o = game.obj(*t);
                let p = if matches!(o.zone, Zone::Battlefield | Zone::Stack) { o.controller } else { o.owner };
                format!("{}: {} [{}]", name(p), o.def.name, stack_ref(game, *t))
            }
        })
        .collect();
    lines.sort();
    let n = lines.len();
    icon(
        "OTHER_HAS_TARGETS",
        &format!("Has {} target(s). Move mouse over card to see target arrows:<br><br>{}", n, lines.join("<br>")),
        &format!("T-{}", n),
    )
}

/// A target's reference; a spell on the stack is its stack object's
fn stack_ref(game: &Game, id: ObjId) -> String {
    match game.stack.iter().rev().position(|t| t.card == id && t.ability.is_none() && t.delayed.is_none()) {
        Some(i) if game.obj(id).zone == Zone::Stack => format!("S{}", i),
        _ => reference(game, id),
    }
}

/// An object's abilities' rules, in order (from its card's or token's facts)
fn ability_rules(game: &Game, id: ObjId) -> Option<Vec<String>> {
    let o = game.obj(id);
    let key = match &o.token_image {
        Some((class, _, _)) => format!("abilities:token:{}", class),
        None => format!("abilities:{}", o.def.name),
    };
    let rules = templates().get(&key)?.as_array()?;
    Some(rules.iter().map(|r| r.as_str().unwrap_or("").to_string()).collect())
}

const STACK_ABILITY: &str = r#"{"name":"Ability","rules":[],"power":"","toughness":"","loyalty":"","defense":"","cardTypes":[],"subTypes":[],"superTypes":[],"color":{"white":false,"blue":false,"black":false,"red":false,"green":false},"manaCostLeftStr":[],"manaCostRightStr":[],"manaValue":0,"mageObjectType":"ABILITY_STACK_FROM_CARD","isAbility":false,"abilityType":"TRIGGERED_NONMANA","isToken":false,"imageFileName":"","imageNumber":0,"extraDeckCard":false,"transformable":false,"transformed":false,"backFace":false,"flipCard":false,"faceDown":false,"isSplitCard":false,"isDoubleFacedCard":false,"artRect":"NORMAL","paid":true,"controlledByOwner":true,"rotate":false,"hideInfo":false,"canAttack":false,"canBlock":false,"inViewerOnly":false,"cardIcons":[],"originalIsCopy":false,"id":"?","expansionSetCode":"","cardNumber":"0","usesVariousArt":false,"gameObject":true,"isChoosable":false,"isSelected":false,"playableStats":{"basicManaAbilities":[],"basicPlayAbilities":[],"basicCastAbilities":[],"other":[]}}"#;

/// The helper emblems' view (`GameImpl.initGameDefaultHelperEmblems`, from the game's start): all three have no
/// reference, so the last, day or night's, is the one left
fn helper_emblems(game: &Game) -> Value {
    if game.step.is_none() {
        return json!({});
    }
    let mut v: Value = serde_json::from_str(HELPER_EMBLEM).unwrap_or_default();
    v["rules"] = json!(["Day or night.", HINT_START, day_night_hint(game)]);
    json!({ "?": v })
}

const HELPER_EMBLEM: &str = r#"{"name":"Helper Emblem","displayName":"Helper Emblem","displayFullName":"Helper Emblem","rules":[],"power":"","toughness":"","loyalty":"","defense":"","startingLoyalty":"","startingDefense":"","cardTypes":[],"subTypes":[],"superTypes":[],"color":{"white":false,"blue":false,"black":false,"red":false,"green":false},"frameColor":{"white":false,"blue":false,"black":false,"red":false,"green":false},"frameStyle":"M15_NORMAL","manaCostLeftStr":[],"manaCostRightStr":[],"manaValue":0,"rarity":"SPECIAL","mageObjectType":"EMBLEM","isAbility":false,"isToken":false,"imageFileName":"","imageNumber":1,"extraDeckCard":false,"transformable":false,"transformed":false,"backFace":false,"flipCard":false,"faceDown":false,"isSplitCard":false,"isDoubleFacedCard":false,"artRect":"NORMAL","paid":false,"controlledByOwner":true,"rotate":false,"hideInfo":false,"canAttack":false,"canBlock":false,"inViewerOnly":false,"cardIcons":[],"originalIsCopy":false,"id":"?","expansionSetCode":"XMAGE","cardNumber":"","usesVariousArt":false,"gameObject":true,"isChoosable":false,"isSelected":false,"playableStats":{"basicManaAbilities":[],"basicPlayAbilities":[],"basicCastAbilities":[],"other":[]}}"#;

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
    // its info lines after the chosen color's, in the order of Java's `HashMap` of them: the protector, prepared
    if let Some(Value::Array(rules)) = m.get_mut("rules") {
        if let Some(p) = o.protector {
            let name = names.get(p).copied().unwrap_or("");
            rules.push(json!(format!("Protected by <font color='#20B2AA'>{}</font>", name)));
        }
        if o.prepared_copy.is_some() {
            rules.push(json!("<font color = 'blue'>Prepared</font>"));
        }
    }
    m.insert("mageObjectType".into(), json!("PERMANENT"));
    m.insert("tapped".into(), json!(o.tapped));
    m.insert("flipped".into(), json!(false));
    m.insert("phasedIn".into(), json!(true));
    // `Permanent.hasSummoningSickness`: not controlled since the turn began, without haste
    m.insert(
        "summoningSickness".into(),
        json!(o.sick && !game.has_ability(id, &crate::card::AbilityDef::HasteAbility)),
    );
    m.insert("damage".into(), json!(o.damage));
    // what's attached to it, in the order they were attached (`Permanent.getAttachments`), and what it's attached to
    let attachments: Vec<Value> = game
        .battlefield
        .iter()
        .filter(|a| game.attached(**a) == Some(id))
        .map(|a| json!(reference(game, *a)))
        .collect();
    m.insert("attachments".into(), Value::Array(attachments));
    if let Some(to) = game.attached(id) {
        m.insert("attachedTo".into(), json!(reference(game, to)));
    }
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
    for k in ["morphed", "disguised", "manifested", "cloaked"] {
        m.insert(k.into(), json!(false));
    }
    let to = game.attached(id);
    m.insert("attachedToPermanent".into(), json!(to.is_some()));
    m.insert("attachedControllerDiffers".into(), json!(to.is_some_and(|t| game.obj(t).controller != o.controller)));
    m.insert("mutateView".into(), json!({}));
    m.insert("mutated".into(), json!(false));
    m.insert("roomLeftUnlocked".into(), json!(false));
    m.insert("roomRightUnlocked".into(), json!(false));
    m.insert("bandedCards".into(), json!([]));
    m.insert("controlledByOwner".into(), json!(o.owner == o.controller));
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
