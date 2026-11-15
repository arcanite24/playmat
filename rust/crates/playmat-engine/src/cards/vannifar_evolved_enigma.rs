//! Vannifar, Evolved Enigma: "At the beginning of combat on your turn, choose one — • Cloak a card from your hand.
//! • Put a +1/+1 counter on each colorless creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VannifarEvolvedEnigma.VannifarCloakAbility", cloak_from_hand)];

/// `controller.choose(TargetCardInHand)`: nothing in hand, nothing chosen; the card chosen is cloaked
/// (`ManifestEffect.doManifestCards(…, true)`)
fn cloak_from_hand(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?;
    if !chosen.is_empty() {
        manifest(game, d, you, &chosen, true);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
