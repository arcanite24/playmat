//! Jace, Memory Adept: "−7: Any number of target players each draw twenty cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JaceMemoryAdept.JaceMemoryAdeptEffect", apply).args(&["text"])];

/// each target player still legal, in the order chosen, draws the amount (`JaceMemoryAdeptEffect(20)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0)?.max(0) as usize;
    let players: Vec<Seat> = cx
        .legal
        .first()
        .map(|ts| ts.iter().filter_map(|t| if let Target::Player(p) = t { Some(*p) } else { None }).collect())
        .unwrap_or_default();
    for p in players {
        draw(game, p, n);
    }
    Some(())
}
