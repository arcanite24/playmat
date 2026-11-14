//! Edgar, Moonlit Sovereign: "At the beginning of your end step, if you didn't cast a spell this turn, put two +1/+1
//! counters on Edgar."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("EdgarMoonlitSovereign.EdgarMoonlitSovereignCondition", no_spell_this_turn)];

/// `CastSpellLastTurnWatcher`: none of this turn's spells is yours
fn no_spell_this_turn(game: &Game, you: Seat, _source: ObjId) -> bool {
    !game.watch.spells.iter().any(|s| s.caster == you)
}
