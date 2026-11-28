//! Absolving Lammasu: "When this creature enters, all suspected creatures are no longer suspected. When this
//! creature dies, you gain 3 life and suspect up to one target creature an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AbsolvingLammasu.AbsolvingLammasuEffect", apply)];

/// every creature on the battlefield stops being suspected
fn apply(game: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    for o in creatures {
        game.obj_mut(o).set_suspected(false);
    }
    Some(())
}
