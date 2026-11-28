//! Dragonclaw Strike: "Double the power and toughness of target creature you control until end of turn. Then it
//! fights up to one target creature an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DragonclawStrike.DragonclawStrikeEffect", apply)];

/// +X/+Y until end of turn, X and Y its power and toughness as this resolves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let (p, t) = game.power_toughness(o);
    boost(game, o, p, t, crate::card::Duration::EndOfTurn);
    Some(())
}
