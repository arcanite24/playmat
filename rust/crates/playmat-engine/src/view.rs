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

/// The game as Java has it, where it differs: a card entering from a hand while its as-enters choices are asked is
/// still in the hand in Java
pub fn as_java(game: &Game) -> Option<Game> {
    let (id, owner, i) = game.entering_from_hand.filter(|(id, _, _)| game.obj(*id).zone == Zone::Battlefield)?;
    let mut g = game.clone();
    Arc::make_mut(&mut g.battlefield).retain(|x| *x != id);
    let hand = &mut g.player_mut(owner).hand;
    hand.insert(i.min(hand.len()), id);
    g.obj_mut(id).zone = Zone::Hand;
    Some(g)
}

/// The view `seat`'s client gets of the game, as Java's `GameSessionPlayer.prepareGameView` builds it and `JsonCodec`
/// writes it, with ids as decision-log references. `names` are the players' names, by seat.
pub fn game_view(game: &Game, seat: Seat, names: &[&str]) -> Value {
    // a card entering from a hand while its as-enters choices are asked is still in the hand in Java
    if let Some(g) = as_java(game) {
        return game_view(&g, seat, names);
    }
    let name = |s: Option<Seat>| s.and_then(|s| names.get(s)).map(|n| n.to_string()).unwrap_or_default();
    let players: Vec<Value> = (0..game.seats()).map(|p| player_view(game, p, seat, names)).collect();
    let mut v = json!({
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
        "exiles": exiles_view(game),
        "combat": combat_view(game, names),
        "revealed": revealed_view(game),
        "lookedAt": [],
        "companion": [],
    });
    if let (Some(_), true) = (game.step, game.priority == Some(seat)) {
        v["canPlayObjects"] = can_play_objects(game, seat);
    }
    face_down_exiles(game, seat, &mut v);
    v
}

const FORETOLD: &str = r#"{"rules":[],"power":"","toughness":"","loyalty":"","defense":"","startingLoyalty":"","startingDefense":"","cardTypes":[],"subTypes":[],"superTypes":[],"color":{"white":false,"blue":false,"black":false,"red":false,"green":false},"frameColor":{"white":false,"blue":false,"black":false,"red":false,"green":false},"frameStyle":"M15_NORMAL","manaCostLeftStr":[],"manaCostRightStr":[],"manaValue":0,"rarity":"SPECIAL","mageObjectType":"CARD","isAbility":false,"isToken":false,"imageFileName":"Foretell","imageNumber":1,"extraDeckCard":false,"transformable":false,"transformed":false,"backFace":false,"flipCard":false,"faceDown":true,"isSplitCard":false,"isDoubleFacedCard":false,"artRect":"NORMAL","paid":false,"controlledByOwner":true,"rotate":false,"hideInfo":false,"canAttack":false,"canBlock":false,"inViewerOnly":false,"cardIcons":[],"originalIsCopy":false,"expansionSetCode":"XMAGE","cardNumber":"0","usesVariousArt":false,"gameObject":true,"isChoosable":false,"isSelected":false,"playableStats":{"basicManaAbilities":[],"basicPlayAbilities":[],"basicCastAbilities":[],"other":[]}}"#;

/// Foretold cards exiled face down (`ForetellAbility`): "Foretell" to everyone, and to their owner "Foretell: Name"
/// with the card as its other face
fn face_down_exiles(game: &Game, seat: Seat, v: &mut Value) {
    for id in game.exile.iter().copied().filter(|c| game.obj(*c).foretold_turn.is_some()) {
        let o = game.obj(id);
        let r = reference(game, id);
        let mut card: Value = serde_json::from_str(FORETOLD).unwrap_or_default();
        let own = o.owner == seat;
        let name = if own { format!("Foretell: {}", o.def.name) } else { "Foretell".to_string() };
        for (k, x) in [
            ("cardId", json!(r)),
            ("id", json!(r)),
            ("ownerId", json!(format!("P{}", o.owner))),
            ("controllerId", json!(format!("P{}", o.owner))),
            ("name", json!(name)),
            ("displayName", json!(name)),
            ("displayFullName", json!(name)),
            ("alternateName", json!(if own { o.def.name.as_str() } else { "Foretell" })),
        ] {
            card[k] = x;
        }
        if own {
            card["transformable"] = json!(true);
            card["secondCardFace"] = card_view(game, id, &r, false);
        }
        let put = |zone: &mut Value| {
            if let Some(Value::Object(m)) = Some(zone).filter(|z| z.get(&r).is_some()) {
                m.insert(r.clone(), card.clone());
            }
        };
        if let Some(Value::Array(players)) = v.get_mut("players") {
            for p in players {
                if let Some(e) = p.get_mut("exile") {
                    put(e);
                }
            }
        }
        if let Some(Value::Array(zones)) = v.get_mut("exiles") {
            for z in zones {
                put(z);
            }
        }
    }
}

