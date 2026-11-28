//! Rotfeaster Maggot: "When Rotfeaster Maggot enters the battlefield, exile target creature card from a graveyard. You
//! gain life equal to that card's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RotfeasterMaggot.RotfeasterMaggotExileEffect", apply)];

/// The target exiled if it's still in a graveyard; then, wherever it is, life equal to its toughness
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if game.obj(card).zone == Zone::Graveyard {
        game.move_to(card, Zone::Exile);
    }
    let n = game.power_toughness(card).1;
    if n > 0 {
        gain_life(game, cx.controller(), n);
    }
    Some(())
}
