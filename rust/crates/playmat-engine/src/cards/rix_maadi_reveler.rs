//! Rix Maadi Reveler: "Spectacle {2}{B}{R}. When this creature enters, discard a card, then draw a card. If this
//! creature's spectacle cost was paid, instead discard your hand, then draw three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RixMaadiReveler.RixMaadiRevelerEffect", apply)];

/// `SpectacleCondition` (the permanent was cast for its spectacle cost: kicked): your hand, then three cards; else
/// one, then one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spectacle = game.condition_holds(Some(&Condition::Kicked), cx.controller(), cx.source_id());
    let effects = if spectacle {
        serde_json::json!([
            {"class": "DiscardHandControllerEffect"},
            {"class": "DrawCardSourceControllerEffect", "amount": 3}
        ])
    } else {
        serde_json::json!([
            {"class": "DiscardControllerEffect", "amount": 1},
            {"class": "DrawCardSourceControllerEffect", "amount": 1}
        ])
    };
    let effects: Vec<EffectDef> = serde_json::from_value(effects).ok()?;
    cx.run(game, d, &effects);
    Some(())
}
