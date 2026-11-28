//! Rustler Rampage: "Untap all creatures target player controls. Target creature you control gains double strike
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RustlerRampage.RustlerRampageEffect", apply)];

/// the first target, a player: each creature they control untaps
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in mine {
        untap_permanent(game, o);
    }
    Some(())
}
