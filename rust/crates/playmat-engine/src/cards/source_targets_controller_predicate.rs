//! `TargetController.SOURCE_TARGETS.getControllerPredicate()`: "creatures target player controls" (Trystan's
//! Command, Gnashing of Teeth, Wail of War): controlled by the resolving spell's target player
//! (`input.getSource().getFirstTarget()`, the chosen mode's first target). Its cards' only player target is that
//! mode's, so the resolving spell's first player target is it.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SourceTargetsControllerPredicate", controlled_by_target)];

fn controlled_by_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let Some(p) = item.targets.iter().flatten().find_map(|t| if let Target::Player(p) = t { Some(*p) } else { None })
    else {
        return false;
    };
    game.obj(id).controller == p
}
