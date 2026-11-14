//! Dire Fleet Daredevil: "When Dire Fleet Daredevil enters, exile target instant or sorcery card from an opponent's
//! graveyard. You may cast it this turn, and mana of any type can be spent to cast that spell. If that spell would
//! be put into a graveyard this turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DireFleetDaredevil.DireFleetDaredevilEffect", apply)];

/// the target into exile, playable this turn with mana of any type (`makeCardPlayable(…, true, EndOfTurn, true)`),
/// and `ThatSpellGraveyardExileReplacementEffect` on it
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let turn = game.turn;
    let o = game.obj_mut(card);
    o.may_play = Some((you, turn));
    o.any_mana = true;
    game.exile_if_graveyard.push(card);
    Some(())
}
