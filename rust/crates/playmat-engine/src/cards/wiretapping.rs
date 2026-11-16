//! Wiretapping: "Hideaway 5. Whenever you draw your first card during each of your draw steps, draw a card. Then if
//! you have nine or more cards in hand, you may play the exiled card without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "Wiretapping.WiretappingTriggeredAbility",
    "new DrawCardControllerTriggeredAbility(new DrawCardSourceControllerEffect(1), false)",
    first_in_draw_step,
)];

/// your turn's draw step, and the card drawn is the first of it (`CardsDrawnDuringDrawStepWatcher` at one); Java
/// checks it only as it triggers, not again as it resolves
fn first_in_draw_step(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    let you = item.controller;
    game.active == Some(you) && game.step == Some(Step::Draw) && game.player(you).drawn_in_draw_step == 1
}
