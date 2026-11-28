//! Sigarda's Summons: "Creatures you control with +1/+1 counters on them have base power and toughness 4/4, have
//! flying, and are Angels in addition to their other types."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("SigardasSummons.SigardasSummonsEffect", with_counters, Layer::Type(angel))
        .and_layers(&[Layer::Ability(flying), Layer::SetPt(four_four)])];

/// `FILTER_CONTROLLED_CREATURE_P1P1`: a creature you control with a +1/+1 counter on it
fn with_counters(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && o.counter("+1/+1") > 0
        && game.has_type(id, CardType::Creature)
}

fn angel(_: &Game, _: &StaticCx, _: ObjId) -> Vec<String> {
    vec!["Angel".into()]
}

fn flying(_: &Game, _: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    vec![AbilityDef::FlyingAbility]
}

fn four_four(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (4, 4)
}
