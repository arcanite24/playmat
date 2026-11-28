//! Leyline of Combustion: "Whenever you and/or at least one permanent you control becomes the target of a spell or
//! ability an opponent controls, Leyline of Combustion deals 2 damage to that player."

use super::*;

/// (once for each targeting object, however many of you and yours it targets)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "LeylineOfCombustion.LeylineOfCombustionTriggeredAbility",
    "new BecomesTargetPlayerOrPermanentTriggeredAbility(new DamageTargetEffect(2), StaticFilters.FILTER_CONTROLLED_PERMANENT, StaticFilters.FILTER_SPELL_OR_ABILITY_A, TargetController.OPPONENT, SetTargetPointer.PLAYER, TargetController.YOU, true, false)",
    any_batch,
)];
