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
    let before = game.objs.len();
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let made: Vec<(ObjId, u32)> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(|o| (o, game.obj(o).zcc))
        .collect();
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, exile those tokens.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: made,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
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
