//! Long Feng, Grand Secretariat: "Whenever another creature you control or a land you control is put into a graveyard
//! from the battlefield, put a +1/+1 counter on target creature you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("LongFengGrandSecretariat.LongFengGrandSecretariatPredicate", land_or_another_creature)];

/// a land, or a creature other than the source
fn land_or_another_creature(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    game.has_type(id, CardType::Land) || game.has_type(id, CardType::Creature) && Some(id) != source
}
