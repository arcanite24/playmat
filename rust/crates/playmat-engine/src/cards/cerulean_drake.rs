//! Cerulean Drake: "Flying, protection from red. Sacrifice Cerulean Drake: Counter target spell that targets you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("CeruleanDrake.CeruleanDrakePredicate", targets_you)];

/// The spell's targets include you (the player choosing)
fn targets_you(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.stack
        .iter()
        .rev()
        .find(|i| i.card == id && i.ability.is_none())
        .is_some_and(|i| i.targets.iter().flatten().any(|t| *t == Target::Player(you)))
}
