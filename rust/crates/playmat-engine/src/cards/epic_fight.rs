//! Epic Fight: "Choose one or both — • Double target creature's power and toughness until end of turn. • Target
//! creature you control fights target creature an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EpicFight.EpicFightEffect", apply)];

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
