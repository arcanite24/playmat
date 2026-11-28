//! Orbital Plunge: "Orbital Plunge deals 6 damage to target creature. If excess damage was dealt to a permanent this
//! way, create a Lander token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OrbitalPlunge.OrbitalPlungeEffect", apply).tokens(&["LanderToken"])];

/// 6 damage with excess (`damageWithExcess`); any excess makes a Lander
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    if super::goblin_negotiation::damage_with_excess(game, cx.source_id(), o, 6) > 0 {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
