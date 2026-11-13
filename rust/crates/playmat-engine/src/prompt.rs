//! Prompts (docs/rust/Views.md): a question as Java's client gets it with its view (`GameController`'s and
//! `GameSessionPlayer`'s callbacks, `RustPrompts` in Java), without the game view, ids written as the decision log
//! writes them. `None` for the questions not rendered yet.

use crate::decide::Query;
use crate::replay::reference;
use crate::state::{Game, ObjId, Seat};
use serde_json::{json, Map, Value};

/// `GameLog.getColorName`
pub fn log_color(colors: &[crate::card::Color]) -> &'static str {
    use crate::card::Color as C;
    match colors {
        [] => "#B0C4DE",
        [_, _, ..] => "#DAA520",
        [C::R] => "#FF6347",
        [C::G] => "#90EE90",
        [C::U] => "#87CEFA",
        [C::W] => "#F0E68C",
        [C::B] => "#696969",
    }
}

/// `GameLog.getColoredObjectIdName`: an object's name for the log and prompts, with its reference (a spell's
/// object id is its `Spell`'s, which the decision log can't name: `?`)
pub fn object_html(game: &Game, id: ObjId) -> String {
    if let Some(g) = crate::view::as_java(game) {
        return object_html(&g, id);
    }
    let r = crate::view::stack_ref(game, id);
    let object = if r.starts_with('S') { "?".to_string() } else { r.clone() };
    format!(
        "<font color='{}' object_id='{}'>{}</font> [{}]",
        log_color(&game.colors(id)),
        object,
        game.obj(id).name(),
        r
    )
}

/// `GameLog.getColoredPlayerName`
pub fn player_html(name: &str) -> String {
    format!("<font color='#20B2AA'>{}</font>", name)
}

/// a `GameClientMessage` with only a message and options (`SELECT`, `PLAY_MANA`, `ASK`)
fn message(method: &str, text: String, options: Value) -> Value {
    json!({"method": method, "data": {"message": text, "flag": false, "min": 0, "max": 0, "options": options}})
}

fn refs(game: &Game, ids: &[ObjId]) -> Vec<String> {
    ids.iter().map(|i| reference(game, *i)).collect()
}

