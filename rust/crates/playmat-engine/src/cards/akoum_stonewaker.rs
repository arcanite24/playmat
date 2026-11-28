//! Akoum Stonewaker: "Landfall — Whenever a land you control enters, you may pay {2}{R}. If you do, create a 3/1 red
//! Elemental creature token with trample and haste. Exile that token at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AkoumStonewaker.AkoumStonewakerEffect", apply)
    .effects(&["new CreateTokenEffect(new Elemental31TrampleHasteToken())"])];

/// The token, then `exileTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
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
