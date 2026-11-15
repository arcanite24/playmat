//! Howlpack Piper: "This spell can't be countered. {1}{G}, {T}: You may put a creature card from your hand onto the
//! battlefield. If it's a Wolf or Werewolf, untap Howlpack Piper. Activate only as a sorcery. Daybound." Wildsong
//! Howler: "Whenever this creature enters the battlefield or transforms into Wildsong Howler, look at the top six
//! cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on
//! the bottom of your library in a random order. Nightbound."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HowlpackPiper.HowlpackPiperEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURE"])];

/// with a creature card in hand, up to one chosen (`player.choose` of `TargetCardInHand(0, 1)`) and put onto the
/// battlefield; the permanent a Wolf or Werewolf and the source still there, the source untaps
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield || !cx.source_on_battlefield(game) {
        return Some(());
    }
    if game.has_subtype(card, "Wolf") || game.has_subtype(card, "Werewolf") {
        untap_permanent(game, src);
    }
    Some(())
}
