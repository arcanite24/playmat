//! Vorel of the Hull Clade: "{G}{U}, {T}: For each counter on target artifact, creature, or land, put another of those
//! counters on that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VorelOfTheHullClade.VorelOfTheHullCladeEffect", apply)];

/// Each kind of counter on it, doubled by you
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let counters: Vec<(String, i32)> = game.obj(o).counters.iter().filter(|(_, n)| *n > 0).cloned().collect();
    for (name, n) in counters {
        add_counters_by(game, o, &name, n, Some(you));
    }
    Some(())
}
