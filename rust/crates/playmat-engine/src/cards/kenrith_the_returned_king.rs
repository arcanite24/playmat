//! Kenrith, the Returned King: "{4}{B}: Put target creature card from a graveyard onto the battlefield under its
//! owner's control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KenrithTheReturnedKing.KenrithTheReturnedKingEffect", apply)];

/// The target card, if still legal, onto the battlefield under its owner (`owner.moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    let owner = game.obj(card).owner;
    if !put_cards(game, d, owner, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
