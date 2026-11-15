//! Thornmantle Striker: "When Thornmantle Striker enters the battlefield, choose one — • Remove X counters from
//! target permanent, where X is the number of Elves you control. • Target creature an opponent controls gets -X/-X
//! until end of turn, where X is the number of Elves you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ThornmantleStriker.ThornmantleStrikerEffect", apply).args(&["text"])];

/// X Elves you control; with that many counters or fewer on the target, all of them go; with one kind, X of it;
/// otherwise kind by kind in Java's `Counters` (a `HashMap`) order, "<kind> counters to remove" (between what the
/// rest can't cover and what's left), until X are gone
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0) else { return Some(()) };
    let elves =
        game.battlefield.iter().filter(|p| game.obj(**p).controller == you && game.has_subtype(**p, "Elf")).count()
            as i32;
    if elves < 1 {
        return Some(());
    }
    let counters: Vec<(String, i32)> = game.obj(o).counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
    let total: i32 = counters.iter().map(|(_, n)| *n).sum();
    if total == 0 {
        return Some(());
    }
    let remove = |game: &mut Game, name: &str, n: i32| {
        let ob = game.obj_mut(o);
        let n = n.min(ob.counter(name));
        if n > 0 {
            ob.add_counters(name, -n);
            ob.counters.retain(|(_, k)| *k > 0);
        }
    };
    let kinds = java_hash_order(counters.iter().map(|(k, _)| k.clone()).collect());
    let count = |k: &str| counters.iter().find(|(c, _)| c == k).map_or(0, |(_, n)| *n);
    if total <= elves {
        for k in &kinds {
            remove(game, k, count(k));
        }
        return Some(());
    }
    if kinds.len() == 1 {
        remove(game, &kinds[0], elves);
        return Some(());
    }
    let mut remaining = total;
    let mut left = elves;
    for k in &kinds {
        let n = count(k);
        remaining -= n;
        let min = (left - remaining).max(0);
        let max = left.min(n);
        let take = if min >= max {
            min
        } else {
            loop {
                match ask(game, d, you, &Query::Amount { min, max }) {
                    Answer::Int(v) => break v.clamp(min, max),
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            }
        };
        remove(game, k, take);
        left -= take;
        if left <= 0 {
            break;
        }
    }
    Some(())
}
