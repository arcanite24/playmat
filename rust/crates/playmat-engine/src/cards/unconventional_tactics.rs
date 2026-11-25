//! Unconventional Tactics: "Target creature gets +3/+3 and gains flying until end of turn. Whenever a Zombie you
//! control enters, you may pay {W}. If you do, return Unconventional Tactics from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "UnconventionalTactics.UnconventionalTacticsTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(Zone.GRAVEYARD, new DoIfCostPaid(new ReturnToHandSourceEffect(), new ManaCostsImpl<>(\"{W}\")), filter, false)",
    creature,
)];

/// `checkTrigger`: the Zombie you control that entered is a creature
fn creature(game: &Game, item: &StackItem) -> bool {
    item.entering.is_some_and(|e| game.has_type(e, CardType::Creature))
}
