//! Pestilent Haze: "Choose one — • All creatures get -2/-2 until end of turn. • Remove two loyalty counters from
//! each planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PestilentHaze.PestilentHazeEffect", apply)];

/// `removeCounters(LOYALTY 2)` on each planeswalker, in battlefield order: as many as it has, up to two
fn apply(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    let walkers: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Planeswalker)).collect();
    for o in walkers {
        let n = game.obj(o).counter("loyalty").clamp(0, 2);
        if n > 0 {
            game.obj_mut(o).add_counters("loyalty", -n);
        }
    }
    Some(())
}
