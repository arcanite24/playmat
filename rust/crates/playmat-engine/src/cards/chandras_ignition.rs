//! Chandra's Ignition: "Target creature you control deals damage equal to its power to each other creature and each
//! opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChandrasIgnition.ChandrasIgnitionEffect", apply)];

/// With a power above 0: each other creature in battlefield order, then each opponent in turn order from you
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(t).0;
    if power <= 0 {
        return Some(());
    }
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| *o != t && game.has_type(*o, CardType::Creature)).collect();
    for o in creatures {
        if game.obj(o).zone == Zone::Battlefield {
            deal_damage(game, t, Target::Obj(o), power, false);
        }
    }
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost {
            deal_damage(game, t, Target::Player(p), power, false);
        }
    }
    Some(())
}
