//! `OpponentDealtNoncombatDamageCondition.THIS_TURN`: "an opponent has been dealt noncombat damage this turn"
//! (`NoncombatDamageToPlayersWatcher`: Grim Repriser, Whiplash Wordsmith)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("OpponentDealtNoncombatDamageCondition.THIS_TURN", opponent_dealt_noncombat)];

fn opponent_dealt_noncombat(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.damage.iter().any(|d| !d.combat && matches!(d.to, Target::Player(p) if p != you))
}
