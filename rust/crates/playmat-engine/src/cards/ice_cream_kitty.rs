//! Ice Cream Kitty: "{2}, Sacrifice another creature or a token: Draw a card. Activate only as a sorcery. {2}, {T},
//! Sacrifice this creature: You gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("IceCreamKitty.IceCreamKittyPredicate", another_creature_or_token)];

/// a token (the source too), or a creature that isn't the source
fn another_creature_or_token(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    if game.obj(id).token {
        return true;
    }
    source != Some(id) && game.has_type(id, CardType::Creature)
}
