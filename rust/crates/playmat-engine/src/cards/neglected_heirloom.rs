//! Neglected Heirloom: "When equipped creature transforms, transform Neglected Heirloom."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "NeglectedHeirloom.NeglectedHeirloomTriggeredAbility",
    "new TransformsAttachedTriggeredAbility(new TransformSourceEffect())",
    any_batch,
)];
