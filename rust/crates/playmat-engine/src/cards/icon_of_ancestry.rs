//! Icon of Ancestry: "{3}, {T}: Look at the top three cards of your library. You may reveal a creature card of the
//! chosen type from among them and put it into your hand. Put the rest on the bottom of your library in a random
//! order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IconOfAncestry.IconOfAncestryEffect", apply)
    .effects(&["new LookLibraryAndPickControllerEffect(3, 1, filter, PutCards.HAND, PutCards.BOTTOM_RANDOM)"])];

/// Nothing without a chosen creature type; else `LookLibraryAndPickControllerEffect(3, 1, <creature card of that
/// type>, HAND, BOTTOM_RANDOM)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.obj(cx.source_id()).chosen_type.is_none() {
        return Some(());
    }
    let Some(mut e) = cx.effects.first().cloned() else { return Some(()) };
    if let EffectDef::LookLibraryAndPickControllerEffect { filter, .. } = &mut e {
        *filter = serde_json::from_value(serde_json::json!([
            {"p": "type", "t": "CREATURE"},
            {"p": "chosen_subtype", "v": true}
        ]))
        .expect("a filter");
    }
    cx.run(game, d, &[e]);
    Some(())
}
