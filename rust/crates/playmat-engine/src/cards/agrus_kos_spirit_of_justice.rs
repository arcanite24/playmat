//! Agrus Kos, Spirit of Justice: "Double strike, vigilance. Whenever Agrus Kos enters or attacks, choose up to one
//! target creature. If it's suspected, exile it. Otherwise, suspect it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AgrusKosSpiritOfJustice.AgrusKosSpiritOfJusticeEffect", apply)];

/// a suspected target is exiled (`moveCards` to exile); any other is suspected
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    if !game.obj(o).suspected() {
        game.obj_mut(o).set_suspected(true);
    } else {
        game.move_to(o, Zone::Exile);
    }
    Some(())
}
