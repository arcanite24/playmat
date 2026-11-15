//! Polymorphist's Jest: "Until end of turn, each creature target player controls loses all abilities and becomes a
//! blue Frog with base power and toughness 1/1."

use super::*;
use crate::card::Duration;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PolymorphistsJest.PolymorphistsJestEffect", apply).tokens(&["FrogToken"])];

/// The creatures the target player controls as it resolves (`init`'s `affectedObjectList`), each losing all
/// abilities and becoming a blue 1/1 Frog until end of turn, as Turn to Frog's effect does: creature types replaced
/// by Frog, its other types kept
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hit: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .map(Target::Obj)
        .collect();
    if hit.is_empty() {
        return Some(());
    }
    let e = EffectDef::BecomesCreatureTargetEffect {
        token: cx.tokens.first()?.clone(),
        duration: Duration::EndOfTurn,
        lose_abilities: true,
        still_land: false,
        named: None,
    };
    cx.run_on(game, d, &[e], hit);
    Some(())
}
