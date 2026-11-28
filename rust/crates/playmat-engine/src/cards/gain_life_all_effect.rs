//! `GainLifeAllEffect`: "each player gains N life" (Explore the Vastlands, Restorative Burst, Centaur Peacemaker),
//! or with `TargetController.OPPONENT`, "each opponent gains N life"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GainLifeAllEffect", apply)];

/// Each player in range from you in turn order (`getPlayersInRange`), or each opponent, gains the amount
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let amount = cx.arg_int(0)?;
    let opponents = cx.args.get(1).is_some_and(|a| a.trim() == "TargetController.OPPONENT");
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost || (opponents && p == you) {
            continue;
        }
        gain_life(game, p, amount);
    }
    Some(())
}
