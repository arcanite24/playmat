//! Expel from Orazca: "Ascend. Return target nonland permanent to its owner's hand. If you have the city's blessing,
//! you may put that permanent on top of its owner's library instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExpelFromOrazca.ExpelFromOrazcaEffect", apply)];

/// with the city's blessing, "Put … on top of its owner's library instead?"; else (or no) to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let top = game.player(you).citys_blessing && {
        let q = format!("Put {} on top of its owner's library instead?", game.obj(o).name());
        choose_use(game, d, you, &q)?
    };
    if top {
        to_library(game, o, true);
    } else {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}
