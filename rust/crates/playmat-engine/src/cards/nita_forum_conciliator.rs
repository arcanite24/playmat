//! Nita, Forum Conciliator: "Whenever you cast a spell you don't own, put a +1/+1 counter on each creature you
//! control. {2}, Sacrifice another creature: Exile target instant or sorcery card from an opponent's graveyard. You
//! may cast it this turn, and mana of any type can be spent to cast that spell. If that spell would be put into a
//! graveyard, exile it instead. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NitaForumConciliator.NitaForumConciliatorEffect", apply)];

/// Exiled, then `CardUtil.makeCardPlayable(…, useCastSpellOnly, EndOfTurn, anyColor)` and the graveyard
/// replacement (`ThatSpellGraveyardExileReplacementEffect`) for the rest of the turn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let (you, turn) = (cx.controller(), game.turn);
    let o = game.obj_mut(card);
    o.may_play = Some((you, turn));
    o.cast_only = true;
    o.any_mana = true;
    game.exile_if_graveyard.push(card);
    Some(())
}
