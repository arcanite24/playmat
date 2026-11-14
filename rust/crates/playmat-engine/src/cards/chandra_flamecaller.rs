//! Chandra, Flamecaller: "+1: Create two 3/1 red Elemental creature tokens with haste. Exile them at the beginning of
//! the next end step. 0: Discard all the cards in your hand, then draw that many cards plus one. −X: Chandra,
//! Flamecaller deals X damage to each creature."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ChandraFlamecaller.ChandraElementalEffect", elementals).tokens(&["ElementalHasteToken"]),
    Handler::effect("ChandraFlamecaller.ChandraDrawEffect", wheel),
];

/// Two Elementals (`CreateTokenEffect`), exiled at the beginning of the next end step
/// (`exileTokensCreatedAtNextEndStep`)
fn elementals(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    remove_tokens_at_next_end_step(game, &made, cx.source_id(), you, true);
    Some(())
}

/// The whole hand discarded (`discard(hand)`), then that many cards plus one drawn
fn wheel(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    for c in &hand {
        discard_card(game, you, *c);
    }
    draw(game, you, hand.len() + 1);
    Some(())
}
