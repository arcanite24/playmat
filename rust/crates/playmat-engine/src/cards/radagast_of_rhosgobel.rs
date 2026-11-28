//! Radagast of Rhosgobel: "The first creature spell you cast each turn costs {2} less to cast and can be cast as
//! though it had flash."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("RadagastOfRhosgobel.RadagastFirstCastCreatureSpellPredicate", first_creature_spell)];

/// a creature card, while the player (the filter's "you") has cast no creature spell this turn
/// (`RadagastOfRhosgobelWatcher`, counting creature spells as they're cast)
fn first_creature_spell(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.has_type(id, CardType::Creature) && !game.watch.spells.iter().any(|s| s.caster == you && s.creature)
}
