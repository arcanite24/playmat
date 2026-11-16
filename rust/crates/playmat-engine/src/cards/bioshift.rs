//! Bioshift: "Move any number of +1/+1 counters from target creature onto another target creature with the same
//! controller."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("Bioshift.MoveCounterFromTargetToTargetEffect", apply),
    Handler::predicate("Bioshift.SameControllerPredicate", same_controller),
];

/// both targets still there with one controller: you choose how many of the first's +1/+1 counters (`getAmount(0,
/// n)`, asked whenever it has any) move onto the second
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(from), Some(to)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    if game.obj(from).controller != game.obj(to).controller {
        return Some(());
    }
    let you = cx.controller();
    let have = game.obj(from).counter("+1/+1");
    if have <= 0 {
        return Some(());
    }
    let n = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: have }) {
            Answer::Int(n) if (0..=have).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if n > 0 {
        let ob = game.obj_mut(from);
        ob.add_counters("+1/+1", -n);
        ob.counters.retain(|(k, c)| k != "+1/+1" || *c > 0);
        add_counters_by(game, to, "+1/+1", n, Some(you));
    }
    Some(())
}

/// controlled by the first target's controller (true before it's chosen, and checked on its own)
fn same_controller(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Obj(x)) => game.obj(*x).controller == game.obj(id).controller,
        _ => true,
    }
}
