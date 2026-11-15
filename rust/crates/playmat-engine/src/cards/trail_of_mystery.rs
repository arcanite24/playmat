//! Trail of Mystery: "Whenever a face-down creature you control enters, you may search your library for a basic land
//! card, reveal it, put it into your hand, then shuffle. Whenever a permanent you control is turned face up, if it's a
//! creature, it gets +2/+2 until end of turn."

use super::*;

/// `TurnedFaceUpAllTriggeredAbility` with its own rule text only: a creature you control turned face up, the boost
/// at it
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TrailOfMystery.TrailOfMysteryTriggeredAbility",
    "new TurnedFaceUpAllTriggeredAbility(new BoostTargetEffect(2, 2, Duration.EndOfTurn), new FilterControlledCreaturePermanent(), true)",
    any_batch,
)];
