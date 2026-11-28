//! Jetmir, Nexus of Revels: "Creatures you control get +1/+0 and have vigilance as long as you control three or more
//! creatures. Creatures you control also get +1/+0 and have trample as long as you control six or more creatures.
//! Creatures you control also get +1/+0 and have double strike as long as you control nine or more creatures."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("JetmirNexusOfRevels.JetmirNexusOfRevelsEffect", yours, Layer::Ability(abilities))
        .and_layers(&[Layer::ModifyPt(boost)])];

fn creature_you_control(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && o.controller == cx.controller && game.has_type(id, CardType::Creature)
}

/// `min(creatures you control / 3, 3)`
fn level(game: &Game, cx: &StaticCx) -> i32 {
    let n = game.battlefield.iter().filter(|id| creature_you_control(game, cx, **id)).count() as i32;
    (n / 3).min(3)
}

fn yours(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    creature_you_control(game, cx, id) && level(game, cx) >= 1
}

fn abilities(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let l = level(game, cx);
    let mut out = vec![AbilityDef::VigilanceAbility];
    if l > 1 {
        out.push(AbilityDef::TrampleAbility);
    }
    if l > 2 {
        out.push(AbilityDef::DoubleStrikeAbility);
    }
    out
}

fn boost(game: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    (level(game, cx), 0)
}
