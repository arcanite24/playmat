//! Dread Defiler: "Devoid. {3}{C}, Exile a creature card from your graveyard: Target opponent loses life equal to the
//! exiled card's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DreadDefiler.DreadDefilerEffect", apply)];

/// the `ExileFromGraveCost`'s card's power, lost by the target opponent (nothing at 0 or less)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = *cx.item.cost_exiled.first()?;
    let amount = game.obj(card).def.power;
    if amount > 0 {
        let p = cx.player(0)?;
        lose_life_by(game, p, amount, Some(cx.source_id()), false);
    }
    Some(())
}
