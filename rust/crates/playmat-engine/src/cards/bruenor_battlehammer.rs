//! Bruenor Battlehammer: "Each creature you control gets +2/+0 for each Equipment attached to it. You may pay {0}
//! rather than pay the equip cost of the first equip ability you activate each turn."

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::continuous("BruenorBattlehammer.BruenorBattlehammerBoostEffect", yours, Layer::ModifyPt(equipped)),
    Handler::cost("BruenorBattlehammer.BruenorBattlehammerCostEffect", CostKind::MayFree, first_equip, free),
];

/// creatures you control
fn yours(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && o.controller == cx.controller && game.has_type(id, CardType::Creature)
}

/// +2/+0 for each Equipment attached to it
fn equipped(game: &Game, _: &StaticCx, id: ObjId) -> (i32, i32) {
    let n = game
        .battlefield
        .iter()
        .filter(|e| game.attached(**e) == Some(id) && game.has_subtype(**e, "Equipment"))
        .count() as i32;
    (2 * n, 0)
}

/// an equip ability you activate, when you haven't activated one this turn (`BruenorBattlehammerWatcher`)
fn first_equip(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller
        && matches!(of.ability, Some(AbilityDef::EquipAbility { .. }))
        && !game.watch.equip_activated.contains(&of.seat)
}

/// `clearManaCostsToPay`
fn free(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    *cost = ManaCost::default();
}
