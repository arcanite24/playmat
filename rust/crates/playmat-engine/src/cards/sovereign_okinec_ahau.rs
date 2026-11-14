//! Sovereign Okinec Ahau: "Whenever Sovereign Okinec Ahau attacks, for each creature you control with power greater
//! than that creature's base power, put a number of +1/+1 counters on that creature equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SovereignOkinecAhau.SovereignOkinecAhauEffect", apply)];

/// Power against `getModifiedBaseValue` for each creature you control, read as it gets its counters
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        let diff = game.power_toughness(o).0 - game.base_power_toughness(o).0;
        if diff > 0 {
            add_counters_by(game, o, "+1/+1", diff, Some(you));
        }
    }
    Some(())
}
