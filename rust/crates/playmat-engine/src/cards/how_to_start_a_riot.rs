//! How to Start a Riot: "Target creature gains menace until end of turn. Creatures target player controls get +2/+0
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HowToStartARiot.HowToStartARiotEffect", apply)];

/// the second target, a player: `BoostAllEffect(2, 0)` of the creatures they control as it resolves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(1) else { return Some(()) };
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in mine {
        boost(game, o, 2, 0, Duration::EndOfTurn);
    }
    Some(())
}
