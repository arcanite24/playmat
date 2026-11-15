//! With Great Power . . .: "Enchant creature you control. Enchanted creature gets +2/+2 for each Aura and Equipment
//! attached to it. All damage that would be dealt to you is dealt to enchanted creature instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("WithGreatPower.WithGreatPowerEffect", &[EventType::DamagePlayer], applies, replace)];

/// damage to you (the Aura's controller)
fn applies(_game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// dealt to the enchanted creature instead while it's still the permanent the Aura was attached to
/// (`getPermanentSourceAttachedToIfItStillExists`); replaced either way, so with none it isn't dealt at all
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let attached = game
        .obj(cx.source)
        .attached_to
        .filter(|(o, z)| game.obj(*o).zone == Zone::Battlefield && game.obj(*o).zcc == *z)
        .map(|(o, _)| o);
    match attached {
        Some(o) => redirect_damage(game, ev, o),
        None => ev.amount = 0,
    }
    true
}
