//! Fiery Annihilation: "Fiery Annihilation deals 5 damage to target creature. Exile up to one target Equipment
//! attached to that creature. If that creature would die this turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("FieryAnnihilation.FieryAnnihilationPredicate", attached_to_first_target)];

/// `FieryAnnihilationPredicate`: attached to the ability's first target. Checked as the second target is chosen
/// (the first is in `chosen_before`); with no first target to compare with (the targets checked again as the
/// spell resolves), it holds, as `ControlledByFirstTarget` does
fn attached_to_first_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Obj(x)) => game.attached(id) == Some(*x),
        Some(Target::Player(_)) => false,
        None => true,
    }
}
