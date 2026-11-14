//! Track Down: "Scry 3, then reveal the top card of your library. If it's a creature or land card, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TrackDown.TrackDownEffect", apply)];

/// The top card revealed; a draw for a creature or land card
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let &card = game.player(you).library.first()?;
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(card, Zone::Library)] });
    if game.obj(card).is(CardType::Land) || game.obj(card).is(CardType::Creature) {
        draw(game, you, 1);
    }
    Some(())
}
