//! Whirlwind Denial: "For each spell and ability your opponents control, counter it unless its controller pays {4}."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhirlwindDenial.WhirlwindDenialEffect", apply)];

/// Each opponent from you in turn order (`getPlayersInRange`), for each of their stack objects from the top down:
/// "Pay {4} to prevent … from being countered?" and the payment; the ones not paid for are all countered at the end,
/// in that order (`Stack.counter`: an ability leaves the stack, a spell that can be countered goes to its owner's
/// graveyard)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let cost = ManaCost::parse("{4}").ok()?;
    // stack positions (bottom first) of the objects to counter
    let mut unpaid: Vec<usize> = Vec::new();
    let opponents: Vec<Seat> = (0..n).map(|k| (you + k) % n).filter(|p| *p != you && !game.player(*p).lost).collect();
    for p in opponents {
        for at in (0..game.stack.len()).rev() {
            if game.stack[at].controller != p {
                continue;
            }
            let message = format!("Pay {{4}} to prevent {} from being countered?", log::object(game, game.stack[at].card));
            let paid = choose_use(game, d, p, &message)? && pay_mana(game, d, p, &cost);
            if d.stopped() {
                return None;
            }
            if !paid {
                unpaid.push(at);
            }
        }
    }
    let mut removed: Vec<usize> = Vec::new();
    for at in unpaid {
        let now = at - removed.iter().filter(|r| **r < at).count();
        let Some(item) = game.stack.get(now) else { continue };
        let card = item.card;
        if item.ability.is_some() || item.delayed.is_some() {
            Arc::make_mut(&mut game.stack).remove(now);
            removed.push(at);
        } else if game.obj(card).zone == Zone::Stack && !cant_be_countered(game, card) {
            note_countered(game, card, cx.source_id(), cx.controller());
            Arc::make_mut(&mut game.stack).remove(now);
            removed.push(at);
            game.move_to(card, Zone::Graveyard);
        }
    }
    Some(())
}
