//! Breaching Dragonstorm: "When this enchantment enters, exile cards from the top of your library until you exile a
//! nonland card. You may cast it without paying its mana cost if that spell's mana value is 8 or less. If you don't,
//! put that card into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BreachingDragonstorm.BreachingDragonstormEffect", apply)];

/// Exiled one at a time down to the first nonland card; `castSpellWithAttributesForFree` with a "mana value 8 or
/// less" filter (asked only when it's castable); still in exile, it goes to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        game.move_to(c, Zone::Exile);
        if !game.has_type(c, CardType::Land) {
            hit = Some(c);
            break;
        }
    }
    let Some(card) = hit else { return Some(()) };
    cast_for_free(game, d, you, card, |g, c| (g.obj(c).def.mana_value() as i32) < 9)?;
    if game.obj(card).zone == Zone::Exile {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

/// `CardUtil.castSpellWithAttributesForFree(player, source, game, card, filter)` for one card: nothing unless it's a
/// nonland card the filter allows; `chooseUse`, then the free cast. Some(true) when it was cast.
pub(super) fn cast_for_free(
    game: &mut Game,
    d: &mut dyn Decider,
    you: Seat,
    card: ObjId,
    filter: impl Fn(&Game, ObjId) -> bool,
) -> Option<bool> {
    if game.has_type(card, CardType::Land) || !filter(game, card) {
        return Some(false);
    }
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if !choose_use(game, d, you, &message)? {
        return Some(false);
    }
    let cast = cast_free(game, d, you, card);
    if d.stopped() {
        return None;
    }
    Some(cast)
}
