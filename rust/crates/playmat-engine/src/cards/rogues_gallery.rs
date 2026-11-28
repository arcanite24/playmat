//! Rogues' Gallery: "Return a creature card of each color from your graveyard to your hand."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "RoguesGallery.RoguesGalleryTarget",
    "new TargetCardInYourGraveyard(0, Integer.MAX_VALUE, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `possibleTargets`: a colored creature card that, with the cards chosen so far, still gets a color of its own each
/// (`ColorAssignment("W", "U", "B", "R", "G").hasSharedRoles` is false)
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let mut cards: Vec<Vec<Color>> = vec![game.colors(id)];
    if cards[0].is_empty() {
        return false;
    }
    for t in &game.chosen_before {
        if let Target::Obj(o) = t {
            if *o != id {
                cards.push(game.colors(*o));
            }
        }
    }
    let mut used = Vec::new();
    most_assigned(&cards, 0, &mut used) == cards.len()
}

/// `RoleAssignment.getMaxAssignment`: how many of the cards can each take one of their colors, each color going to one
/// card at most
fn most_assigned(cards: &[Vec<Color>], i: usize, used: &mut Vec<Color>) -> usize {
    if i == cards.len() {
        return 0;
    }
    let mut best = most_assigned(cards, i + 1, used);
    for c in &cards[i] {
        if !used.contains(c) {
            used.push(*c);
            best = best.max(1 + most_assigned(cards, i + 1, used));
            used.pop();
        }
    }
    best
}
