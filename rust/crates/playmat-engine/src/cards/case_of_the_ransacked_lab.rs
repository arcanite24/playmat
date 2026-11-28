//! Case of the Ransacked Lab: "Instant and sorcery spells you cast cost {1} less to cast. To solve — You've cast four
//! or more instant and sorcery spells this turn. Solved — Whenever you cast an instant or sorcery spell, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("CaseOfTheRansackedLab.CaseOfTheRansackedLabCondition", four_instants_and_sorceries)];

/// `CaseOfTheRansackedLabWatcher`: the instant and sorcery spells you cast this turn, more than three
fn four_instants_and_sorceries(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch
        .spells
        .iter()
        .filter(|s| s.caster == you && (game.obj(s.card).is(CardType::Instant) || game.obj(s.card).is(CardType::Sorcery)))
        .count()
        > 3
}
