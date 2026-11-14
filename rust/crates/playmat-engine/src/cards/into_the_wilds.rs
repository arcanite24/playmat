//! Into the Wilds: "At the beginning of your upkeep, look at the top card of your library. If it's a land card, you
//! may put it onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IntoTheWilds.IntoTheWildsEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    if !game.obj(card).is(CardType::Land) {
        return Some(());
    }
    let message = format!("Put {} onto the battlefield?", game.obj(card).name());
    if choose_use(game, d, you, &message)? && !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
