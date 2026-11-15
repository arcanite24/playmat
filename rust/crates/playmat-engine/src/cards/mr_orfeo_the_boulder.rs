//! Mr. Orfeo, the Boulder: "Whenever you attack, double target creature's power until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MrOrfeoTheBoulder.MrOrfeoTheBoulderEffect", apply)];

/// +X/+0 until end of turn, X its power as this resolves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let p = game.power_toughness(o).0;
    boost(game, o, p, 0, crate::card::Duration::EndOfTurn);
    Some(())
}
