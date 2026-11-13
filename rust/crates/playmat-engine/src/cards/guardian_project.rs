//! Guardian Project: "Whenever a nontoken creature you control enters, if it doesn't have the same name as another
//! creature you control or a creature card in your graveyard, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GuardianProject.GuardianProjectTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE_NON_TOKEN)",
    check,
)];

/// `checkCondition`, as it triggers (`checkTrigger`) and as it resolves (`GuardianProjectEffect`, with the creature
/// as it last was): no card of its name in your graveyard (any card: `FilterCard`), no other creature of its name you
/// control
fn check(game: &Game, t: &StackItem) -> bool {
    let Some(entered) = t.entering else { return false };
    let you = t.controller;
    let name = game.obj(entered).name().to_string();
    if !name.is_empty() && game.player(you).graveyard.iter().any(|c| game.obj(*c).name() == name) {
        return false;
    }
    !game.battlefield.iter().any(|o| {
        *o != entered
            && game.obj(*o).controller == you
            && game.has_type(*o, CardType::Creature)
            && game.obj(*o).name() == name
    })
}
