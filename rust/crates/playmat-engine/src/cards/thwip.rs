//! Thwip!: "Target creature gets +2/+2 and gains flying until end of turn. If it's a Spider, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Thwip.ThwipEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone == Zone::Battlefield && game.has_subtype(o, "Spider") {
        gain_life(game, cx.controller(), 2);
    }
    Some(())
}
