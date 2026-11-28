//! The Stone Brain: "{2}, {T}, Exile The Stone Brain: Choose a card name. Search target opponent's graveyard, hand,
//! and library for up to four cards with that name and exile them. That player shuffles, then draws a card for each
//! card exiled from their hand this way. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheStoneBrain.TheStoneBrainEffect", apply)];

/// Unmoored Ego's `applySearchAndExile` (graveyard optional, up to four, a draw for each card from the hand) with
/// the name `ChooseACardNameEffect` kept on the source
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(name) = game.obj(cx.source_id()).chosen_name.clone() else { return Some(()) };
    let Some(p) = cx.player(0) else { return Some(()) };
    super::search_for_card_name_and_exile::search_and_exile_named(game, d, cx.controller(), p, &name, 4, true)
}
