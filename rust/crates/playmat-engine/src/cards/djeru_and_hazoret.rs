//! Djeru and Hazoret: "As long as you have one or fewer cards in hand, Djeru and Hazoret has vigilance and haste.
//! Whenever Djeru and Hazoret attacks, look at the top six cards of your library. You may exile a legendary creature
//! card from among them. Put the rest on the bottom of your library in a random order. Until end of turn, you may
//! cast the exiled card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DjeruAndHazoret.DjeruAndHazoretEffect", apply).filters(&["filter"])];

/// up to one legendary creature card of the top six (`player.choose`), exiled and castable by you for free
/// (`PlayFromNotOwnHandZoneTargetEffect(EXILED, YOU, Duration.Custom, true)`: while it stays exiled); the rest to
/// the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let possible: Vec<ObjId> =
        cards.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), *c)).collect();
    // (`HumanPlayer.choose` over the cards: asked even with none of them choosable)
    let chosen =
        if cards.is_empty() { Vec::new() } else { super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)? };
    if let Some(&card) = chosen.first() {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Exiled) {
            return None;
        }
        if game.obj(card).zone == Zone::Exile {
            let o = game.obj_mut(card);
            o.may_play = Some((you, u32::MAX));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.play_free = true;
        }
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    Some(())
}
