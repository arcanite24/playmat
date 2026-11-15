//! Cryptborn Horror: "Trample. Cryptborn Horror enters the battlefield with X +1/+1 counters on it, where X is the
//! total life lost by your opponents this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CryptbornHorror.CryptbornHorrorEffect", apply)];

/// `OpponentsLostLifeCount`: the life your opponents lost this turn, all together; none when it's 0
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n: i32 = (0..game.seats()).filter(|p| *p != you).map(|p| game.player(p).life_lost).sum();
    if n > 0 {
        let e = EffectDef::AddCountersSourceEffect {
            counter: crate::card::CounterDef { name: "+1/+1".into(), amount: n },
            amount: None,
        };
        cx.run(game, d, &[e]);
    }
    Some(())
}
