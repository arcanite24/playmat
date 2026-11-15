//! Discover the Impossible: "Look at the top five cards of your library. Exile one of them face down and put the rest
//! on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost if
//! it's an instant spell with mana value 2 or less. If you don't, put that card into your hand."

use super::breaching_dragonstorm::cast_for_free;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscoverTheImpossible.DiscoverTheImpossibleEffect", apply)];

/// One of the top five chosen (required) and exiled, the others that are still in the library to the bottom
/// shuffled; then `castSpellWithAttributesForFree` with "instant, mana value less than 3", or else the card goes to
/// your hand. The engine keeps no face-down exile: the card is only hidden from the opponent in Java.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let Some(&card) = choose_cards(game, d, you, &top, &top, 1, 1, "card to exile")?.first() else {
        return Some(());
    };
    game.move_to(card, Zone::Exile);
    let rest: Vec<ObjId> =
        top.into_iter().filter(|c| *c != card && game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false);
    let small_instant = |g: &Game, c: ObjId| g.obj(c).is(CardType::Instant) && g.obj(c).def.mana_value() < 3;
    if !cast_for_free(game, d, you, card, small_instant)? && game.obj(card).zone == Zone::Exile {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}
