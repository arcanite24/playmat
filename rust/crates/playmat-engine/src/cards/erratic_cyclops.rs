//! Erratic Cyclops: "Trample. Whenever you cast an instant or sorcery spell, Erratic Cyclops gets +X/+0 until end of
//! turn, where X is that spell's mana value."

use super::*;

/// `SPELL_CAST` of an instant or sorcery you control; the boost is the spell's mana value as it was cast (the
/// trigger's X, set as it triggers: `spell.getManaValue()` in `checkTrigger`)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ErraticCyclops.ErraticCyclopsTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new BoostSourceEffect(GetXValue.instance, StaticValue.get(0), Duration.EndOfTurn), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false)",
    any_batch,
)];
