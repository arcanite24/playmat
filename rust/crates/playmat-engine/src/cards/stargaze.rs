//! Stargaze: "Look at twice X cards from the top of your library. Put X cards from among them into your hand and the
//! rest into your graveyard. You lose X life."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("Stargaze.StargazeEffect", apply)];

/// X of 1 or more: the top 2X cards; more than X of them, X chosen (`TargetCardInLibrary(x, …)` with
/// `Player.choose`) go to the hand and the rest to the graveyard, else all to the hand; then X life lost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let cards: Vec<ObjId> = game.player(you).library.iter().take(2 * x as usize).copied().collect();
    if cards.len() > x as usize {
        let rule = item_rule(game, cx.item);
        let chosen = choose_cards(game, d, you, &cards, &cards, x as usize, x as usize, &rule)?;
        if !put_cards(game, d, you, &chosen, PutCards::Hand) {
            return None;
        }
        let rest: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
        if !put_cards(game, d, you, &rest, PutCards::Graveyard) {
            return None;
        }
    } else if !put_cards(game, d, you, &cards, PutCards::Hand) {
        return None;
    }
    lose_life(game, you, x);
    Some(())
}
