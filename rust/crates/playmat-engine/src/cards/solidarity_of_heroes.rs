//! Solidarity of Heroes: "Strive — This spell costs {1}{G} more to cast for each target beyond the first. Choose any
//! number of target creatures. Double the number of +1/+1 counters on each of them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SolidarityOfHeroes.SolidarityOfHeroesEffect", apply)];

/// Each target still on the battlefield, in the order chosen, gets as many +1/+1 counters as it has
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let n = game.obj(o).counter("+1/+1");
        if n > 0 {
            let counter = crate::card::CounterDef { name: "+1/+1".into(), amount: n };
            cx.run_on(game, d, &[EffectDef::AddCountersTargetEffect { counter, value: None, second: false }], vec![
                Target::Obj(o),
            ]);
        }
    }
    Some(())
}
