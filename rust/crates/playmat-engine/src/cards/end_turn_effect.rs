//! `EndTurnEffect`: "End the turn." (Time Stop, Day's Undoing, Ultima, Discontinuity, Glorious End, Hurkyl's Final
//! Meditation). `Turn.endTurn`: every spell on the stack is exiled (the resolving one first, as Java's is still on
//! the stack), every ability on it removed, attacking and blocking creatures leave combat, the waiting triggered
//! abilities are dropped and the state is checked with no trigger put on the stack; then the turn goes straight
//! to its cleanup step (`Game::end_turn`, cleared as the cleanup step starts).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EndTurnEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    game.end_turn = true;
    // the resolving spell (a resolving ability is just gone once it's done)
    if let Some((_, id)) = game.resolving {
        if game.resolving_item.as_ref().is_some_and(|i| i.ability.is_none() && i.delayed.is_none())
            && game.obj(id).zone == Zone::Stack
        {
            game.move_to(id, Zone::Exile);
        }
    }
    while let Some(item) = Arc::make_mut(&mut game.stack).pop() {
        if item.ability.is_none() && item.delayed.is_none() && game.obj(item.card).zone == Zone::Stack {
            game.move_to(item.card, Zone::Exile);
        }
    }
    let fighting: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.attacking.is_some() || !o.blocking.is_empty()
        })
        .collect();
    for c in fighting {
        remove_from_combat(game, c);
    }
    game.pending.clear();
    game.batch_pending.clear();
    check_state_and_triggered(game, d);
    Some(())
}
