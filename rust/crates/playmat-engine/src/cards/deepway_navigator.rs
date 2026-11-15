//! Deepway Navigator: "Flash. When this creature enters, untap each other Merfolk you control. As long as you
//! attacked with three or more Merfolk this turn, Merfolk you control get +1/+0."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("DeepwayNavigator.DeepwayNavigatorCondition", three_merfolk)];

/// `DeepwayNavigatorWatcher`: the Merfolk that attacked this turn under your control, each object once
fn three_merfolk(game: &Game, you: Seat, _: ObjId) -> bool {
    game.attacked_this_turn
        .iter()
        .filter(|(a, _)| game.obj(*a).controller == you && game.has_subtype(*a, "Merfolk"))
        .count()
        >= 3
}
