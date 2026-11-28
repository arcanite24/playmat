//! `DefendingPlayerControlsAttachedAttackingPredicate`: a permanent the defending player of the attacking creature
//! the source is attached to controls (Captain America's Shield: "tap target creature defending player controls").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate(
    "DefendingPlayerControlsAttachedAttackingPredicate",
    defending_player_controls,
)];

/// the source's creature attacks a player who controls `id` (`game.getCombat().getDefendingPlayerId`)
fn defending_player_controls(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let Some(attacker) = game.attached(src) else { return false };
    game.obj(attacker).attacking.is_some_and(|p| game.obj(id).controller == p)
}
