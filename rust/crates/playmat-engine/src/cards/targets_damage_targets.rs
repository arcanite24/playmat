//! `TargetsDamageTargetsEffect` (Band Together…): "up to two target creatures you control each deal damage equal to
//! their power to another target creature". The permanents of every target but the last (Java's tags 1 and 2:
//! Graceful Takedown's two) deal the damage, one at a time, to the last target's; nothing happens unless both sides
//! are still there.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TargetsDamageTargetsEffect", apply).args(&["text"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.legal.len();
    if n < 2 {
        return Some(());
    }
    let on = |game: &Game, o: ObjId| game.obj(o).zone == Zone::Battlefield;
    let dealing: Vec<ObjId> = (0..n - 1).flat_map(|i| cx.objs(i)).filter(|o| on(game, *o)).collect();
    let receiving: Vec<ObjId> = cx.objs(n - 1).into_iter().filter(|o| on(game, *o)).collect();
    if dealing.is_empty() || receiving.is_empty() {
        return Some(());
    }
    for r in receiving {
        for s in &dealing {
            let power = game.power_toughness(*s).0;
            deal_damage(game, *s, Target::Obj(r), power, false);
        }
    }
    Some(())
}
