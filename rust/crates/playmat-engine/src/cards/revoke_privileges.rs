//! Revoke Privileges: "Enchant creature. Enchanted creature can't attack, block, or crew Vehicles."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("RevokePrivileges.RevokePrivilegeCantCrewEffect", Rule::CantCrew(enchanted))];

/// `CREW_VEHICLE` of the enchanted creature
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}
