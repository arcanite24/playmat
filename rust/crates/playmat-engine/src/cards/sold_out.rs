//! Sold Out: "Exile target creature. If it was dealt damage this turn, create a Clue token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoldOut.SoldOutEffect", apply).tokens(&["ClueArtifactToken"])];

/// Whether it was dealt damage this turn is read before it's exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let damaged = crate::card::Filter(vec![Pred::DamagedThisTurn]);
    let flag = game.filter_matches(&damaged, cx.controller(), Some(cx.source_id()), t);
    if !put_cards(game, d, cx.controller(), &[t], crate::card::PutCards::Exiled) {
        return None;
    }
    if flag {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
