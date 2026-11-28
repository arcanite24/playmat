//! Metallic Mimic: "Each other creature you control of the chosen type enters with an additional +1/+1 counter on
//! it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MetallicMimic.MetallicMimicReplacementEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// another creature entering under your control that has the type chosen as Metallic Mimic entered
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    let Some(t) = game.obj(cx.source).chosen_type.as_ref() else { return false };
    id != cx.source && ev.player == cx.controller && game.has_type(id, CardType::Creature) && game.has_subtype(id, t)
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        add_counters_by(game, id, "+1/+1", 1, Some(cx.controller));
    }
    false
}
