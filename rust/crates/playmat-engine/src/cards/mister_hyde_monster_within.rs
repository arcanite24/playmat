//! Mister Hyde, Monster Within: "At the beginning of your upkeep, choose one — • Put a +1/+1 counter on Mister Hyde.
//! • Remove a counter from a creature you control. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MisterHydeMonsterWithin.MisterHydeMonsterWithinRemoveCounterEffect", apply)];

/// `RemoveCounterCost(TargetControlledCreaturePermanent)` paid by the effect: a creature you control is chosen
/// (`TargetImpl.choose`, counters or not); with counters on it, one kind (a choice in `Counters`' hash order when
/// there are several) loses one, and you draw a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let can = controlled_creatures(game, you);
    if can.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_in_rounds(game, d, you, &can, 1, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    let kinds = java_hash_order(game.obj(o).counters.iter().filter(|(_, n)| *n > 0).map(|(c, _)| c.clone()).collect());
    let kind = match kinds.len() {
        0 => return Some(()),
        1 => kinds[0].clone(),
        _ => {
            let message = format!("Choose a counter to remove from {}", log::object(game, o));
            let q = Query::Choice { message, choices: kinds.clone() };
            loop {
                match ask(game, d, you, &q) {
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
    draw(game, you, 1);
    Some(())
}
