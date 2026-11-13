//! The Antiquities War's chapter III: "Artifacts you control become artifact creatures with base power and toughness
//! 5/5 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheAntiquitiesWar.TheAntiquitiesWarEffect", apply)];

/// The artifacts you control as it resolves (its affected objects) become 5/5 artifact creatures until end of turn,
/// keeping their other types, colors and abilities (`BecomesCreatureTargetEffect` of an artifact creature)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let artifacts: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Artifact))
        .map(Target::Obj)
        .collect();
    if artifacts.is_empty() {
        return Some(());
    }
    let token = crate::card::TokenDef {
        types: vec![CardType::Artifact, CardType::Creature],
        power: 5,
        toughness: 5,
        ..Default::default()
    };
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: Duration::EndOfTurn,
        lose_abilities: false,
        still_land: false,
        named: None,
    };
    cx.run_on(game, d, &[e], artifacts);
    Some(())
}
