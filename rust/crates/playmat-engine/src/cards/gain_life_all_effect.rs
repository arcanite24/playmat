//! `GainLifeAllEffect(amount[, TargetController])`: "each player gains 3 life" (Explore the Vastlands), from you on
//! in turn order (`getPlayersInRange`), or "each opponent gains …".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GainLifeAllEffect", apply)
    .args(&["text", "one_of:TargetController.EACH_PLAYER|TargetController.ANY|TargetController.OPPONENT"])];

fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let amount = cx.arg_int(0)?;
    let you = cx.controller();
    let opponents_only = cx.args.get(1).is_some_and(|a| a.trim() == "TargetController.OPPONENT");
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if opponents_only && p == you {
            continue;
        }
        gain_life(game, p, amount);
    }
    Some(())
}
