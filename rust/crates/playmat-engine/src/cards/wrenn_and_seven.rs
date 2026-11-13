//! Wrenn and Seven: "0: Put any number of land cards from your hand onto the battlefield tapped." and "−8: Return
//! all permanent cards from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WrennAndSeven.WrennAndSevenLandEffect", lands),
    Handler::effect("WrennAndSeven.WrennAndSevenReturnEffect", permanents_back),
];

/// With a land card in hand: any number of them (`TargetCardInHand(0, MAX)`, `Player.choose` rounds), onto the
/// battlefield tapped together
fn lands(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> =
        game.player(you).hand.iter().copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, you, &possible, 0, possible.len())?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}

/// Every permanent card in your graveyard, to your hand
fn permanents_back(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let kinds = [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker, CardType::Battle];
    let cards: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| kinds.iter().any(|k| game.has_type(*c, *k))).collect();
    for c in cards {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
