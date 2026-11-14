//! Tymaret, Chosen from Death: "{1}{B}: Exile up to two target cards from graveyards. You gain 1 life for each
//! creature card exiled this way." (its toughness: `SetBaseToughnessSourceEffect`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TymaretChosenFromDeath.TymaretChosenFromDeathEffect", apply)];

/// the targets still there exiled together, then 1 life for each creature card of them now in exile (no gain for
/// none)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards = cx.objs(0);
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let n = cards
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Exile && game.has_type(**c, CardType::Creature))
        .count() as i32;
    if n > 0 {
        gain_life(game, cx.controller(), n);
    }
    Some(())
}
