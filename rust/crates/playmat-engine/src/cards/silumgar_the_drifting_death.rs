//! Silumgar, the Drifting Death: "Whenever a Dragon you control attacks, creatures defending player controls get
//! -1/-1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SilumgarTheDriftingDeath.UnboostCreaturesTargetPlayerEffect", apply).args(&["text", "text"])];

/// The creatures the target player (the defending player) controls as the effect starts (its affected objects)
/// get the power and toughness given until end of turn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let (power, toughness) = (cx.arg_int(0).unwrap_or(0), cx.arg_int(1).unwrap_or(0));
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for c in creatures {
        boost(game, c, power, toughness, Duration::EndOfTurn);
    }
    Some(())
}
