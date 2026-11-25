//! Chronic Flooding: "Whenever enchanted land becomes tapped, its controller mills three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ChronicFlooding.ChronicFloodingAbility",
    "new BecomesTappedAttachedTriggeredAbility(new MillCardsTargetEffect(3), \"enchanted land\", false, SetTargetPointer.PLAYER)",
    any_batch,
)];
