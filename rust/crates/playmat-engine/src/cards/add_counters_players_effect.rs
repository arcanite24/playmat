//! `AddCountersPlayersEffect(counter, TargetController)`: "each opponent gets a poison counter" (Infectious Bite,
//! Vraska's Fall, Prologue to Phyresis): each player of the group, in range order from you (`getOpponents`,
//! `getPlayersInRange`), gets the counters (`Player.addCounters`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddCountersPlayersEffect", apply)
    .args(&["counter", "one_of:TargetController.OPPONENT|TargetController.EACH_PLAYER|TargetController.ANY|TargetController.YOU"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut counter = cx.args.first()?.split(' ');
    let name = counter.next()?.to_string();
    let n: i32 = counter.next()?.parse().ok()?;
    let who = cx.args.get(1)?.trim();
    if n <= 0 {
        return Some(());
    }
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost || (who == "TargetController.OPPONENT" && p == you) || (who == "TargetController.YOU" && p != you) {
            continue;
        }
        if name == "poison" {
            add_poison(game, p, n);
        } else {
            let counters = &mut game.player_mut(p).counters;
            match counters.iter_mut().find(|(c, _)| *c == name) {
                Some((_, k)) => *k += n,
                None => counters.push((name.clone(), n)),
            }
        }
    }
    Some(())
}
