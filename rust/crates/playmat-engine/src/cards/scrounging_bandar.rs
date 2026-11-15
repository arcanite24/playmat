//! Scrounging Bandar: "Scrounging Bandar enters the battlefield with two +1/+1 counters on it. At the beginning of
//! your upkeep, you may move any number of +1/+1 counters from Scrounging Bandar onto another target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScroungingBandar.ScroungingBandarEffect", apply)];

/// Both permanents: "How many counters do you want to move?" (`getAmount(0, counters)`, not asked with none), then
/// that many removed from it and put on the target by you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(src).zone != Zone::Battlefield || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    let max = game.obj(src).counter("+1/+1");
    if max <= 0 {
        return Some(());
    }
    let n = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max }) {
            Answer::Int(n) if (0..=max).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if n > 0 {
        let o = game.obj_mut(src);
        o.add_counters("+1/+1", -n);
        o.counters.retain(|(_, k)| *k > 0);
        add_counters_by(game, to, "+1/+1", n, Some(you));
    }
    Some(())
}
