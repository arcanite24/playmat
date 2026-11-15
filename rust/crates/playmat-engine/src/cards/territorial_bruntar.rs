//! Territorial Bruntar: "Landfall — Whenever a land you control enters, exile cards from the top of your library
//! until you exile a nonland card. You may cast that card this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TerritorialBruntar.TerritorialBruntarEffect", apply)];

/// One card at a time, each its own move; the nonland card is castable this turn, not played as a land
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for card in game.player(you).library.clone() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile && !game.has_type(card, CardType::Land) {
            let turn = game.turn;
            let o = game.obj_mut(card);
            o.may_play = Some((you, turn));
            o.cast_only = true;
            break;
        }
    }
    Some(())
}
