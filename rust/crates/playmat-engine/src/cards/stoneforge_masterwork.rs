//! Stoneforge Masterwork: "Equipped creature gets +1/+1 for each other creature you control that shares a creature
//! type with it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("StoneforgeMasterwork.StoneforgeMasterworkDynamicValue", sharing)];

/// the other creatures you control sharing a creature type with the equipped creature
fn sharing(game: &Game, you: Seat, source: ObjId, _: i32) -> i32 {
    if game.obj(source).zone != Zone::Battlefield {
        return 0;
    }
    let Some(equipped) = game.attached(source) else { return 0 };
    game.battlefield
        .iter()
        .copied()
        .filter(|o| {
            *o != equipped
                && game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
                && super::invasion_of_new_capenna::share_creature_types(game, equipped, *o)
        })
        .count() as i32
}
