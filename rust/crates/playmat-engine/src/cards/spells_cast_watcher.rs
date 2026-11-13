//! `SpellsCastWatcher`'s conditions: the spells each player cast this turn, as `Game::watch` keeps them.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("HaventCastSpellFromHandThisTurnCondition", none_from_hand),
    Handler::condition("CastAnotherSpellThisTurnCondition", another),
    Handler::condition("CastNoncreatureSpellThisTurnCondition", noncreature),
    Handler::condition("HaventCastSpellThisTurnCondition", none),
    Handler::condition("OpponentCastSpellThisTurnCondition", opponent),
    Handler::condition("CastHistoricSpellThisTurnCondition", historic),
];

fn mine(game: &Game, you: Seat) -> impl Iterator<Item = &crate::state::SpellSeen> {
    game.watch.spells.iter().filter(move |s| s.caster == you)
}

/// "if you haven't cast a spell from your hand this turn"
fn none_from_hand(game: &Game, you: Seat, _: ObjId) -> bool {
    !mine(game, you).any(|s| s.from == Some(crate::state::Zone::Hand))
}

/// "you've cast another spell this turn": one that isn't the source (as the object it is now)
fn another(game: &Game, you: Seat, source: ObjId) -> bool {
    mine(game, you).any(|s| s.card != source)
}

fn noncreature(game: &Game, you: Seat, _: ObjId) -> bool {
    mine(game, you).any(|s| !s.creature)
}

fn none(game: &Game, you: Seat, _: ObjId) -> bool {
    mine(game, you).next().is_none()
}

/// an opponent cast a spell this turn
fn opponent(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().any(|s| s.caster != you)
}

/// you've cast a historic spell this turn (artifact, legendary or Saga)
fn historic(game: &Game, you: Seat, _: ObjId) -> bool {
    mine(game, you).any(|s| s.historic)
}
