//! Every question as a client callback: the engine's prompt (`playmat_engine::prompt`, compared with Java's) where it
//! renders one, and otherwise one the client can answer, with Java's shape and a plainer message.

use playmat_engine::decide::Query;
use playmat_engine::prompt::{object_html, prompt_view};
use playmat_engine::replay::reference;
use playmat_engine::state::{Game, ObjId, Seat, Zone};
use playmat_engine::view::cards_view;
use serde_json::{json, Map, Value};

fn message(method: &str, text: String, options: Value) -> Value {
    json!({"method": method, "data": {"message": text, "flag": false, "min": 0, "max": 0, "options": options}})
}

fn refs(game: &Game, ids: &[ObjId]) -> Vec<String> {
    ids.iter().map(|i| reference(game, *i)).collect()
}

/// What the question is about: the spell or ability being cast or resolving
fn about(game: &Game) -> String {
    let item = game.unpaid.and_then(|at| game.stack.get(at)).or(game.resolving_item.as_ref());
    match item {
        Some(item) => format!(" ({})", object_html(game, item.card)),
        None => String::new(),
    }
}

/// What a target question picks, from what it can pick (the engine's targets carry no text of their own; Java's say
/// `target.getMessage`): "any target", "target creature", "target player", …
fn target_kind(game: &Game, players: &[Seat], objs: &[ObjId]) -> &'static str {
    use playmat_engine::card::CardType;
    let all = |f: &dyn Fn(ObjId) -> bool| !objs.is_empty() && objs.iter().all(|o| f(*o));
    let on_field = |o: ObjId| game.obj(o).zone == Zone::Battlefield;
    let creature = |o: ObjId| on_field(o) && game.has_type(o, CardType::Creature);
    if objs.is_empty() {
        return if players.is_empty() { "a target" } else { "target player" };
    }
    if !players.is_empty() {
        return if objs
            .iter()
            .all(|o| creature(*o) || game.has_type(*o, CardType::Planeswalker) || game.has_type(*o, CardType::Battle))
        {
            "any target"
        } else {
            "a target"
        };
    }
    if all(&creature) {
        "target creature"
    } else if all(&|o| on_field(o) && game.has_type(o, CardType::Land)) {
        "target land"
    } else if all(&|o| on_field(o) && game.has_type(o, CardType::Artifact)) {
        "target artifact"
    } else if all(&|o| on_field(o) && game.has_type(o, CardType::Enchantment)) {
        "target enchantment"
    } else if all(&|o| on_field(o) && game.has_type(o, CardType::Planeswalker)) {
        "target planeswalker"
    } else if all(&on_field) {
        "target permanent"
    } else if all(&|o| game.obj(o).zone == Zone::Stack) {
        "target spell"
    } else if all(&|o| game.obj(o).zone == Zone::Graveyard) {
        "target card in a graveyard"
    } else {
        "a target"
    }
}

