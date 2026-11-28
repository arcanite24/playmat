//! Beseech the Mirror: "Search your library for a card, exile it face down, then shuffle. If this spell was
//! bargained, you may cast the exiled card without paying its mana cost if that spell's mana value is 4 or less.
//! Put the exiled card into your hand if it wasn't cast this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BeseechTheMirror.BeseechTheMirrorEffect", apply)];

/// The search (`TargetCardInLibrary()`: one card), the exile and the shuffle; bargained (the spell kicked), the free
/// cast as `CardUtil.castSpellWithAttributesForFree` offers it: a nonland card of mana value 4 or less, asked
/// whether to cast. A card still in exile goes to the hand.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &crate::card::Filter::default(), 1, 1, cx.source_id())?;
    // (nothing found: Java's `apply` returns before its shuffle)
    let Some(&card) = found.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    shuffle_library(game, you);
    let def = game.obj(card).def.clone();
    if cx.item.kicked && !def.is(CardType::Land) && def.mana_value() <= 4 {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    if game.obj(card).zone == Zone::Exile {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

fn shuffle_library(game: &mut Game, seat: Seat) {
    let mut library = game.player(seat).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(seat).library = library;
}
