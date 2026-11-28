//! Orzhov Charm: "Choose one — • Return target creature you control and all Auras you control attached to it to
//! their owner's hand. • Destroy target creature and you lose life equal to its toughness. • Return target creature
//! card with mana value 1 or less from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("OrzhovCharm.OrzhovCharmReturnToHandEffect", return_to_hand),
    Handler::effect("OrzhovCharm.OrzhovCharmDestroyAndLoseLifeEffect", destroy_and_lose_life),
];

/// `moveCards` of the creature and the Auras you control attached to it (in attachment order) to the hand
fn return_to_hand(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let mut cards = vec![t];
    cards.extend(game.attachments(t).filter(|a| game.obj(*a).controller == you && game.has_subtype(*a, "Aura")));
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Hand) {
        return None;
    }
    Some(())
}

/// The toughness before it's destroyed; above 0, that much life lost
fn destroy_and_lose_life(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let toughness = game.power_toughness(t).1;
    destroy(game, t);
    if toughness > 0 {
        lose_life(game, cx.controller(), toughness);
    }
    Some(())
}