/// What the priority player can play now, per object: each ability's reference and its text, in the client's groups
/// (`PlayerImpl.getPlayableObjects`, `PlayableObjectStats`)
fn can_play_objects(game: &Game, seat: Seat) -> Value {
    use crate::rules::PlayKind;
    let mut objects = Map::new();
    for (id, k, kind, face) in crate::rules::playable_abilities(game, seat) {
        let o = game.obj(id);
        let r = reference(game, id);
        // its card's text at its index, unless effects changed its abilities: then its own
        let changed = printed(&o.def.name).is_some_and(|p| p.abilities.get(k) != o.def.abilities.get(k));
        let card_text = ability_rules(game, id).and_then(|r| r.get(k).cloned()).filter(|t| !t.is_empty());
        let own_text = o
            .def
            .abilities
            .get(k)
            .and_then(|a| {
                a.triggered()
                    .map(|t| t.0.to_string())
                    .or_else(|| a.activated().map(|t| t.0.to_string()))
                    .filter(|t| !t.is_empty())
                    .or_else(|| ability_rule(a))
            })
            .filter(|t| !t.is_empty());
        let text = if changed { own_text.or(card_text) } else { card_text.or(own_text) };
        let name = match face {
            Some(1) => o.def.back.as_ref().map_or(&o.def.name, |b| &b.name),
            _ => &o.def.name,
        };
        let text = match kind {
            PlayKind::Cast => format!("Cast {}", name),
            PlayKind::RoomHalf => format!("Cast {}", o.def.room.get(k).map_or("", |h| h.name.as_str())),
            PlayKind::Adventure => format!("Cast {}", o.def.adventure.as_ref().map_or("", |a| a.name.as_str())),
            PlayKind::Plotted => format!("Cast {} using Plot", name),
            PlayKind::Foretold => {
                let cost = o.def.abilities.iter().find_map(|a| match a {
                    crate::card::AbilityDef::ForetellAbility { cost, .. } => Some(cost.as_str()),
                    _ => None,
                });
                format!("Foretell {}", cost.unwrap_or(""))
            }
            PlayKind::Other if o.def.abilities.get(k) == Some(&crate::card::AbilityDef::SpellAbility) => {
                format!("Cast {}", name)
            }
            PlayKind::Other
                if matches!(o.def.abilities.get(k), Some(crate::card::AbilityDef::FlashbackAbility { .. })) =>
            {
                // (`FlashbackAbility`'s own name, without its reminder)
                match o.def.abilities.get(k) {
                    Some(crate::card::AbilityDef::FlashbackAbility { cost, .. }) => format!("Flashback {}", cost),
                    _ => String::new(),
                }
            }
            PlayKind::Other if matches!(o.def.abilities.get(k), Some(crate::card::AbilityDef::WarpAbility { .. })) => {
                format!("Cast {} with Warp", name)
            }
            PlayKind::Land => text.unwrap_or_else(|| format!("Play {}", name)),
            _ => text.unwrap_or_default(),
        };
        // a short line for the client's popup (`PlayableObjectStats.load`)
        let mut text = text.replace("<br>", " ").replace('\n', " ");
        if text.encode_utf16().count() > 50 {
            let cut: Vec<u16> = text.encode_utf16().take(49).collect();
            text = String::from_utf16_lossy(&cut) + "...";
        }
        let group = match kind {
            PlayKind::BasicMana => "basicManaAbilities",
            PlayKind::Land => "basicPlayAbilities",
            PlayKind::Cast | PlayKind::Plotted | PlayKind::Foretold => "basicCastAbilities",
            PlayKind::Other | PlayKind::RoomHalf | PlayKind::Adventure => "other",
        };
        // Sunken Citadel's: the chosen color's symbol in its text
        if let (Some(crate::card::AbilityDef::ColoredManaAbility { color, .. }), true) =
            (o.def.abilities.get(k), text.contains("chosen color."))
        {
            text = text.replace("chosen color.", &format!("chosen color {{{}}}.", color_symbol(*color)));
        }
        let tag = if kind == PlayKind::RoomHalf { "h" } else { "a" };
        let record = json!({"id": format!("{}{}{}", r, tag, k), "value": text});
        let part = match kind {
            PlayKind::RoomHalf => Some(format!("{}h{}", r, k)),
            _ => face.map(|f| format!("{}f{}", r, f)),
        };
        for key in std::iter::once(r.clone()).chain(part) {
            let entry = objects.entry(key).or_insert_with(
                || json!({"basicManaAbilities": [], "basicPlayAbilities": [], "basicCastAbilities": [], "other": []}),
            );
            if let Some(Value::Array(list)) = entry.get_mut(group) {
                list.push(record.clone());
            }
        }
    }
    json!({ "objects": objects })
}

/// The exile zones (`GameView.exiles`): the main one, then one per source that exiled cards to its own (exiled until
/// it leaves, for it, its impulse draws), per prepared permanent and per player's adventures. Java keeps its zones in
/// a `HashMap` by random ids, so `compare` puts both sides' in one order and drops the empty ones (and the main zone
/// isn't always first).
fn exiles_view(game: &Game) -> Value {
    let mut zones: Vec<(String, Vec<ObjId>)> = vec![(String::new(), Vec::new())];
    for id in crate::replay::exile_order(game) {
        let o = game.obj(id);
        let linked =
            |list: &[(ObjId, u32, ObjId)]| list.iter().rev().find(|(_, _, c)| *c == id).map(|(s, z, _)| (*s, *z));
        let key = if let Some((p, z)) = o.prepared_for {
            format!("prepared {} {}", p.0, z)
        } else if o.on_adventure {
            format!("adventure {}", o.owner)
        } else if o.foretold_turn.is_some() {
            // `ForetellAbility` exiles each card into a zone of its own
            format!("foretell {}", id.0)
        } else if o.warp_from.is_some() {
            format!("warp {}", o.owner)
        } else if let Some((s, z)) =
            o.exiled_with.or_else(|| linked(&game.exiled_until_leaves)).or_else(|| linked(&game.exiled_for_source))
        {
            format!("source {} {}", s.0, z)
        } else {
            String::new()
        };
        match zones.iter_mut().find(|(k, _)| *k == key) {
            Some((_, ids)) => ids.push(id),
            None => zones.push((key, vec![id])),
        }
    }
    Value::Array(zones.iter().map(|(_, ids)| cards_view(game, ids)).collect())
}

