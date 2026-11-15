//! Madame Null, Power Broker: "Deathtouch. Whenever another creature you control enters, you may pay life equal to
//! its power. If you do, put that many +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MadameNullPowerBroker.MadameNullPowerBrokerEffect", apply)];

/// `CardUtil.tryPayLife(power)` (the trigger's "may" asked already): with power 1 or more and that much life, the
/// life is lost and that many +1/+1 counters go on it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creature = cx.obj(0)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(creature).0;
    if power < 1 || power > game.player(you).life {
        return Some(());
    }
    lose_life(game, you, power);
    put_counters(game, creature, "+1/+1", power, you);
    Some(())
}
