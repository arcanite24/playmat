//! `CanBlockOnlyFlyingEffect(duration)`: "{this} can block only creatures with flying" (the Faerie token of
//! Faerie Slumber Party's `FaerieBlockFliersToken`): the source can't block an attacker without flying.

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("CanBlockOnlyFlyingEffect", applies, &[Restrict::CantBlock(without_flying)])
        .args(&["one_of:Duration.EndOfTurn|Duration.WhileOnBattlefield"])];

fn applies(_: &Game, cx: &StaticCx, blocker: ObjId) -> bool {
    blocker == cx.source
}

/// `canBlock(attacker, blocker)`: only an attacker with flying
fn without_flying(game: &Game, _: &StaticCx, _: ObjId, attacker: ObjId) -> bool {
    !game.has_ability(attacker, &AbilityDef::FlyingAbility)
}
