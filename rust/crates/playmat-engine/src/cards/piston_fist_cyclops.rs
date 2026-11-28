//! Piston-Fist Cyclops: "Defender. As long as you've cast an instant or sorcery spell this turn, Piston-Fist Cyclops
//! can attack as though it didn't have defender."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("PistonFistCyclops.PistonFistCyclopsCondition", cast_one)];

/// an instant or sorcery spell you cast this turn, other than this card
fn cast_one(game: &Game, you: Seat, source: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster == you
            && s.card != source
            && (game.obj(s.card).def.types.contains(&CardType::Instant)
                || game.obj(s.card).def.types.contains(&CardType::Sorcery))
    })
}
