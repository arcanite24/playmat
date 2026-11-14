//! Neutralize the Guards: "Creatures target opponent controls get -1/-1 until end of turn. Surveil 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NeutralizeTheGuards.NeutralizeTheGuardsEffect", apply)];

/// `BoostAllEffect(-1, -1, EndOfTurn)` from a resolving spell: the creatures the opponent controls now
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        boost(game, o, -1, -1, Duration::EndOfTurn);
    }
    Some(())
}
