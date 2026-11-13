//! Nylea's Intervention: "Search your library for up to X land cards, reveal them, put them into your hand, then
//! shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NyleasIntervention.NyleasInterventionEffect", apply)];

/// `SearchLibraryPutInHandEffect(TargetCardInLibrary(0, X, lands), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0);
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "SearchLibraryEffect", "filter": [{"p": "type", "t": "LAND"}], "min": 0, "max": x, "to": "HAND"
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
