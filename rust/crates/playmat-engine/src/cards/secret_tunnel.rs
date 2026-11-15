//! Secret Tunnel: "{4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "SecretTunnel.SecretTunnelTarget",
    "new TargetPermanent(2, 2, StaticFilters.FILTER_CONTROLLED_CREATURE, false)",
    can_target,
)];

/// `canTarget`: the second shares a creature type with the first chosen (`MageObject.shareCreatureTypes`)
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Obj(first)) => super::invasion_of_new_capenna::share_creature_types(game, *first, id),
        _ => true,
    }
}
