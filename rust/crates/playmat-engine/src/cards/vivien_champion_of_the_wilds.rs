//! Vivien, Champion of the Wilds: "−2: Look at the top three cards of your library. Exile one face down and put the
//! rest on the bottom of your library in any order. For as long as it remains exiled, you may look at that card and
//! you may cast it if it's a creature card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VivienChampionOfTheWilds.VivienChampionOfTheWildsEffect", apply)];

/// One of the top three chosen (required) and exiled; a creature card (not a land) may be cast from there for as
/// long as it stays (`CAST_FROM_NOT_OWN_HAND_ZONE`, `Duration.Custom`); the rest go to the bottom in an order you
/// choose. Casting it first asks whether to only look at it, as for any face-down card.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let Some(&card) = choose_cards(game, d, you, &top, &top, 1, 1, "card to exile face down")?.first() else {
        return Some(());
    };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    if game.obj(card).is(CardType::Creature) && !game.obj(card).is(CardType::Land) {
        let o = game.obj_mut(card);
        o.may_play = Some((you, u32::MAX));
        o.cast_only = true;
        o.exiled_face_down = true;
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| *c != card).collect();
    if !put_on_library(game, d, you, &rest, false, true) {
        return None;
    }
    Some(())
}
