//! Depressurize: "Target creature gets -3/-0 until end of turn. Then if that creature's power is 0 or less, destroy
//! it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Depressurize.DepressurizeTargetEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.power_toughness(o).0 <= 0 {
        destroy(game, o);
    }
    Some(())
}
