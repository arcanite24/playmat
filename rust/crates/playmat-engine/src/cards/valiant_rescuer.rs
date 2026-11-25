//! Valiant Rescuer: "Whenever you cycle another card for the first time each turn, create a 1/1 white Human Soldier
//! creature token. Cycling {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ValiantRescuer.ValiantRescuerTriggeredAbility",
    "new CycleControllerTriggeredAbility(new CreateTokenEffect(new HumanSoldierToken()), false, true)",
    first_cycle,
)];

/// `ValiantRescuerWatcher.checkSpell`: the controller cycled no card this turn but the one just cycled (the watcher
/// has already seen it). Checked as it triggers only.
fn first_cycle(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    let you = item.controller;
    let Some(&(_, now)) = game.watch.cycled.iter().rev().find(|(s, _)| *s == you) else { return false };
    !game.watch.cycled.iter().any(|(s, c)| *s == you && *c != now)
}
