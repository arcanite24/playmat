//! `TargetController.SOURCE_TARGETS.getControllerPredicate()`: "creatures target player controls" (Trystan's
//! Command, Gnashing of Teeth, Wail of War): controlled by the source's first target
//! (`input.getSource().getFirstTarget()`). While a later target of the same ability is chosen (Cloak and Dagger,
//! Entwined's "up to one target creature they control"), that's the player chosen before it (`game.chosen_before`);
//! as it resolves, the resolving spell's or ability's first player target (its cards' only player target is the
//! chosen mode's).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SourceTargetsControllerPredicate", controlled_by_target)];

fn controlled_by_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let first_player = |ts: &[Target]| ts.iter().find_map(|t| if let Target::Player(p) = t { Some(*p) } else { None });
    let p = match first_player(&game.chosen_before) {
        Some(p) => p,
        None => {
            let Some(item) = game.resolving_item.as_ref() else { return false };
            let all: Vec<Target> = item.targets.iter().flatten().copied().collect();
            let Some(p) = first_player(&all) else { return false };
            p
        }
    };
    game.obj(id).controller == p
}
