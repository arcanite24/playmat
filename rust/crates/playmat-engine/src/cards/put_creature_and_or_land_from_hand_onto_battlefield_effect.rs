//! `PutCreatureAndOrLandFromHandOntoBattlefieldEffect`: "you may put a creature card and/or a land card from your
//! hand onto the battlefield" (Michelangelo, Improviser; Yuna's Decision).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PutCreatureAndOrLandFromHandOntoBattlefieldEffect", apply)];

/// `choose(hand, CreatureAndOrLandInHandTarget)`, In the Presence of Ages' pick among the hand, then the chosen
/// cards onto the battlefield together (`moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    // an empty hand: nothing asked (`HumanPlayer.choose` ignores empty cards)
    if hand.is_empty() {
        return Some(());
    }
    let chosen = super::in_the_presence_of_ages::choose_creature_and_or_land(game, d, you, &hand)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
