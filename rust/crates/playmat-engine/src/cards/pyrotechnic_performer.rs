//! Pyrotechnic Performer: "Whenever Pyrotechnic Performer or another creature you control is turned face up, that
//! creature deals damage equal to its power to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PyrotechnicPerformer.PyrotechnicPerformerEffect", apply)];

/// the creature turned face up (its last known power if it's gone) deals that much to each opponent
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = *cx.item.targets.first()?.iter().find_map(|t| if let Target::Obj(o) = t { Some(o) } else { None })?;
    let zcc = cx.item.target_zcc.first().and_then(|z| z.first()).copied();
    let o = game.obj(creature);
    let power = if o.zone == Zone::Battlefield && Some(o.zcc) == zcc {
        game.power_toughness(creature).0
    } else {
        o.power_lki?
    };
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if !game.player(p).lost {
            deal_damage(game, creature, Target::Player(p), power, false);
        }
    }
    Some(())
}