/// Exile zones in one order to compare, by their cards, the empty ones left out (Java's order is its `HashMap`'s)
fn normalize_exiles(v: &mut Value) {
    if let Some(Value::Array(zones)) = v.get_mut("exiles") {
        zones.retain(|z| z.as_object().is_some_and(|m| !m.is_empty()));
        zones.sort_by_key(|z| {
            let mut keys: Vec<String> = z.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default();
            keys.sort();
            keys.join(",")
        });
    }
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

/// `GameView.revealed` (`RevealedView`): each reveal titled by its source's id name and zone change count
/// (`CardUtil.createObjectRelatedWindowTitle`), its cards with the zones they were revealed from
fn revealed_view(game: &Game) -> Vec<Value> {
    let stack = view_stack(game);
    game.revealed
        .iter()
        .map(|r| {
            let at =
                stack.iter().rev().position(|(i, _)| i.card == r.source && i.ability.is_none() && i.delayed.is_none());
            let source = match at {
                Some(i) => format!("S{}", i),
                None => reference(game, r.source),
            };
            // (Java's count is one more: a card starts at 1 in its library)
            let zcc = if r.paren { format!("({})", r.zcc + 1) } else { format!("[{}]", r.zcc + 1) };
            let mut cards = Map::new();
            for (c, zone) in &r.cards {
                let rf = reference(game, *c);
                let mut v = card_view(game, *c, &rf, true);
                v["zone"] = json!(zone_name(*zone));
                cards.insert(rf, v);
            }
            json!({"name": format!("{} [{}] {}", game.obj(r.source).def.name, source, zcc), "cards": cards})
        })
        .collect()
}

fn zone_name(z: Zone) -> &'static str {
    match z {
        Zone::Hand => "HAND",
        Zone::Library => "LIBRARY",
        Zone::Graveyard => "GRAVEYARD",
        Zone::Battlefield => "BATTLEFIELD",
        Zone::Stack => "STACK",
        Zone::Exile => "EXILED",
        Zone::Command => "COMMAND",
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
        "commandList": command_view(game, p),
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
        // (a permanent transformed in place has its front face for its card; one that entered transformed its back)
        let front = o.front.as_ref().filter(|f| f.transform_back.is_some() && !game_object && o.transformed_in_place);
        let name = front.map_or(&o.def.name, |f| &f.name);
        (o.zone == Zone::Battlefield).then(|| templates().get(&format!("{}:{}", kind, name)).cloned()).flatten()
    };
    // a prepared permanent's spell, its copy in exile
    let prepared = || o.prepared_for.and_then(|_| templates().get(&format!("prepared:{}", o.def.name)).cloned());
    let mut v = token_view(game, id)
        .or_else(prepared)
        .or_else(permanent)
        // a part cast on its own (a Room's half, an adventure's spell) as a spell
        .or_else(|| {
            (o.zone == Zone::Stack).then(|| templates().get(&format!("spell:{}", o.def.name)).cloned()).flatten()
        })
        .or_else(|| templates().get(&o.def.name).cloned())
        .or_else(|| if game_object { room_view(game, id) } else { room_card(game, id) })
        .unwrap_or_else(|| json!({"name": o.def.name}));
    place_ids(&mut v, r, &format!("P{}", o.owner), &format!("P{}", o.controller), game_object);
    if let Value::Object(m) = &mut v {
        m.insert("id".into(), json!(r));
        m.entry("cardId").or_insert_with(|| json!(r));
        let mut hints = if game_object { hint_lines(game, id) } else { Vec::new() };
        // `RegenerateSourceEffect` gives the permanent a hint of its shields for the turn
        if game_object && o.zone == Zone::Battlefield && o.regen_shields > 0 {
            hints.push(format!(
                "ICON_GOODRegeneration shields: {} (permanent will be regenerated instead of destroyed)",
                o.regen_shields
            ));
        }
        // a Room's halves: each half's rules get its hints, and a Room permanent's its unlocked doors'
        if game_object && o.zone == Zone::Battlefield {
            if let Some(full) = o.printed_def.as_ref().filter(|f| !f.room.is_empty()) {
                for (half, _) in full.room.iter().zip(o.doors).filter(|(_, u)| *u) {
                    let half_hints = hint_lines_of(game, id, half);
                    hints.extend(half_hints.into_iter().filter(|h| !hints.contains(h)).collect::<Vec<_>>());
                }
            }
        }
        if game_object {
            for (half, key) in o.def.room.iter().zip(["leftSplitRules", "rightSplitRules"]) {
                let half_hints = hint_lines_of(game, id, half);
                if let (false, Some(Value::Array(rules))) = (half_hints.is_empty(), m.get_mut(key)) {
                    rules.push(json!(HINT_START));
                    rules.extend(half_hints.iter().map(|h| json!(h)));
                }
            }
        }
        if let (true, Some(Value::Array(rules))) = (game_object, m.get_mut("rules")) {
            changed_rules(game, id, rules);
            // the ability Can't Stay Away gives what it returns
            if o.exile_instead_of_dying && o.zone == Zone::Battlefield {
                rules.push(json!("If {this} would die, exile it instead."));
            }
        }
        // an adventurer's creature half has the card's hints too
        if let (true, false, Some(Value::Array(rules))) =
            (o.def.adventure.is_some(), hints.is_empty(), m.get_mut("leftSplitRules"))
        {
            rules.push(json!(HINT_START));
            rules.extend(hints.iter().map(|h| json!(h)));
        }
        if let Some(Value::Array(rules)) = m.get_mut("rules") {
            if !hints.is_empty() {
                rules.push(json!(HINT_START));
                rules.extend(hints.into_iter().map(Value::String));
            }
            // a permanent's info comes after its hints, then its restrictions (`PermanentImpl.getRules`)
            if game_object && o.zone == Zone::Battlefield {
                rules.extend(o.info.iter().map(|(_, l)| json!(l)));
                let restricts = restriction_hints(game, id);
                if !restricts.is_empty() {
                    if !rules.iter().any(|r| r.as_str().is_some_and(|t| t.contains(HINT_START))) {
                        rules.push(json!(HINT_START));
                    }
                    rules.extend(restricts.into_iter().map(Value::String));
                }
            }
        }
        if game_object {
            face_hints(game, id, m);
            // a transformed permanent's card's other face is a card of its own, with an id the log doesn't place
            if let Some(Value::Object(f)) =
                m.get_mut("secondCardFace").and_then(|f| f.get_mut("secondCardFace")).filter(|f| f.is_object())
            {
                if o.transformed_in_place && f.get("id").and_then(|i| i.as_str()) == Some(r) {
                    f.insert("id".into(), json!("?"));
                }
            }
        }
        if game_object && o.zone != Zone::Battlefield {
            land_frame(game, id, m);
            cda_power(game, id, m);
        }
        if game_object && o.zone == Zone::Battlefield {
            characteristics(game, id, m);
            // a transforming card's front as a permanent has the whole card's color identity
            if o.def.transform_back.is_some() {
                if let Some(c) = templates().get(&o.def.name).and_then(|t| t.get("originalColorIdentity")) {
                    m.insert("originalColorIdentity".into(), c.clone());
                }
            }
            // a face shown again inside its other face (a Pathway's) is the permanent: its subtypes as they are
            let (name, subtypes) = (m.get("name").cloned(), m.get("subTypes").cloned());
            if let Some(Value::Object(f)) =
                m.get_mut("secondCardFace").and_then(|f| f.get_mut("secondCardFace")).filter(|f| f.is_object())
            {
                if f.get("name") == name.as_ref() && f.get("id").and_then(|i| i.as_str()) == Some(r) {
                    if let Some(st) = subtypes {
                        f.insert("subTypes".into(), st);
                    }
                }
            }
            permanent_state(game, id, m);
            let mut icons = permanent_icons(game, id);
            if spell_x(&o.def) {
                icons.push(x_icon(o.cast_x));
            }
            // (`CardView`: its restriction and requirement hints, sorted, as one icon)
            let mut restricts: Vec<String> = restriction_hints(game, id)
                .iter()
                .map(|r| r.replace("ICON_RESTRICT", "").replace("ICON_REQUIRE", "").trim().to_string())
                .collect();
            if !restricts.is_empty() {
                restricts.sort();
                icons.push(icon("OTHER_HAS_RESTRICTIONS", &restricts.join("<br>"), ""));
            }
            m.insert("cardIcons".into(), Value::Array(icons));
        }
    }
    v
}

/// A characteristic-defining power (Adeline's) applies in every zone (`CardView` of a card in a game)
fn cda_power(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    use crate::card::{AbilityDef as A, EffectDef as E};
    let o = game.obj(id);
    let amount = o.def.abilities.iter().find_map(|a| match a {
        A::SimpleStaticAbility { effects, .. } => effects.iter().find_map(|e| match e {
            E::SetBasePowerSourceEffect { amount } => Some(amount),
            _ => None,
        }),
        _ => None,
    });
    // and both (Crusader of Odric in hand)
    if cda_counts_creatures(&o.def) {
        set_pt(m, creatures_of(game, o.controller));
    }
    let Some(amount) = amount else { return };
    let n = game.value(amount, o.controller, id, 0);
    m.insert("power".into(), json!(n.to_string()));
    if let Some(Value::Object(x)) = m.get_mut("originalPower") {
        x.insert("modifiedBaseValue".into(), json!(n));
        x.insert("boostedValue".into(), json!(n));
        x.insert("cardValue".into(), json!(n.to_string()));
    }
}

/// A power and toughness equal to the creatures you control (`SetBasePowerToughnessSourceEffect`)
fn cda_counts_creatures(def: &CardDef) -> bool {
    use crate::card::{AbilityDef as A, EffectDef as E};
    def.abilities.iter().any(|a| match a {
        A::SimpleStaticAbility { effects, .. } => effects
            .iter()
            .any(|e| matches!(e, E::SetBasePowerToughnessSourceEffect { count } if count == "creatures_you_control")),
        _ => false,
    })
}

