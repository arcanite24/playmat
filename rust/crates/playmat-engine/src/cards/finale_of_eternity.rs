//! Finale of Eternity: "Destroy up to three target creatures with toughness X or less. If X is 10 or more, return all
//! creature cards from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinaleOfEternity.FinaleOfEternityEffect", apply)];

/// one `moveCards` of every creature card in your graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.item.x < 10 {
        return Some(());
    }
    let you = cx.controller();
    let cards: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
