//! Relic's Roar: "Until end of turn, target artifact or creature becomes a Dinosaur artifact creature with base power
//! and toughness 4/3 in addition to its other types."

use super::*;
use crate::card::{Duration, TokenDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("RelicsRoar.RelicsRoarEffect", apply)];

/// `BecomesCreatureTargetEffect` until end of turn that keeps every prior type (layer 4: artifact, creature,
/// Dinosaur added; 7b: base 4/3), its colors and abilities as they were
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let token = TokenDef {
        types: vec![CardType::Artifact, CardType::Creature],
        subtypes: vec!["Dinosaur".into()],
        power: 4,
        toughness: 3,
        ..serde_json::from_value(serde_json::json!({"name": "", "types": []})).ok()?
    };
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: Duration::EndOfTurn,
        lose_abilities: false,
        still_land: true,
        named: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}
