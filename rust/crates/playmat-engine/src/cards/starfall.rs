//! Starfall: "Starfall deals 3 damage to target creature. If that creature is an enchantment, Starfall deals 3
//! damage to that creature's controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Starfall.StarfallEffect", apply)];

/// The damage to the creature, then (it still on the battlefield, before state-based actions) to its controller
/// if it's an enchantment
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    deal_damage(game, cx.source_id(), Target::Obj(t), 3, false);
    if game.has_type(t, CardType::Enchantment) {
        let p = game.obj(t).controller;
        if !game.player(p).lost {
            deal_damage(game, cx.source_id(), Target::Player(p), 3, false);
        }
    }
    Some(())
}
