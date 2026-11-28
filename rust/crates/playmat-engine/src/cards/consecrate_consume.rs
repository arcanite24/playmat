//! Consecrate // Consume: Consume's "Target player sacrifices a creature with the greatest power among creatures
//! they control. You gain life equal to its power."

use super::*;
use crate::card::{Amount, Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("ConsecrateConsume.ConsumeEffect", consume)];

/// the greatest power among the target player's creatures (0 with none); `SacrificeEffect` of a creature with that
/// power; then you gain that much life, whatever was sacrificed
fn consume(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let greatest = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == player && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .fold(0, i32::max);
    let filter =
        Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Power { cmp: Cmp::EqualTo, n: greatest }]);
    cx.run(game, d, &[EffectDef::SacrificeEffect { filter, amount: Amount::Fixed(1) }]);
    if d.stopped() {
        return None;
    }
    gain_life(game, cx.controller(), greatest);
    Some(())
}
