//! Icingdeath, Frost Tyrant: its token, Icingdeath, Frost Tongue: "Whenever equipped creature attacks, tap target
//! creature defending player controls."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("IcingdeathFrostTongueToken.IcingdeathFrostTonguePredicate", defending_player_controls)];

/// `IcingdeathFrostTonguePredicate`: controlled by the player the equipped creature is attacking
fn defending_player_controls(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let Some((to, _)) = game.obj(src).attached_to else { return false };
    game.obj(to).attacking == Some(game.obj(id).controller)
}
