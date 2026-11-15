//! Shang-Chi, Master of Kung Fu: "You may activate abilities of creatures you control as though those creatures had
//! haste. {T}: Add two mana of any one color. Spend this mana only to activate abilities of creature sources."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[
    Handler::as_though("ShangChiMasterOfKungFu.ShangChiMasterOfKungFuHasteEffect", AsThough::Haste(haste)),
    Handler::spend("ShangChiMasterOfKungFu.ShangChiMasterOfKungFuManaBuilder", spend),
];

/// creatures you control
fn haste(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Battlefield
        && game.has_type(id, CardType::Creature)
        && game.obj(id).controller == cx.controller
}

/// an activated ability (not a spell, nor a special action) of a creature source
fn spend(game: &Game, cx: &SpendCx) -> bool {
    let special = matches!(
        cx.ability_def(game),
        Some(
            AbilityDef::TurnFaceUpAbility { .. }
                | AbilityDef::RoomUnlockAbility { .. }
                | AbilityDef::ForetellAbility { .. }
                | AbilityDef::PlotAbility { .. }
        )
    );
    cx.ability && !special && game.has_type(cx.on, CardType::Creature)
}
