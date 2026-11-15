//! Spider-Man Noir: "Menace. Whenever a creature you control attacks alone, put a +1/+1 counter on it. Then surveil
//! X, where X is the number of counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpiderManNoir.SpiderManNoirEffect", apply)];

/// surveil X for the total of the counters on the attacker (as it last was on the battlefield, once it has left)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.item.targets.first().and_then(|g| g.first()).and_then(|t| match t {
        Target::Obj(o) => Some(*o),
        _ => None,
    }) else {
        return Some(());
    };
    let obj = game.obj(o);
    let counters = if obj.zone == Zone::Battlefield { &obj.counters } else { &obj.lki_counters };
    let x: i32 = counters.iter().map(|(_, n)| *n).sum();
    cx.run(game, d, &[EffectDef::SurveilEffect { amount: Amount::Fixed(x) }]);
    Some(())
}
