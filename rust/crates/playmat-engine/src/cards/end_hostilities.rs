//! End Hostilities: "Destroy all creatures and all permanents attached to creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EndHostilities.EndHostilitiesEffect", apply)];

/// The creatures and the permanents attached to a creature, found first, then destroyed one by one in battlefield
/// order
fn apply(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.has_type(*o, CardType::Creature)
                || game.attached(*o).is_some_and(|to| game.has_type(to, CardType::Creature))
        })
        .collect();
    for o in doomed {
        if game.obj(o).zone == Zone::Battlefield {
            destroy(game, o);
        }
    }
    Some(())
}
