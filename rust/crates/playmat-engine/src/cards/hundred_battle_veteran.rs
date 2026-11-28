//! Hundred-Battle Veteran: "As long as there are three or more different kinds of counters among creatures you
//! control, this creature gets +2/+4. You may cast this card from your graveyard. If you do, it enters with a finality
//! counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("HundredBattleVeteran.HundredBattleVeteranCondition", three_kinds)];

/// `CounterTypeCount`, as Java counts it: each creature you control's kinds of counters, summed over the creatures
/// (two creatures with +1/+1 counters count two), three or more
fn three_kinds(game: &Game, you: Seat, _: ObjId) -> bool {
    let kinds: usize = game
        .battlefield
        .iter()
        .filter(|id| game.obj(**id).controller == you && game.has_type(**id, CardType::Creature))
        .map(|id| game.obj(*id).counters.iter().filter(|(_, n)| *n > 0).count())
        .sum();
    kinds >= 3
}
