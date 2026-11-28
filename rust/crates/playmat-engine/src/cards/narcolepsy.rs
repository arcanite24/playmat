//! Narcolepsy: "At the beginning of each upkeep, if enchanted creature is untapped, tap it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "Narcolepsy.NarcolepsyTriggeredAbility",
    "new BeginningOfUpkeepTriggeredAbility(TargetController.ANY, new TapEnchantedEffect(), false)",
    enchanted_untapped,
)];

/// `checkInterveningIfClause`, as it triggers and as it resolves: the Aura is on the battlefield, attached to an
/// untapped permanent
fn enchanted_untapped(game: &Game, t: &StackItem) -> bool {
    if game.obj(t.card).zone != Zone::Battlefield {
        return false;
    }
    game.attached(t.card).is_some_and(|o| game.obj(o).zone == Zone::Battlefield && !game.obj(o).tapped)
}
