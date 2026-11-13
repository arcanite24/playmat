//! Cling to Dust: "Exile target card from a graveyard. If it was a creature card, you gain 3 life. Otherwise, you
//! draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ClingToDust.ClingToDustEffect", apply)];

/// Nothing unless the card is exiled; then 3 life for a creature card, else a card drawn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let creature = game.obj(c).is(CardType::Creature);
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone != Zone::Exile {
        return Some(());
    }
    if creature {
        gain_life(game, cx.controller(), 3);
    } else {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
