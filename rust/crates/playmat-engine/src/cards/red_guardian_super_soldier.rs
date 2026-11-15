//! Red Guardian, Super-Soldier: "When this creature enters, destroy target creature an opponent controls that dealt
//! damage this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate(
    "RedGuardianSuperSoldier.RedGuardianSuperSoldierPredicate",
    super::executioners_swing::dealt_damage_this_turn,
)];
