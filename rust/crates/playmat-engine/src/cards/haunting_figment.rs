//! Haunting Figment: "Haunting Figment can't be blocked as long as you've cast an instant or sorcery spell this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("HauntingFigment.HauntingFigmentCondition", cast_instant_or_sorcery)];

/// `SpellsCastWatcher`: one of your spells this turn was an instant or sorcery
fn cast_instant_or_sorcery(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster == you && (game.obj(s.card).def.is(CardType::Instant) || game.obj(s.card).def.is(CardType::Sorcery))
    })
}