/// `{method, data}` for a question, ids as references
pub fn prompt(game: &Game, seat: Seat, q: &Query) -> Value {
    if let Some(p) = prompt_view(game, seat, q) {
        return p;
    }
    match q {
        Query::Target { players, objs, required } => {
            let mut targets: Vec<String> = players.iter().map(|p| format!("P{p}")).collect();
            targets.extend(refs(game, objs));
            // cards that aren't on the battlefield or the stack are shown to choose from
            let off: Vec<ObjId> = objs
                .iter()
                .copied()
                .filter(|o| !matches!(game.obj(*o).zone, Zone::Battlefield | Zone::Stack))
                .collect();
            json!({"method": "GAME_TARGET", "data": {
                "message": format!("Select {}{}", target_kind(game, players, objs), about(game)), "flag": required,
                "cardsView1": cards_view(game, &off), "targets": targets, "min": 0, "max": 0,
                "options": {"queryType": "PICK_TARGET", "possibleTargets": targets.clone()}}})
        }
        Query::Cards { objs, required } => {
            let text = if game.step == Some(playmat_engine::state::Step::Cleanup) {
                "Select a card to discard".to_string()
            } else {
                format!("Select a card{}", about(game))
            };
            // a library search shows the whole library; only the cards of the filter can be picked, and the ones
            // chosen so far can be picked again to take them back (`HumanPlayer.chooseTarget`'s options)
            if let Some((shown, chosen)) = &game.searching {
                let possible: Vec<ObjId> =
                    shown.iter().copied().filter(|c| objs.contains(c) || chosen.contains(c)).collect();
                let mut options = Map::new();
                options.insert("queryType".into(), json!("PICK_TARGET"));
                options.insert("chosenTargets".into(), json!(refs(game, chosen)));
                if !possible.is_empty() {
                    options.insert("possibleTargets".into(), json!(refs(game, &possible)));
                }
                return json!({"method": "GAME_TARGET", "data": {
                    "message": text, "flag": required, "cardsView1": cards_view(game, shown),
                    "targets": refs(game, &possible), "min": 0, "max": 0, "options": options}});
            }
            json!({"method": "GAME_TARGET", "data": {
                "message": text, "flag": required, "cardsView1": cards_view(game, objs),
                "targets": refs(game, objs), "min": 0, "max": 0, "options": {"queryType": "PICK_TARGET"}}})
        }
        Query::BlockTarget { attackers } => json!({"method": "GAME_TARGET", "data": {
            "message": "Select an attacker to block", "flag": true, "targets": refs(game, attackers),
            "min": 0, "max": 0, "options": {"queryType": "PICK_TARGET"}}}),
        Query::PickTrigger { triggers, .. } => {
            // each waiting trigger by its place in the list the engine asks about (`AbilityView`: the source's name)
            let mut cards = Map::new();
            for (i, (source, rule)) in triggers.iter().enumerate() {
                let r = format!("T{seat}.{i}");
                let name = game.obj(*source).name();
                let rule = rule.replace("{this}", name);
                // the source's card, for its art (`AbilityView.sourceCard`)
                let source_card = cards_view(game, &[*source]).as_object().and_then(|m| m.values().next().cloned());
                cards.insert(
                    r.clone(),
                    json!({"id": r, "name": name, "sourceName": name, "sourceCard": source_card,
                           "rules": [rule], "cardTypes": [], "superTypes": [], "subTypes": [], "manaCost": [],
                           "power": "", "toughness": "", "expansionSetCode": "", "cardNumber": ""}),
                );
            }
            let targets: Vec<String> = cards.keys().cloned().collect();
            json!({"method": "GAME_TARGET", "data": {
                "message": "Pick triggered ability (goes to the stack first)", "flag": true,
                "cardsView1": cards, "targets": targets, "min": 0, "max": 0,
                "options": {"queryType": "PICK_ABILITY"}}})
        }
        Query::ChooseMode { modes } => {
            let item = game.unpaid.and_then(|at| game.stack.get(at)).or(game.resolving_item.as_ref());
            // a triggered ability going on the stack chooses its mode before it's there: named by its source
            let (item, r) = match (item, game.moding.as_ref()) {
                (None, Some(t)) => (Some(t), playmat_engine::replay::reference(game, t.card)),
                (i, _) => (i, i.map(|i| playmat_engine::view::stack_ref(game, i.card)).unwrap_or_else(|| "S0".into())),
            };
            let texts = item.map(|i| playmat_engine::view::mode_texts(game, i)).unwrap_or_default();
            let mut choices = Map::new();
            for k in modes {
                let text = texts.get(*k).cloned().unwrap_or_else(|| format!("Mode {}", k + 1));
                choices.insert(format!("{r}sm{k}"), json!(text));
            }
            json!({"method": "GAME_CHOOSE_ABILITY", "data": {"choices": choices, "message": "Choose mode"}})
        }
        Query::Amount { min, max } => json!({"method": "GAME_GET_AMOUNT", "data": {
            "message": format!("Announce the value for X{}", about(game)), "flag": false, "min": min, "max": max,
            "options": {"queryType": "AMOUNT"}}}),
        Query::MultiAmount { defaults, ranges, min, max, source, items } => {
            // `CombatGroup.assignDamageToBlockers`: each row names its creature, the header the attacker
            let creature = |o: ObjId| {
                let (p, t) = game.power_toughness(o);
                format!("{}, P/T: {}/{}", object_html(game, o), p, t)
            };
            let messages: Vec<Value> = ranges
                .iter()
                .enumerate()
                .map(|(i, (lo, hi))| {
                    let text = items.get(i).map(|o| creature(*o)).unwrap_or_else(|| format!("Amount {}", i + 1));
                    json!({"message": text, "min": lo, "max": hi,
                           "defaultValue": defaults.get(i).copied().unwrap_or(*lo)})
                })
                .collect();
            let trample = min < max;
            let (title, header) = match source {
                Some(a) => {
                    let header = format!("Assign combat damage among creatures blocking {}", creature(*a));
                    if trample {
                        (
                            "Assign combat damage (with trample)",
                            format!("{header} (Unassigned damage tramples through)"),
                        )
                    } else {
                        ("Assign combat damage", header)
                    }
                }
                None => ("Divide the amount", String::new()),
            };
            json!({"method": "GAME_GET_MULTI_AMOUNT", "data": {"messages": messages, "min": min, "max": max,
                "options": {"title": title, "header": header, "canCancel": false,
                            "queryType": "MULTI_AMOUNT"}}})
        }
        Query::Choice { message: m, choices } => json!({"method": "GAME_CHOOSE_CHOICE", "data": {
            "choice": {"message": m, "choices": choices, "required": true, "searchEnabled": false},
            "message": m, "options": {"queryType": "CHOOSE_CHOICE"}}}),
        Query::ChooseAbility { .. }
        | Query::Priority { .. }
        | Query::PlayMana { .. }
        | Query::Attackers { .. }
        | Query::Blockers { .. }
        | Query::YesNo { .. } => message("GAME_SELECT", "Play spells and abilities".into(), json!({})),
    }
}

