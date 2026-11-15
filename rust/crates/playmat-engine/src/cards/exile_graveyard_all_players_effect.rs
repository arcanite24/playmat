//! `ExileGraveyardAllPlayersEffect(filter, TargetController.OPPONENT)`: "exile each opponent's graveyard" (Dauntless
//! Scrapbot, Ashiok, Dream Render). The form without arguments is the engine's own effect.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExileGraveyardAllPlayersEffect", apply)
    .args(&["filter", "one_of:TargetController.OPPONENT"])];

/// the cards of the filter in each opponent's graveyard, moved to exile together (graveyard order)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let mut cards = Vec::new();
    for p in 0..game.seats() {
        if p == you {
            continue;
        }
        for c in game.player(p).graveyard.clone() {
            if game.filter_matches(&filter, you, Some(cx.source_id()), c) {
                cards.push(c);
            }
        }
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
