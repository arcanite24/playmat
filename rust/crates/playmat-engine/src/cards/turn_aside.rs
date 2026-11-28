//! Turn Aside: "Counter target spell that targets a permanent you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("TurnAside.TargetsPermanentPredicate", targets_permanent_you_control)];

/// `TargetsPermanentPredicate(new FilterControlledPermanent())`: the spell has a target that's a permanent "you"
/// control
fn targets_permanent_you_control(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.stack.iter().filter(|i| i.card == id && i.ability.is_none()).any(|i| {
        i.targets.iter().flatten().any(|t| match t {
            Target::Obj(o) => game.obj(*o).zone == Zone::Battlefield && game.obj(*o).controller == you,
            Target::Player(_) => false,
        })
    })
}
