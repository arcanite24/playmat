//! Saving Grace: "When Saving Grace enters, all damage that would be dealt this turn to you and permanents you
//! control is dealt to enchanted creature instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "SavingGrace.SavingGraceReplacementEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage to you, or to a permanent you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| game.obj(t).zone == Zone::Battlefield && game.obj(t).controller == cx.controller),
    }
}

/// dealt to the creature Saving Grace enchants instead, while the Aura is on the battlefield attached to one
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    let Some(creature) = game.attached(cx.source) else { return false };
    redirect_damage(game, ev, creature);
    true
}
