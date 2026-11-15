//! Profane Memento: "Whenever a creature card is put into an opponent's graveyard from anywhere, you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ProfaneMemento.ProfaneMementoTriggeredAbility",
    "new PutCardIntoGraveFromAnywhereAllTriggeredAbility(new GainLifeEffect(1), false, StaticFilters.FILTER_CARD_CREATURE, TargetController.OPPONENT)",
    any,
)];

/// `checkTrigger`: a creature card an opponent owns, put into a graveyard: the base trigger's event and filter
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
