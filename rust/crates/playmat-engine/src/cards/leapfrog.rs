//! Leapfrog: "Leapfrog has flying as long as you've cast an instant or sorcery spell this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("Leapfrog.LeapfrogCondition", cast_instant_or_sorcery)];

/// You've cast an instant or sorcery spell this turn (`SpellsCastWatcher`)
fn cast_instant_or_sorcery(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster == you && {
            let def = &game.obj(s.card).def;
            def.is(CardType::Instant) || def.is(CardType::Sorcery)
        }
    })
}
