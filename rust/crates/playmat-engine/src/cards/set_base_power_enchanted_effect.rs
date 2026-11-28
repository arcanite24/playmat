//! `SetBasePowerEnchantedEffect(n)`: "Enchanted creature has base power n" (Withercrown).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("SetBasePowerEnchantedEffect", enchanted, Layer::SetPower(power)).args(&["text"])];

/// the permanent the source is attached to (`getPermanentSourceAttachedToIfItStillExists`)
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

/// the base power it sets (`setModifiedBaseValue`, layer 7b), the power alone
fn power(_: &Game, cx: &StaticCx, _: ObjId) -> i32 {
    cx.arg_int(0).unwrap_or(0)
}
