//! Glint-Horn Buccaneer: "Haste. Whenever you discard a card, Glint-Horn Buccaneer deals 1 damage to each opponent.
//! {1}{R}, Discard a card: Draw a card. Activate only if Glint-Horn Buccaneer is attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GlintHornBuccaneer.GlintHornBuccaneerTriggeredAbility",
    "new DiscardCardControllerTriggeredAbility(new DamagePlayersEffect(1, TargetController.OPPONENT), false)",
    discards,
)];

/// `DISCARDED_CARD` by you: the base trigger is all of it
fn discards(_: &Game, _: &StackItem) -> bool {
    true
}