/// Keep or mulligan (`HumanPlayer.chooseMulligan`): `true` mulligans
pub fn mulligan(down_to: usize) -> Value {
    message(
        "GAME_ASK",
        format!("Mulligan <font color=#ffff00>down to {down_to} cards</font>?"),
        json!({"UI.left.btn.text": "Mulligan", "UI.right.btn.text": "Keep", "webPrompt": "mulligan",
               "queryType": "ASK"}),
    )
}

/// A card to put on the bottom after a mulligan (`LondonMulligan`), one at a time
pub fn mulligan_bottom(game: &Game, seat: Seat, more: usize) -> Value {
    let hand = game.player(seat).hand.clone();
    json!({"method": "GAME_TARGET", "data": {
        "message": format!("Select a card ({more} more) to put on the bottom of your library"), "flag": true,
        "targets": refs(game, &hand), "min": 0, "max": 0,
        "options": {"targetZone": "HAND", "chosenTargets": [], "webPrompt": "mulliganBottom",
                    "queryType": "PICK_TARGET"}}})
}

/// Who takes the first turn (`GameImpl.init`), the choosing player's question
pub fn starting_player(seats: usize) -> Value {
    let targets: Vec<String> = (0..seats).map(|s| format!("P{s}")).collect();
    json!({"method": "GAME_TARGET", "data": {
        "message": "Select a starting player", "flag": true, "targets": targets, "min": 0, "max": 0,
        "options": {"webPrompt": "startingPlayer", "queryType": "PICK_TARGET"}}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use playmat_ai::duel::{new_duel, Deck};

    /// The engine takes the triggers it asks about out of `pending`: the prompt lists the ones in the question
    #[test]
    fn trigger_order_lists_the_asked_triggers() {
        let db = playmat_engine::cards();
        let text = "20 Plains\n20 Llanowar Elves\n20 Elvish Mystic";
        let deck = Deck::parse("d", text);
        let game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let a = game.players[0].library[0];
        let b = game.players[0].library[1];
        let q = Query::PickTrigger {
            count: 2,
            triggers: vec![(a, "When {this} enters, draw a card.".into()), (b, "Scry 1.".into())],
        };
        let p = prompt(&game, 0, &q);
        let cards = p["data"]["cardsView1"].as_object().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(p["data"]["targets"].as_array().unwrap().len(), 2);
        let first = &cards["T0.0"];
        assert_eq!(first["rules"][0], format!("When {} enters, draw a card.", game.obj(a).name()));
        assert_eq!(first["sourceCard"]["name"], game.obj(a).name(), "the source's card, for its art");
    }

    /// A triggered ability choosing its mode isn't on the stack yet (Territorial Kavu's attack trigger): the prompt
    /// shows its modes' texts, and each choice names its source, which the answer maps back to
    #[test]
    fn a_triggers_modes_are_named_by_its_source() {
        let db = playmat_engine::cards();
        let deck = Deck::parse("d", "20 Plains\n40 Territorial Kavu");
        let mut game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let kavu = game.players[0].library[0];
        game.move_to(kavu, playmat_engine::state::Zone::Battlefield);
        let k = game.obj(kavu).def.abilities.iter().position(|a| a.trigger_modes().len() == 2).unwrap();
        let zcc = game.obj(kavu).zcc;
        game.moding = Some(playmat_engine::state::StackItem {
            ability: Some((k, zcc)),
            ..playmat_engine::state::StackItem::spell(kavu, 0)
        });
        let p = prompt(&game, 0, &Query::ChooseMode { modes: vec![0, 1] });
        let choices = p["data"]["choices"].as_object().unwrap();
        let r = playmat_engine::replay::reference(&game, kavu);
        assert_eq!(choices[&format!("{r}sm0")], "discard a card. If you do, draw a card.");
        assert_eq!(choices[&format!("{r}sm1")], "exile up to one target card from a graveyard.");
        assert!(crate::ids::is_ref(&format!("{r}sm1")));
    }

    /// A target question says what it targets, from what it can pick
    #[test]
    fn a_target_question_names_its_kind() {
        let db = playmat_engine::cards();
        let deck = Deck::parse("d", "20 Plains\n20 Llanowar Elves");
        let mut game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let lib = game.players[0].library.clone();
        let elf = *lib.iter().find(|c| game.obj(**c).name() == "Llanowar Elves").unwrap();
        let land = *lib.iter().find(|c| game.obj(**c).name() == "Plains").unwrap();
        game.move_to(elf, Zone::Battlefield);
        game.move_to(land, Zone::Battlefield);
        let text = |game: &Game, players: Vec<Seat>, objs: Vec<ObjId>| {
            let p = prompt(game, 0, &Query::Target { players, objs, required: true });
            p["data"]["message"].as_str().unwrap().to_string()
        };
        assert_eq!(text(&game, vec![0, 1], vec![elf]), "Select any target");
        assert_eq!(text(&game, vec![], vec![elf]), "Select target creature");
        assert_eq!(text(&game, vec![1], vec![]), "Select target player");
        assert_eq!(text(&game, vec![], vec![elf, land]), "Select target permanent");
        assert_eq!(text(&game, vec![], vec![land]), "Select target land");
    }

    /// A library search shows the whole library, with only the cards of the filter to pick
    #[test]
    fn a_library_search_shows_the_whole_library() {
        let db = playmat_engine::cards();
        let deck = Deck::parse("d", "10 Plains\n10 Llanowar Elves");
        let mut game = new_duel(&db, [&deck, &deck], 1, 0).unwrap();
        let library = game.players[0].library.clone();
        let plains: Vec<ObjId> = library.iter().copied().filter(|c| game.obj(*c).name() == "Plains").collect();
        game.searching = Some((library.clone(), vec![plains[0]]));
        let p = prompt(&game, 0, &Query::Cards { objs: plains[1..].to_vec(), required: false });
        assert_eq!(p["data"]["cardsView1"].as_object().unwrap().len(), library.len());
        assert_eq!(p["data"]["options"]["possibleTargets"].as_array().unwrap().len(), plains.len());
        assert_eq!(p["data"]["options"]["chosenTargets"], json!([reference(&game, plains[0])]));
    }
}
