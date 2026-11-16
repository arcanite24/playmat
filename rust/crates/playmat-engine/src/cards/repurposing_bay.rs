//! Repurposing Bay: "{2}, {T}, Sacrifice another artifact: Search your library for an artifact card with mana value
//! equal to 1 plus the sacrificed artifact's mana value, put that card onto the battlefield, then shuffle. Activate
//! only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RepurposingBay.RepurposingBayEffect", apply)];

/// an artifact card of mana value one more than the sacrificed one's, searched for and put onto the battlefield;
/// then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let sacrificed = *cx.item.sacrificed.first()?;
    let n = game.obj(sacrificed).def.mana_value() + 1;
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "type", "t": "ARTIFACT"},
        {"p": "mv", "cmp": "EQUAL_TO", "n": n}
    ]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        put_onto_battlefield(game, d, card, you);
        if d.stopped() {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
