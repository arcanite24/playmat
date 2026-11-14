//! Mage Duel: "This spell costs {2} less to cast if you've cast another instant or sorcery spell this turn. Target
//! creature you control gets +1/+2 until end of turn. Then it fights target creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("MageDuel.MageDuelCondition", another_instant_or_sorcery)];

/// You've cast an instant or sorcery spell this turn that isn't the source (`SpellsCastWatcher`)
fn another_instant_or_sorcery(game: &Game, you: Seat, source: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster == you && s.card != source && {
            let def = &game.obj(s.card).def;
            def.is(CardType::Instant) || def.is(CardType::Sorcery)
        }
    })
}
