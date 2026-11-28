//! `RemoveCounterTargetEffect()` with no counter named (Thrull Parasite: "Remove a counter from target nonland
//! permanent."): one counter of a kind the controller chooses.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RemoveCounterTargetEffect", apply)];

/// `selectCounterType`: one kind on it, that one; more, the controller picks one (`ChoiceImpl`, the kinds in
/// `Counters`' hash order); a cancelled choice removes nothing
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let kinds = java_hash_order(game.obj(o).counters.iter().filter(|(_, n)| *n > 0).map(|(c, _)| c.clone()).collect());
    let kind = match kinds.len() {
        0 => return Some(()),
        1 => kinds[0].clone(),
        _ => {
            let name = game.obj(o).def.name.clone();
            let q = Query::Choice { message: format!("Choose a counter type to remove from {name}"), choices: kinds.clone() };
            loop {
                match ask(game, d, cx.controller(), &q) {
                    Answer::Text(t) if kinds.contains(&t) => break t,
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            }
        }
    };
    let ob = game.obj_mut(o);
    ob.add_counters(&kind, -1);
    ob.counters.retain(|(_, k)| *k > 0);
    Some(())
}
