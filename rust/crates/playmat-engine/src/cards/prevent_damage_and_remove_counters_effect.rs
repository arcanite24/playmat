//! `PreventDamageAndRemoveCountersEffect`: "If damage would be dealt to {this} while it has a +1/+1 counter on it,
//! prevent that damage and remove a +1/+1 counter from it" (Undergrowth Champion, Oathsworn Knight; "that many
//! +1/+1 counters": Ugin's Conjurant). Its watcher (`PreventDamageAndRemoveCountersWatcher`) keeps, for each
//! damage batch, the sources that already removed one (`Game::counter_removed_batch`): those remove no other and
//! keep preventing in that batch with no counter left.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageAndRemoveCountersEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["text", "text", "text"])];

/// (thatMany, whileHasCounter, textFromIt)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) || ev.target != Some(cx.source) || game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    !cx.arg_bool(1) || game.obj(cx.source).counter("+1/+1") > 0 || game.counter_removed_batch.contains(&cx.source)
}

/// all of it prevented; then one counter (once a batch), or as many as the damage
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let amount = ev.amount;
    cx.prevent(ev, i32::MAX);
    if amount <= 0 {
        return false;
    }
    let done = game.counter_removed_batch.contains(&cx.source);
    let n = if cx.arg_bool(0) {
        amount
    } else if done {
        0
    } else {
        1
    };
    let o = game.obj_mut(cx.source);
    let n = n.min(o.counter("+1/+1"));
    if n > 0 {
        o.add_counters("+1/+1", -n);
        o.counters.retain(|(_, k)| *k > 0);
    }
    if !done {
        game.counter_removed_batch.push(cx.source);
    }
    false
}
