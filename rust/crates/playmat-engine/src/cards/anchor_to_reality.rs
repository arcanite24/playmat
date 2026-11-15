//! Anchor to Reality: "As an additional cost to cast this spell, sacrifice an artifact or creature. Search your
//! library for an Equipment or Vehicle card, put that card onto the battlefield, then shuffle. If it has mana value
//! less than the sacrificed permanent's mana value, scry 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AnchorToReality.AnchorToRealityEffect", apply)];

/// the search, the card onto the battlefield, the shuffle; then scry 2 if its mana value is less than the
/// sacrificed permanent's
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "or", "xs": [{"p": "subtype", "s": "Equipment"}, {"p": "subtype", "s": "Vehicle"}]}
    ]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    let shuffle = |game: &mut Game| {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    };
    let Some(&card) = found.first() else {
        shuffle(game);
        return Some(());
    };
    let searched = game.obj(card).def.mana_value();
    put_onto_battlefield(game, d, card, you);
    if d.stopped() {
        return None;
    }
    shuffle(game);
    let sacrificed = cx.item.sacrificed.first().map_or(0, |s| game.obj(*s).def.mana_value());
    if searched < sacrificed {
        let rule = item_rule(game, cx.item);
        scry(game, d, you, 2, &rule).then_some(())?;
    }
    Some(())
}