/// The prompt seat `seat`'s client gets with this question, as `RustPrompts.prompt` renders Java's.
pub fn prompt_view(game: &Game, seat: Seat, query: &Query) -> Option<Value> {
    if let Some(g) = crate::view::as_java(game) {
        return prompt_view(&g, seat, query);
    }
    Some(match query {
        Query::Priority { .. } => {
            // `GameImpl.firePriorityEvent`: `canPlaySorcery`
            let sorcery =
                game.active == Some(seat) && game.step.map(|s| s.is_main()).unwrap_or(false) && game.stack.is_empty();
            let text = if sorcery { "Play spells and abilities" } else { "Play instants and activated abilities" };
            message("GAME_SELECT", text.into(), json!({"queryType": "SELECT"}))
        }
        Query::PlayMana { cost, .. } => {
            // `ManaUtil.addSpecialManaPayAbilities`: the cost, then what it's paid for
            let source =
                game.unpaid.and_then(|at| game.stack.get(at)).or(game.resolving_item.as_ref()).map(|item| item.card);
            let text = match source {
                Some(id) => format!("Pay {}<div style='font-size:11pt'>{}</div>", cost, object_html(game, id)),
                None => format!("Pay {}", cost),
            };
            message("GAME_PLAY_MANA", text, json!({"queryType": "PLAY_MANA"}))
        }
        Query::Attackers { creatures } => {
            let mut options = Map::new();
            options.insert("possibleAttackers".into(), json!(refs(game, creatures)));
            if !creatures.is_empty() {
                options.insert("specialButton".into(), json!("All attack"));
            }
            options.insert("queryType".into(), json!("SELECT"));
            message("GAME_SELECT", "Select attackers".into(), Value::Object(options))
        }
        Query::Blockers { creatures } => message(
            "GAME_SELECT",
            "Select blockers".into(),
            json!({"possibleBlockers": refs(game, creatures), "queryType": "SELECT"}),
        ),
        Query::YesNo { message: m } if m.starts_with("You still have mana in your mana pool") => {
            // `HumanPlayer.passWithManaPoolCheck`
            let text = format!("<font color='#D2691E'>{}</font>", m);
            message(
                "GAME_ASK",
                text.clone(),
                json!({"autoAnswerMessage": text, "UI.left.btn.text": "Yes", "UI.right.btn.text": "No",
                       "queryType": "ASK"}),
            )
        }
        Query::ChooseAbility { source, abilities } => {
            // `AbilityPickerView`: each ability's rule, numbered, by its id
            let name = game.obj(*source).name().to_string();
            let rules = crate::view::ability_rules(game, *source).unwrap_or_default();
            let r = reference(game, *source);
            let mut choices = Map::new();
            for (n, k) in abilities.iter().enumerate() {
                let rule = rules.get(*k).cloned().unwrap_or_default().replace("{this}", &name);
                let spell =
                    matches!(game.obj(*source).def.abilities.get(*k), Some(crate::card::AbilityDef::SpellAbility));
                let adventure = game.obj(*source).def.adventure.as_ref().filter(|_| rule.starts_with("Adventure "));
                let rule = if let Some(a) = adventure {
                    format!("Cast {}: {}", a.name, rule)
                } else if spell && rule.is_empty() {
                    format!("Cast {}", name)
                } else if spell && !rule.starts_with("Cast ") {
                    format!("Cast {}: {}", name, rule)
                } else {
                    rule
                };
                choices.insert(format!("{}a{}", r, k), json!(format!("{}. {}", n + 1, rule)));
            }
            // the object its first ability is of: an adventurer's adventure is its face `f1`
            let first = abilities.first().and_then(|k| rules.get(*k));
            let object = match game
                .obj(*source)
                .def
                .adventure
                .as_ref()
                .filter(|_| first.is_some_and(|f| f.starts_with("Adventure ")))
            {
                Some(a) => {
                    format!("<font color='{}' object_id='{}f1'>{}</font> [{}f1]", log_color(&a.colors), r, a.name, r)
                }
                None => object_html(game, *source),
            };
            json!({"method": "GAME_CHOOSE_ABILITY", "data": {"choices": choices,
                "message": format!("Choose spell or ability to play<br>{}", object)}})
        }
        Query::YesNo { message: m } => {
            // `HumanPlayer.chooseUse`: what asks, as its source's name and its ability's id
            let mut options = Map::new();
            let source = asking(game);
            let mut auto = m.clone();
            if let Some((html, original)) = &source {
                auto = auto.replace(html.as_str(), "{this}");
                options.insert("originalId".into(), json!(original));
                options.insert("secondMessage".into(), json!(html));
            }
            options.insert("autoAnswerMessage".into(), json!(auto));
            options.insert("UI.left.btn.text".into(), json!("Yes"));
            options.insert("UI.right.btn.text".into(), json!("No"));
            options.insert("queryType".into(), json!("ASK"));
            message("GAME_ASK", m.clone(), Value::Object(options))
        }
        _ => return None,
    })
}

/// What a question comes from, as Java's source ability: the spell or ability being cast or activated, else what's
/// resolving, else a card entering (its as-enters ability): its source's log name and the ability's id
fn asking(game: &Game) -> Option<(String, String)> {
    use crate::card::{AbilityDef, EffectDef};
    let item = game.unpaid.and_then(|at| game.stack.get(at)).or(game.resolving_item.as_ref());
    if let Some(item) = item {
        let html = object_html(game, item.card);
        let r = crate::view::stack_ref(game, item.card);
        let original = match item.ability {
            Some((k, _)) if !r.starts_with('S') => format!("{}a{}", r, k),
            _ => r,
        };
        return Some((html, original));
    }
    let (id, _, _) = game.entering_from_hand?;
    let k = game.obj(id).def.abilities.iter().position(|a| {
        matches!(a, AbilityDef::AsEntersBattlefieldAbility { effects, .. } | AbilityDef::EntersBattlefieldAbility { effects, .. }
            if effects.iter().any(|e| matches!(e, EffectDef::TapSourceUnlessPaysEffect { .. } | EffectDef::TapSourceUnlessRevealsEffect { .. })))
    })?;
    Some((object_html(game, id), format!("{}a{}", reference(game, id), k)))
}
