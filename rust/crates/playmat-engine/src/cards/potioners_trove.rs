//! Potioner's Trove: "{T}: You gain 2 life. Activate only if you've cast an instant or sorcery spell this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("PotionersTrove.PotionersTroveCondition", cast_one)];

fn cast_one(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster == you
            && (game.obj(s.card).def.types.contains(&CardType::Instant)
                || game.obj(s.card).def.types.contains(&CardType::Sorcery))
    })
}
