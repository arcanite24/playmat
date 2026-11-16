//! Encircling Fissure: "Prevent all combat damage that would be dealt this turn by creatures target opponent
//! controls."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EncirclingFissure.EncirclingFissurePreventEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// combat damage from a creature on the battlefield controlled by the target opponent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(opponent) = cx.target_player(0) else { return false };
    let Some(src) = ev.source else { return false };
    let o = game.obj(src);
    cx.preventable(ev, true)
        && ev.amount > 0
        && o.zone == Zone::Battlefield
        && o.controller == opponent
        && game.has_type(src, CardType::Creature)
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
