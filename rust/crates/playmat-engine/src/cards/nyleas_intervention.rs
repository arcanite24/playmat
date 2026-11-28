//! Nylea's Intervention: "Search your library for up to X land cards, reveal them, put them into your hand, then
//! shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NyleasIntervention.NyleasInterventionEffect", apply)];

/// `SearchLibraryPutInHandEffect(TargetCardInLibrary(0, X, lands), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0);
    // X of 0: nothing can be chosen and no one is asked, but the library is still shuffled
    if x == 0 {
        let you = cx.controller();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
        return Some(());
    }
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "SearchLibraryEffect", "filter": [{"p": "type", "t": "LAND"}], "min": 0, "max": x, "to": "HAND"
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
