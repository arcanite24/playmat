//! Tyrannical Pitlord: "Flying, trample. As Tyrannical Pitlord enters, choose another creature you control. The
//! chosen creature gets +3/+3 and has flying. When Tyrannical Pitlord leaves the battlefield, sacrifice the chosen
//! creature."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::continuous("TyrannicalPitlord.TyrannicalPitlordBoostEffect", is_chosen, Layer::ModifyPt(plus_three)),
    Handler::continuous("TyrannicalPitlord.TyrannicalPitlordGainFlyingEffect", is_chosen, Layer::Ability(flying)),
    Handler::effect("TyrannicalPitlord.TyrannicalPitlordSacrificeEffect", sacrifice_chosen),
];

/// the creature chosen as the Pitlord entered (`ChooseCreatureEffect`), still that permanent
fn chosen(game: &Game, source: ObjId) -> Option<ObjId> {
    let (o, zcc) = game.obj(source).chosen?;
    (game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc).then_some(o)
}

fn is_chosen(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    chosen(game, cx.source) == Some(id)
}

fn plus_three(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (3, 3)
}

fn flying(_: &Game, _: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    vec![AbilityDef::FlyingAbility]
}

/// the chosen creature, still on the battlefield, is sacrificed
fn sacrifice_chosen(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(o) = chosen(game, cx.source_id()) {
        sacrifice(game, o);
    }
    Some(())
}
