//! Garruk, Savage Herald: "+1: Reveal the top card of your library. If it's a creature card, put it into your hand.
//! Otherwise, put it on the bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GarrukSavageHerald.GarrukSavageHeraldEffect", apply)];

/// the top card revealed, then to your hand (a creature card) or the bottom of your library (`putCardsOnBottomOfLibrary`
/// of one card asks nothing)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = *game.player(you).library.first()?;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: vec![(card, game.obj(card).zone)],
    });
    if game.obj(card).def.types.contains(&CardType::Creature) {
        game.move_to(card, Zone::Hand);
    } else {
        to_library(game, card, false);
    }
    Some(())
}
