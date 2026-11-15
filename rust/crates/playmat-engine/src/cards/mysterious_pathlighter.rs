//! Mysterious Pathlighter: "Flying. Each creature you control that has an Adventure enters with an additional +1/+1
//! counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MysteriousPathlighter.MysteriousPathlighterEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// a creature card (not a token: `PermanentCard` of an `AdventureCard`) with an Adventure entering under your control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    let o = game.obj(id);
    ev.player == cx.controller && !o.token && o.def.adventure.is_some() && game.has_type(id, CardType::Creature)
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        add_counters_by(game, id, "+1/+1", 1, Some(cx.controller));
    }
    false
}
