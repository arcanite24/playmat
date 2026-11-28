//! Nissa's Judgment: "Support 2. Choose up to one target creature an opponent controls. Each creature you control
//! with a +1/+1 counter on it deals damage equal to its power to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NissasJudgment.NissasJudgmentEffect", apply)];

/// the spell's second target (`SecondTargetPointer`: the last group, whether or not the pointer reached the engine
/// as its own wrapper); each creature you control with a +1/+1 counter, in battlefield order, deals damage equal to
/// its power to it when that's above 0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(Target::Obj(target)) = cx.legal.last().and_then(|g| g.first()).copied() else { return Some(()) };
    let sources: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| {
            let o = game.obj(*b);
            o.controller == you && game.has_type(*b, CardType::Creature) && o.counter("+1/+1") > 0
        })
        .collect();
    for s in sources {
        if game.obj(target).zone != Zone::Battlefield {
            break;
        }
        let power = game.power_toughness(s).0;
        if power > 0 {
            deal_damage(game, s, Target::Obj(target), power, false);
        }
    }
    Some(())
}
