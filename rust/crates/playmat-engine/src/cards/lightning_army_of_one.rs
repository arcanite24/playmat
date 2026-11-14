//! Lightning, Army of One: "Stagger — Whenever Lightning deals combat damage to a player, until your next turn, if a
//! source would deal damage to that player or a permanent that player controls, it deals double that damage
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LightningArmyOfOne.LightningArmyOfOneEffect", apply)];

/// the replacement effect, for the player the trigger points at, until your next turn (`rules::deal_damage`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    game.damage_doubled_to.push((p, cx.controller()));
    Some(())
}