fn creatures_of(game: &Game, you: Seat) -> i32 {
    game.battlefield.iter().filter(|b| game.obj(**b).controller == you && game.obj(**b).is(CardType::Creature)).count()
        as i32
}

/// A card's power and toughness set to `n`, before boosts
fn set_pt(m: &mut Map<String, Value>, n: i32) {
    m.insert("power".into(), json!(n.to_string()));
    m.insert("toughness".into(), json!(n.to_string()));
    for k in ["originalPower", "originalToughness"] {
        if let Some(Value::Object(x)) = m.get_mut(k) {
            x.insert("boostedValue".into(), json!(n));
            x.insert("modifiedBaseValue".into(), json!(n));
            x.insert("cardValue".into(), json!(n.to_string()));
        }
    }
}

/// What a card's view of a permanent shows of its state (`CardView(permanent)`): its power and toughness, its
/// counters, loyalty and defense
fn permanent_state(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    let o = game.obj(id);
    // (a Vehicle's too while it isn't a creature: `CardView` shows any permanent's)
    if game.obj(id).zone == Zone::Battlefield
        && (o.def.types.contains(&CardType::Creature)
            || game.has_type(id, CardType::Creature)
            || o.def.subtypes.iter().any(|t| t == "Vehicle"))
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
    // a static `AddCardSubTypeSourceEffect` (Skyknight Squire's Knight while it has three counters)
    let mut added: Vec<&str> = Vec::new();
    for a in &o.def.abilities {
        if let crate::card::AbilityDef::SimpleStaticAbility { effects, .. } = a {
            for e in effects {
                let (e, holds) = match e {
                    crate::card::EffectDef::ConditionalContinuousEffect { effect, condition } => {
                        (&**effect, game.condition_holds(Some(condition), o.controller, id))
                    }
                    e => (e, true),
                };
                if let (crate::card::EffectDef::AddCardSubTypeSourceEffect { subtype }, true) = (e, holds) {
                    if !subtype.is_empty() {
                        added.push(subtype);
                    }
                }
            }
        }
    }
    if let (false, Some(Value::Array(subtypes))) = (added.is_empty(), m.get_mut("subTypes")) {
        for t in added {
            let t = json!(subtype_name(t));
            if !subtypes.contains(&t) {
                subtypes.push(t);
            }
        }
    }
    // Evolved Sleeper's "becomes a Human Cleric": `AddCardSubTypeSourceEffect` adds the ones it hasn't
    if let (Some(types), Some(Value::Array(subtypes))) = (&o.creature_types, m.get_mut("subTypes")) {
        for t in types {
            let t = json!(subtype_name(t));
            if !subtypes.contains(&t) {
                subtypes.push(t);
            }
        }
    }
    let rules: Vec<String> = m
        .get("rules")
        .and_then(|r| r.as_array())
        .map(|r| r.iter().filter_map(|x| x.as_str()).take_while(|x| *x != HINT_START).map(String::from).collect())
        .unwrap_or_default();
    land_frame(game, id, m);
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

fn color_symbol(c: crate::card::Color) -> char {
    use crate::card::Color as C;
    match c {
        C::W => 'W',
        C::U => 'U',
        C::B => 'B',
        C::R => 'R',
        C::G => 'G',
    }
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

/// A land's frame: the colors of mana it can make (`MageObjectImpl.getFrameColor`, in any zone while the game is on)
fn land_frame(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    let o = game.obj(id);
    let five = |has: &dyn Fn(usize) -> bool| -> Value {
        const NAMES: [&str; 5] = ["white", "blue", "black", "red", "green"];
        Value::Object(NAMES.iter().enumerate().map(|(i, n)| (n.to_string(), json!(has(i)))).collect())
    };
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
        // (`getProducableManaTypes`: a chosen color's, and Nykthos's colors with devotion)
        let left = matches!(o.zone, Zone::Graveyard | Zone::Library | Zone::Hand | Zone::Exile);
        let def = if left { printed(&o.def.name).unwrap_or_else(|| o.def.clone()) } else { o.def.clone() };
        for a in &def.abilities {
            match a {
                crate::card::AbilityDef::ColoredManaAbility { color, .. } => makes[color.index()] = true,
                crate::card::AbilityDef::NykthosShrineToNyxManaAbility { .. } => {
                    for c in crate::card::Color::ALL {
                        makes[c.index()] |= crate::rules::devotion(game, o.controller, c) > 0;
                    }
                }
                _ => {}
            }
        }
        m.insert("frameColor".into(), five(&|i| makes[i]));
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
    // (a card that left the battlefield is its printed self: what effects gave its permanent is gone)
    if matches!(o.zone, Zone::Graveyard | Zone::Library | Zone::Hand | Zone::Exile)
        && o.def.room.is_empty()
        && printed(&o.def.name).is_some_and(|p| !Arc::ptr_eq(&p, &o.def))
    {
        return;
    }
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

/// A double-faced card's other faces' rules get their own hints (`CardView` of each face)
fn face_hints(game: &Game, id: ObjId, m: &mut Map<String, Value>) {
    let o = game.obj(id);
    let defs: Vec<&Arc<CardDef>> = [
        Some(&o.def),
        o.def.back.as_ref(),
        o.def.transform_back.as_ref(),
        o.front.as_ref(),
        o.front.as_ref().and_then(|f| f.back.as_ref()),
        o.front.as_ref().and_then(|f| f.transform_back.as_ref()),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut face = m.get_mut("secondCardFace").filter(|f| f.is_object());
    while let Some(Value::Object(f)) = face {
        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        if let (Some(def), Some(Value::Array(rules))) = (defs.iter().find(|d| d.name == name), f.get_mut("rules")) {
            let hints = hint_lines_of(game, id, def);
            if !hints.is_empty() && !rules.iter().any(|r| r.as_str() == Some(HINT_START)) {
                rules.push(json!(HINT_START));
                rules.extend(hints.into_iter().map(Value::String));
            }
        }
        face = f.get_mut("secondCardFace").filter(|f| f.is_object());
    }
}

/// A Room permanent's card (`PermanentView.original`): the whole Room's
fn room_card(game: &Game, id: ObjId) -> Option<Value> {
    let full = game.obj(id).printed_def.as_ref().filter(|f| !f.room.is_empty())?;
    templates().get(&full.name).cloned()
}

/// A Room permanent with one door unlocked (`PermanentCard` of a `RoomCard`): the card's view as the unlocked
/// half, its rules the room's, the unlocked half's and the locked half's unlock ability
fn room_view(game: &Game, id: ObjId) -> Option<Value> {
    let o = game.obj(id);
    let full = o.printed_def.as_ref().filter(|f| f.room.len() == 2 && o.zone == Zone::Battlefield)?;
    let mut v = templates().get(&full.name)?.clone();
    let m = v.as_object_mut()?;
    let rules = |k: &str| m.get(k).and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let mut out = rules("rules");
    let mut costs = Vec::new();
    let mut unlocks = Vec::new();
    for (k, side) in ["left", "right"].iter().enumerate() {
        if o.doors[k] {
            out.extend(rules(&format!("{}SplitRules", side)));
            let cost = if k == 0 { "manaCostLeftStr" } else { "manaCostRightStr" };
            costs.extend(rules(cost));
        } else {
            let cost = m.get(&format!("{}SplitCostsStr", side)).and_then(|c| c.as_str()).unwrap_or("");
            unlocks.push(json!(format!(
                "{}: Unlock the {} half. <i>(Activate only as a sorcery, and only if the {} half is locked.)</i>",
                cost, side, side
            )));
        }
    }
    out.extend(unlocks);
    for k in ["Name", "CostsStr", "Rules", "TypeLine"] {
        m.remove(&format!("leftSplit{}", k));
        m.remove(&format!("rightSplit{}", k));
    }
    for k in ["name", "displayName", "displayFullName"] {
        m.insert(k.into(), json!(o.def.name));
    }
    m.insert("rules".into(), Value::Array(out));
    m.insert("manaValue".into(), json!(o.def.cost.mana_value()));
    m.insert("manaCostLeftStr".into(), Value::Array(costs));
    m.insert("manaCostRightStr".into(), json!([]));
    m.insert("isSplitCard".into(), json!(false));
    m.insert("rotate".into(), json!(false));
    Some(v)
}

/// Where a card's hints start in its rules (`HintUtils.HINT_START_MARK`)
const HINT_START: &str = "<br/><hintstart/>";

/// The hints of a card's abilities in a started game, each once (`CardUtil.getCardRulesWithAdditionalInfo`)
fn hint_lines(game: &Game, id: ObjId) -> Vec<String> {
    hint_lines_of(game, id, &game.obj(id).def)
}

/// The hints of one of its faces' abilities
fn hint_lines_of(game: &Game, id: ObjId, def: &CardDef) -> Vec<String> {
    hint_lines_for(game, id, def, None)
}

/// The hints of its abilities, or of its ability `k` alone (`StackAbilityView`)
fn hint_lines_for(game: &Game, id: ObjId, def: &CardDef, k: Option<usize>) -> Vec<String> {
    if game.step.is_none() {
        return Vec::new();
    }
    let o = game.obj(id);
    let you = o.controller;
    let mut out: Vec<String> = Vec::new();
    // (a back face's own hints aren't in its facts: daybound and nightbound add the day's)
    let day_night = |a: &crate::card::AbilityDef| {
        matches!(a, crate::card::AbilityDef::DayboundAbility | crate::card::AbilityDef::NightboundAbility)
    };
    if k.is_none() && def.hints.is_empty() && def.abilities.iter().any(day_night) {
        return vec![day_night_hint(game)];
    }
    for (i, h) in def.hints.iter().enumerate() {
        if k.is_some() && def.hint_abilities.get(i).copied() != k {
            continue;
        }
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
        "OpenSideboardHint" => format!(
            "Cards in your sideboard: {} (<i>Right click on battlefield to open player's sideboard at any time</i>)",
            game.player(you).sideboard
        ),
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
    // the groups in the order they attacked; any attacker not in one after them
    let mut order: Vec<&crate::state::CombatGroup> = game.combat_groups.iter().collect();
    let extra: Vec<crate::state::CombatGroup> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.obj(*id).attacking.is_some()
                && !game.combat_groups.iter().any(|g| g.attacker == *id && g.left.is_none())
        })
        .map(|attacker| crate::state::CombatGroup { attacker, left: None })
        .collect();
    order.extend(extra.iter());
    let mut groups = Vec::new();
    for g in order {
        let id = g.attacker;
        let o = game.obj(id);
        let (defender, pw, blocked, blockers, here) = match &g.left {
            Some((d, pw, b, bl)) => (*d, *pw, *b, bl.clone(), false),
            None => {
                let Some(d) = o.attacking.filter(|_| o.zone == Zone::Battlefield) else { continue };
                (d, o.attacking_pw.map(|p| p.0), o.blocked, o.blocked_by.clone(), true)
            }
        };
        let (defender_id, defender_name) = match pw {
            Some(pw) if game.obj(pw).zone == Zone::Battlefield => (reference(game, pw), game.obj(pw).def.name.clone()),
            _ => (format!("P{}", defender), names.get(defender).copied().unwrap_or("").to_string()),
        };
        let mut attackers = Map::new();
        if here {
            let r = reference(game, id);
            attackers.insert(r.clone(), permanent_view(game, id, &r, nobody, names));
        }
        let mut blockers_view = Map::new();
        for b in blockers.iter().copied().filter(|b| game.obj(*b).zone == Zone::Battlefield) {
            let r = reference(game, b);
            blockers_view.insert(r.clone(), permanent_view(game, b, &r, nobody, names));
        }
        groups.push(json!({
            "attackers": attackers,
            "blockers": blockers_view,
            "isBlocked": blocked,
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
    for (i, (item, at)) in view_stack(game).into_iter().rev().enumerate() {
        let r = format!("S{}", i);
        // a step's trigger with no targets points at the step's player (`AtStepTriggeredAbility`)
        let pointed;
        let item = match (item.ability.and_then(|(k, _)| game.item_def(item).abilities.get(k)), game.active) {
            (Some(a), Some(p))
                if item.delayed.is_none()
                    && item.targets.iter().all(|t| t.is_empty())
                    && a.triggered().is_some_and(|t| t.2.is_empty())
                    && matches!(a.trigger(), Some(crate::card::Trigger::StepBegins { .. })) =>
            {
                let mut c = item.clone();
                c.targets = vec![vec![crate::state::Target::Player(p)]];
                pointed = c;
                &pointed
            }
            // warp's end-step exile points at the permanent (`FixedTarget`) while it's still that object
            _ if item.delayed.as_ref().is_some_and(|d| !d.warp_exile.is_empty()) && item.targets.is_empty() => {
                let mut c = item.clone();
                let objs = item.delayed.as_ref().map_or(Vec::new(), |d| {
                    d.warp_exile
                        .iter()
                        .filter(|(o, z)| game.obj(*o).zcc == *z)
                        .map(|(o, _)| crate::state::Target::Obj(*o))
                        .collect::<Vec<_>>()
                });
                c.targets = vec![objs];
                pointed = c;
                &pointed
            }
            _ => item,
        };
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
        let paid = at.is_none() || game.unpaid != at || game.mana_paid == at;
        let casting = at.is_some() && game.unpaid == at;
        let mut icons = Vec::new();
        let def = game.item_def(item);
        let spell = item.ability.is_none() && item.delayed.is_none();
        let ability = item.ability.and_then(|(k, _)| def.abilities.get(k));
        // the targets it declares, chosen or not (`CardView.targets`, set when its ability has targets)
        let declared = !item.targets.is_empty()
            || if spell {
                !def.spell.targets.is_empty()
                    || item.modes.iter().any(|k| def.spell.modes.get(*k).is_some_and(|m| !m.targets.is_empty()))
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
                // a transforming card's spell is its front face (`Spell` of the face card)
                if o.def.transform_back.is_some() {
                    m.insert("displayFullName".into(), json!(o.def.name));
                    m.insert("isDoubleFacedCard".into(), json!(false));
                    if let Some(c) =
                        templates().get(&format!("face:{}", o.def.name)).and_then(|f| f.get("originalColorIdentity"))
                    {
                        m.insert("originalColorIdentity".into(), c.clone());
                    }
                    if let Some(Value::Object(f)) = m.get_mut("secondCardFace") {
                        f.insert("id".into(), json!("?"));
                        f.insert("cardId".into(), json!("?"));
                    }
                }
                m.insert("paid".into(), json!(paid));
                m.insert("cardIcons".into(), Value::Array(icons));
                if let Some(c) = m.get("color").cloned() {
                    m.insert("frameColor".into(), c);
                }
                if let Some(t) = targets {
                    m.insert("targets".into(), Value::Array(t));
                }
                if let Some(Value::Array(rules)) = m.get_mut("rules") {
                    let k = def.abilities.iter().position(|a| matches!(a, crate::card::AbilityDef::SpellAbility));
                    rules.extend(chosen_modes(game, item, k, casting).into_iter().map(Value::String));
                    rules.extend(targets_on_stack(game, item).into_iter().map(Value::String));
                }
            }
            v
        } else {
            let mut v: Value = serde_json::from_str(STACK_ABILITY).unwrap_or_default();
            let def = game.item_def(item);
            let ability = item.ability.and_then(|(k, _)| def.abilities.get(k));
            let rule = item_rule(game, item).replace("{this}", &o.def.name);
            let triggered = item.delayed.is_some() || ability.is_some_and(|a| a.trigger().is_some());
            let kind = if o.token { "ABILITY_STACK_FROM_TOKEN" } else { "ABILITY_STACK_FROM_CARD" };
            // (the decision log places a spell on the stack, not an ability's source; a token gone is nowhere)
            let source_ref = match reference(game, item.card) {
                r if r.starts_with('S') || r.starts_with('?') => {
                    let spell = game
                        .stack
                        .iter()
                        .rev()
                        .position(|t| t.card == item.card && t.ability.is_none() && t.delayed.is_none());
                    let exiled = crate::replay::exile_order(game).iter().position(|x| *x == item.card);
                    match (spell, exiled) {
                        (Some(i), _) => format!("S{}", i),
                        (None, Some(i)) => format!("E{}", i),
                        _ => "?".to_string(),
                    }
                }
                r => r,
            };
            let mut source = card_view(game, item.card, &source_ref, true);
            source["mageObjectType"] = json!(kind);
            // (a permanent's card view: `CardView(Permanent)`)
            // (a permanent's, or its last known one's: a token gone)
            if o.zone == Zone::Battlefield || o.token && source_ref == "?" {
                source["bandedCards"] = json!([]);
                source["controlledByOwner"] = json!(o.owner == o.controller);
                if o.zone == Zone::Battlefield {
                    if let Value::Object(m) = &mut source {
                        permanent_info(game, item.card, names, m);
                    }
                }
            }
            v["id"] = json!(r);
            let mut rules = vec![rule];
            // what its effects point at when it doesn't target (`StackAbilityView.updateTargets`)
            let own_targets = ability.is_some_and(|a| {
                a.triggered().is_some_and(|t| !t.2.is_empty()) || a.activated().is_some_and(|t| !t.2.is_empty())
            });
            let mut related: Vec<String> = Vec::new();
            if !own_targets {
                for t in item.targets.iter().flatten() {
                    if let crate::state::Target::Obj(id) = t {
                        let name = format!(
                            "<font color='{}'>{} [{}]</font>",
                            tooltip_color(&game.obj(*id).def.colors),
                            game.obj(*id).def.name,
                            stack_ref(game, *id)
                        );
                        if !related.contains(&name) {
                            related.push(name);
                        }
                    }
                }
            }
            if !related.is_empty() {
                rules.push(format!("<i>Related objects: [{}]</i>", related.join(", ")));
            }
            rules.extend(chosen_modes(game, item, item.ability.map(|(k, _)| k), casting));
            if let Some((k, _)) = item.ability.filter(|_| item.delayed.is_none()) {
                let hints = match room_half_ability(game, item.card, k) {
                    Some((half, local)) => hint_lines_for(game, item.card, &half, Some(local)),
                    None => hint_lines_for(game, item.card, def, Some(k)),
                };
                if !hints.is_empty() {
                    rules.push(HINT_START.to_string());
                    rules.extend(hints);
                }
            }
            if related.is_empty() {
                rules.extend(targets_on_stack(game, item));
            }
            v["rules"] = json!(rules);
            v["mageObjectType"] = json!(kind);
            v["abilityType"] = json!(if triggered { "TRIGGERED_NONMANA" } else { "ACTIVATED_NONMANA" });
            if let Some(c) = source.get("counters").cloned() {
                v["counters"] = c;
            }
            // (`GameView.checkPaid`: an ability with no mana to pay is paid)
            let mana = ability.and_then(|a| a.activated()).is_some_and(|t| {
                t.3.iter().any(|c| matches!(c, crate::card::CostDef::ManaCost { cost } if !cost.is_empty()))
            });
            v["paid"] = json!(paid || !mana);
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

/// The restriction hints of a permanent (`PermanentImpl.getRules`): what its restriction effects stop it from
/// doing, with their source, sorted
fn restriction_hints(game: &Game, id: ObjId) -> Vec<String> {
    use crate::card::{AbilityDef as A, EffectDef as E};
    if game.step.is_none() {
        return Vec::new();
    }
    let id_name = |src: ObjId| format!("{} [{}]", game.obj(src).def.name, reference(game, src));
    let mut out = Vec::new();
    let o = game.obj(id);
    // `BlocksIfAbleSourceEffect`'s requirement, from what gave it
    for (_, _, src) in game.must_block_from.iter().filter(|(t, z, _)| *t == id && *z == o.zcc) {
        out.push(format!("ICON_REQUIREMust block any ({} [{}])", game.obj(*src).def.name, stack_ref(game, *src)));
    }
    if game.cant_attack_alone(id) {
        out.push(format!("ICON_RESTRICTCan't attack ({})", id_name(id)));
    }
    for a in &o.def.abilities {
        if matches!(a, A::CantBlockAbility) {
            out.push(format!("ICON_RESTRICTCan't block ({})", id_name(id)));
        }
        if let A::SimpleStaticAbility { effects, .. } = a {
            for e in effects {
                if let E::CantAttackBlockUnlessConditionSourceEffect { condition } = e {
                    if !game.condition_holds(Some(condition), o.controller, id) {
                        out.push(format!("ICON_RESTRICTCan't attack ({})", id_name(id)));
                        out.push(format!("ICON_RESTRICTCan't block ({})", id_name(id)));
                    }
                }
            }
        }
    }
    for src in game.battlefield.iter().copied().filter(|s| game.attached(*s) == Some(id)) {
        for a in &game.obj(src).def.abilities {
            if let A::SimpleStaticAbility { effects, .. } = a {
                if effects.iter().any(|e| matches!(e, E::CantAttackBlockAttachedEffect)) {
                    out.push(format!("ICON_RESTRICTCan't attack ({})", id_name(src)));
                    out.push(format!("ICON_RESTRICTCan't block ({})", id_name(src)));
                }
            }
        }
    }
    out.sort();
    out
}

/// `GameLog.getTooltipColorName`
fn tooltip_color(colors: &[crate::card::Color]) -> &'static str {
    use crate::card::Color as C;
    match colors {
        [] => "#94A4BA",
        [_, _, ..] => "#A97A00",
        [C::R] => "Red",
        [C::G] => "Green",
        [C::U] => "Blue",
        [C::W] => "#FDFFE6",
        [C::B] => "Black",
    }
}

/// A modal spell's or ability's chosen modes (`CardView(Spell)`, `StackAbilityView`), by its ability's index `k`
/// in its card's abilities
fn chosen_modes(game: &Game, item: &crate::state::StackItem, k: Option<usize>, casting: bool) -> Vec<String> {
    let o = game.obj(item.card);
    let name = o.printed_def.as_ref().map_or(&o.def.name, |p| &p.name);
    let Some(texts) = k.and_then(|k| templates().get(&format!("modes:{}", name))?.get(k.to_string())?.as_array())
    else {
        return Vec::new();
    };
    // (a spree spell's modes are chosen with its costs: none before; `Modes.choose` clears them while it asks)
    if casting && item.modes.is_empty() {
        return Vec::new();
    }
    if game.item_def(item).spell.modes.iter().any(|m| m.cost.is_some())
        && item.ability.is_none()
        && item.modes.is_empty()
    {
        return Vec::new();
    }
    let chosen: Vec<usize> = if item.modes.is_empty() { vec![0] } else { item.modes.clone() };
    chosen
        .iter()
        .filter_map(|m| texts.get(*m).and_then(|t| t.as_str()))
        .map(|t| format!("<span color='green'><i>Chosen mode: {}</i></span>", t))
        .collect()
}

/// A room permanent's ability `k` (`rules::room_face`: the unlocked halves' abilities but their spells, after the
/// card's own) as its half's and its index there
fn room_half_ability(game: &Game, id: ObjId, k: usize) -> Option<(Arc<CardDef>, usize)> {
    let o = game.obj(id);
    let full = o.printed_def.as_ref().filter(|f| f.room.len() == 2)?;
    let mut at = full.abilities.len();
    for (h, _) in full.room.iter().zip(o.doors).filter(|(_, u)| *u) {
        for (i, a) in h.abilities.iter().enumerate() {
            if matches!(a, crate::card::AbilityDef::SpellAbility) {
                continue;
            }
            if at == k {
                return Some((h.clone(), i));
            }
            at += 1;
        }
    }
    None
}

/// `Target on stack: Name [id]` for its targets once one is a spell (Java names that spell for each target after)
fn targets_on_stack(game: &Game, item: &crate::state::StackItem) -> Vec<String> {
    let mut found: Option<String> = None;
    let mut out = Vec::new();
    for t in item.targets.iter().flatten() {
        if let crate::state::Target::Obj(id) = t {
            if game.obj(*id).zone == Zone::Stack {
                found = Some(format!("{} [{}]", game.obj(*id).def.name, stack_ref(game, *id)));
            }
        }
        if let Some(f) = &found {
            out.push(format!("<span color='green'><i>Target on stack: {}", f));
        }
    }
    out
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
        // `CrewAbility`'s icon
        a => {
            let crew = a.activated().and_then(|t| {
                t.3.iter().find_map(|c| match c {
                    crate::card::CostDef::CrewCost { value } => Some(*value),
                    _ => None,
                })
            })?;
            return Some(icon("ABILITY_CREW", &format!("Crew {}", crew), ""));
        }
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
        // (an ability its own effect already put in its face, Soulstone Sanctuary's vigilance, once)
        .chain(game.granted_abilities(id).into_iter().filter(|a| !o.def.abilities.contains(a)))
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
/// The stack as Java's views show it (`GameImpl.resolve` removes an object after it resolves): with the object
/// resolving where it was, from the bottom, and its index on the stack if it's there (None: the one resolving)
fn view_stack(game: &Game) -> Vec<(&crate::state::StackItem, Option<usize>)> {
    let mut items: Vec<(&crate::state::StackItem, Option<usize>)> =
        game.stack.iter().enumerate().map(|(k, i)| (i, Some(k))).collect();
    if let (Some(item), Some((at, _))) = (&game.resolving_item, game.resolving) {
        let there = game.stack.get(at).is_some_and(|i| i.card == item.card && i.ability == item.ability);
        if !there {
            items.insert(at.min(items.len()), (item, None));
        }
    }
    items
}

/// A stack ability's rule, `{this}` for its source's name
pub(crate) fn item_rule(game: &Game, item: &crate::state::StackItem) -> String {
    let def = game.item_def(item);
    let ability = item.ability.and_then(|(k, _)| def.abilities.get(k));
    match (&item.delayed, item.ability) {
        (Some(d), _) => d.rule.clone(),
        // its own rule, or its card's at its index when it has none (an effect's def can reorder them)
        (None, Some((k, _))) => ability
            .and_then(|a| a.triggered().map(|t| t.0).or_else(|| a.activated().map(|t| t.0)))
            .filter(|r| !r.is_empty())
            .map(str::to_string)
            .or_else(|| ability_rules(game, item.card).and_then(|r| r.get(k).cloned()))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub(crate) fn stack_ref(game: &Game, id: ObjId) -> String {
    match view_stack(game)
        .into_iter()
        .map(|(i, _)| i)
        .rev()
        .position(|t| t.card == id && t.ability.is_none() && t.delayed.is_none())
    {
        Some(i) if game.obj(id).zone == Zone::Stack => format!("S{}", i),
        _ => reference(game, id),
    }
}

/// `PlayerView.commandList`: the emblems seat `p` controls (`EmblemView`), `M` and their index in the command zone
fn command_view(game: &Game, p: Seat) -> Vec<Value> {
    game.command
        .iter()
        .enumerate()
        .filter(|(_, c)| game.obj(**c).controller == p)
        .map(|(i, c)| {
            let o = game.obj(*c);
            let rules: Vec<String> = o
                .def
                .abilities
                .iter()
                .filter_map(ability_rule)
                .map(|r| {
                    let mut ch = r.chars();
                    ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
                })
                .collect();
            json!({
                "id": format!("M{}", i),
                "name": o.def.name,
                "cardNumber": "",
                "imageFileName": "",
                "imageNumber": 0,
                "usesVariousArt": false,
                "expansionSetCode": o.token_image.as_ref().map(|t| t.1.as_str()).unwrap_or(""),
                "rules": rules,
                "playableStats": {"basicManaAbilities": [], "basicPlayAbilities": [], "basicCastAbilities": [], "other": []},
            })
        })
        .collect()
}

/// An object's abilities' rules, in order (from its card's or token's facts)
pub(crate) fn ability_rules(game: &Game, id: ObjId) -> Option<Vec<String>> {
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
        // a token made with its size (`CustomIllusionToken(x)`): its class's view is 0/0
        let base = o.printed_def.as_ref().unwrap_or(&o.def);
        if base.types.contains(&CardType::Creature) && m.get("power") == Some(&json!("0")) && base.power != 0 {
            m.insert("power".into(), json!(base.power.to_string()));
            m.insert("toughness".into(), json!(base.toughness.to_string()));
            for (k, n) in [("originalPower", base.power), ("originalToughness", base.toughness)] {
                if let Some(Value::Object(x)) = m.get_mut(k) {
                    x.insert("baseValue".into(), json!(n));
                }
            }
        }
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
        // an attachment shows with what it's attached to (`PlayerView.showInBattlefield`)
        let shown_by = match game.attached(id).filter(|t| game.obj(*t).zone == Zone::Battlefield) {
            Some(t) => game.obj(t).controller,
            None => game.obj(id).controller,
        };
        if shown_by == p {
            let r = reference(game, id);
            m.insert(r.clone(), permanent_view(game, id, &r, viewer, names));
        }
    }
    Value::Object(m)
}

/// A permanent's info lines after the chosen color's, in the order of Java's `HashMap` of them: the protector,
/// prepared
fn permanent_info(game: &Game, id: ObjId, names: &[&str], m: &mut Map<String, Value>) {
    let o = game.obj(id);
    if let Some(Value::Array(rules)) = m.get_mut("rules") {
        if let Some(n) = &o.chosen_name {
            // `ChooseACardNameEffect`
            rules.push(json!(format!("<font color = 'blue'>Chosen name: {n}</font>")));
        }
        if let Some(p) = o.protector {
            let name = names.get(p).copied().unwrap_or("");
            rules.push(json!(format!("Protected by <font color='#20B2AA'>{}</font>", name)));
        }
        if o.prepared_copy.is_some() {
            rules.push(json!("<font color = 'blue'>Prepared</font>"));
        }
    }
}

/// `PermanentView`: the card's view as a permanent, with its state, and (for its controller, or a token) the card
/// it was printed as
fn permanent_view(game: &Game, id: ObjId, r: &str, viewer: Seat, names: &[&str]) -> Value {
    let o = game.obj(id);
    let mut v = card_view(game, id, r, true);
    let controlled = o.controller == viewer;
    let Value::Object(m) = &mut v else { return v };
    // its info lines after the chosen color's, in the order of Java's `HashMap` of them: the protector, prepared
    permanent_info(game, id, names, m);
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
    let mut attached: Vec<ObjId> = game.battlefield.iter().copied().filter(|a| game.attached(*a) == Some(id)).collect();
    attached.sort_by_key(|a| game.obj(*a).attached_at);
    let attachments: Vec<Value> = attached.iter().map(|a| json!(reference(game, *a))).collect();
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
    } else if o.token {
        // a token copy of a card (`CreateTokenCopyTargetEffect`): the card's view as a token, its original the
        // token as made
        for (k, x) in [("isToken", json!(true)), ("rarity", json!("SPECIAL")), ("originalIsCopy", json!(true))] {
            m.insert(k.into(), x);
        }
        m.insert("mageObjectType".into(), json!("TOKEN"));
        let mut original = card_view(game, id, r, false);
        if let Value::Object(x) = &mut original {
            for k in
                ["originalPower", "originalToughness", "originalColorIdentity", "ownerId", "controllerId", "cardId"]
            {
                x.remove(k);
            }
            for (k, v) in [
                ("id", json!("?")),
                ("isToken", json!(true)),
                ("mageObjectType", json!("NULL")),
                ("rarity", json!("SPECIAL")),
                ("usesVariousArt", json!(false)),
                ("manaValue", json!(0)),
            ] {
                x.insert(k.into(), v);
            }
        }
        m.insert("original".into(), original);
    } else if let (Some(printed), true) = (o.printed_def.as_ref().filter(|_| o.copied_def.is_some()), controlled) {
        // entered as a copy (Mockingbird): its card is the printed one
        let mut original = templates().get(&printed.name).cloned().unwrap_or_else(|| json!({"name": printed.name}));
        place_ids(&mut original, r, &format!("P{}", o.owner), &format!("P{}", o.controller), false);
        if let Value::Object(x) = &mut original {
            x.insert("id".into(), json!(r));
        }
        m.insert("original".into(), original);
    } else if controlled {
        let mut original = card_view(game, id, r, false);
        // (its card counts a characteristic-defining power too, as last counted before it entered: Java doesn't
        // count it again on the battlefield, and the others are as they were unless one came or went since)
        if let (Value::Object(x), true) = (&mut original, cda_counts_creatures(&o.def)) {
            let n = creatures_of(game, o.controller) - 1;
            set_pt(x, n);
        }
        m.insert("original".into(), original);
    }
    if let Some(printed) = o.printed_def.as_ref().filter(|_| o.copied_def.is_some() && !o.token) {
        // `PermanentImpl` copying another: a copy, its base power and toughness its card's
        m.insert("originalIsCopy".into(), json!(true));
        for (k, n) in [("originalPower", printed.power), ("originalToughness", printed.toughness)] {
            if let Some(Value::Object(x)) = m.get_mut(k) {
                x.insert("baseValue".into(), json!(n));
            }
        }
    }
    // a permanent whose card has another name gets that name too
    let original_name = m.get("original").and_then(|x| x.get("name")).cloned();
    if let Some(n) = original_name.filter(|n| Some(n) != m.get("name") && !m.contains_key("alternateName")) {
        m.insert("alternateName".into(), n);
    }
    m.insert("copy".into(), json!((o.token && o.token_image.is_none()) || (!o.token && o.copied_def.is_some())));
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
    if let Some(full) = o.printed_def.as_ref().filter(|f| f.room.len() == 2) {
        m.insert("roomLeftDoor".into(), json!(full.room[0].name));
        m.insert("roomRightDoor".into(), json!(full.room[1].name));
    }
    m.insert("roomLeftUnlocked".into(), json!(o.doors[0]));
    m.insert("roomRightUnlocked".into(), json!(o.doors[1]));
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
    ("wins", "the match's score, which the host passes through"),
    ("winsNeeded", "the match's score, which the host passes through"),
    (
        "sideboard",
        "the deck's sideboard: the decision log doesn't place its cards (every id is `?`), and play doesn't change it",
    ),
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
    let (mut java, mut rust) = (java.clone(), rust.clone());
    normalize_exiles(&mut java);
    normalize_exiles(&mut rust);
    let mut out = Vec::new();
    diff(&mut out, String::new(), Some(&java), Some(&rust));
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
