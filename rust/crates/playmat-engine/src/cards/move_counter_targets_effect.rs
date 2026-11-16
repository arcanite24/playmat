//! `MoveCounterTargetsEffect(counterType)`: "move a +1/+1 counter from target creature onto a second target
//! creature" (Combine Guildmage, Daghatar the Adamant). Both targets still legal (`EachTargetPointer`), the first with
//! such a counter: one put on the second, then one removed from the first. (The no-type overload picks a kind at
//! random when there's one, and isn't handled.)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MoveCounterTargetsEffect", apply).args(&["counter_type"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let counter = cx.args.first()?.clone();
    let (Some(from), Some(to)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    let on = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield;
    if !on(game, from) || !on(game, to) || game.obj(from).counter(&counter) == 0 {
        return Some(());
    }
    add_counters_by(game, to, &counter, 1, Some(cx.controller()));
    if game.obj(from).counter(&counter) > 0 {
        let ob = game.obj_mut(from);
        ob.add_counters(&counter, -1);
        ob.counters.retain(|(_, k)| *k > 0);
    }
    Some(())
}
