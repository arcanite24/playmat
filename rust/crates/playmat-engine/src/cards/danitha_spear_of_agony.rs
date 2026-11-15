//! Danitha, Spear of Agony: "Whenever you cast a spell that targets an opponent or a creature an opponent controls,
//! put a +1/+1 counter on Danitha."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("DanithaSpearOfAgony.TargetsPermanentOrPlayerPredicate", targets_opponent_or_theirs)];

/// `TargetsPermanentOrPlayerPredicate(FILTER_OPPONENTS_PERMANENT_CREATURE, FilterOpponent)`: one of the spell's
/// targets is an opponent of `you`, or a creature on the battlefield one of them controls
fn targets_opponent_or_theirs(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.stack.iter().any(|i| {
        i.card == id
            && i.ability.is_none()
            && i.targets.iter().flatten().any(|t| match t {
                Target::Player(p) => *p != you,
                Target::Obj(x) => {
                    game.obj(*x).zone == Zone::Battlefield
                        && game.has_type(*x, CardType::Creature)
                        && game.obj(*x).controller != you
                }
            })
    })
}
