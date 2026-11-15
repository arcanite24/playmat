//! Vaan, Street Thief: "Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a
//! player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.
//! Whenever you cast a spell you don't own, put a +1/+1 counter on each Scout, Pirate, and Rogue you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VaanStreetThief.VaanStreetThiefEffect", apply).tokens(&["TreasureToken"])];

/// The damaged player's top card exiled; "Cast it?" and the cast now, its costs paid (`CardUtil.castSingle`); no
/// (or no cast) makes a Treasure
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(card) = game.player(p).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    let message = format!("Cast {}?", log::object(game, card));
    // (a land has no spell to cast: `castSingle` fails)
    let land = game.obj(card).is(CardType::Land) && game.obj(card).def.back.is_none();
    let cast = choose_use(game, d, you, &message)?
        && game.obj(card).zone == Zone::Exile
        && !land
        && cast_allowed(game, d, you, card);
    if d.stopped() {
        return None;
    }
    if !cast {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}
