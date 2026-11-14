//! Calix, Destiny's Hand: "−3: Exile target creature or enchantment you don't control until target enchantment you
//! control leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CalixDestinysHand.CalixDestinysHandExileEffect", apply)];

/// Both targets still there: the first exiled, back when the second (as it is now) leaves the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(theirs), Some(mine)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    if game.obj(theirs).zone != Zone::Battlefield || game.obj(mine).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(mine).zcc;
    game.move_to(theirs, Zone::Exile);
    if game.obj(theirs).zone == Zone::Exile {
        game.exiled_until_leaves.push((mine, zcc, theirs));
    }
    Some(())
}
