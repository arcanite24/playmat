//! Season of the Bold: "{P}{P}{P} — Until the end of your next turn, whenever you cast a spell, Season of the Bold
//! deals 2 damage to up to one target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SeasonOfTheBold.SeasonOfTheBoldDelayedTriggeredAbility",
    "new SpellCastControllerTriggeredAbility(new DamageTargetEffect(2), false)",
    any_spell,
)];

fn any_spell(_: &Game, _: &StackItem) -> bool {
    true
}
