//! Lie in Wait: "Return target creature card from your graveyard to your hand. Lie in Wait deals damage equal to
//! that card's power to target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LieInWait.LieInWaitTargetEffect", apply)];

/// the damage only when the card moved, and the creature is still a legal target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    game.move_to(c, Zone::Hand);
    if game.obj(c).zone != Zone::Hand {
        return Some(());
    }
    let power = game.power_toughness(c).0;
    if let Some(o) = cx.obj(1).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
        deal_damage(game, cx.source_id(), Target::Obj(o), power, false);
    }
    Some(())
}
