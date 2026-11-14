//! Conduit of Ruin: "The first creature spell you cast each turn costs {2} less to cast."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ConduitOfRuin.FirstCastCreatureSpellPredicate", first)];

/// `FirstCastCreatureSpellPredicate`: a creature card, and you've cast no creature spell this turn
/// (`ConduitOfRuinWatcher`, counted as each spell is cast)
fn first(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.has_type(id, CardType::Creature) && !game.watch.spells.iter().any(|s| s.caster == you && s.creature)
}
