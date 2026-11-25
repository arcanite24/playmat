//! Guul Draz Overseer: "Flying. Landfall — Whenever a land you control enters, other creatures you control get +1/+0
//! until end of turn. If that land is a Swamp, those creatures get +2/+0 until end of turn instead."

use super::*;
use crate::card::{Amount, Duration, Filter};

pub const HANDLERS: &[Handler] = &[Handler::effect("GuulDrazOverseer.GuulDrazOverseerEffect", apply)];

/// the land that entered, or as it last was (`getFirstTargetPermanentOrLKI`): a Swamp makes it +2/+0;
/// `BoostControlledEffect(boost, 0, EndOfTurn, true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(land)) = cx.item.targets.first().and_then(|ts| ts.first()).copied() else {
        return Some(());
    };
    let boost = if game.has_subtype(land, "Swamp") { 2 } else { 1 };
    let filter: Filter = serde_json::from_value(serde_json::json!([{"p": "type", "t": "CREATURE"}])).ok()?;
    let e = EffectDef::BoostControlledEffect {
        power: Amount::Fixed(boost),
        toughness: Amount::Fixed(0),
        duration: Duration::EndOfTurn,
        filter,
        exclude_source: true,
    };
    cx.run(game, d, &[e]);
    Some(())
}
